//! Full-text-search (FTS5) index maintenance and querying.

use crate::db::Database;
use crate::search::{FtsSearchResult, SearchField};
use anyhow::Result;
use rusqlite::params;

impl Database {
    pub fn rebuild_fts_index(&self) -> Result<u64> {
        self.conn.execute("DELETE FROM books_fts", [])?;

        let count = self.conn.execute(
            r#"
            INSERT INTO books_fts(rowid, title, author, content)
            SELECT b.id, b.title, b.author,
                   COALESCE(GROUP_CONCAT(s.content, ' '), '')
            FROM books b
            LEFT JOIN sections s ON b.id = s.book_id
            GROUP BY b.id
            "#,
            [],
        )?;

        Ok(count as u64)
    }

    pub fn update_fts_index(&self, book_id: i64) -> Result<()> {
        self.conn
            .execute("DELETE FROM books_fts WHERE rowid = ?1", params![book_id])?;

        self.conn.execute(
            r#"
            INSERT INTO books_fts(rowid, title, author, content)
            SELECT b.id, b.title, b.author,
                   COALESCE(GROUP_CONCAT(s.content, ' '), '')
            FROM books b
            LEFT JOIN sections s ON b.id = s.book_id
            WHERE b.id = ?1
            GROUP BY b.id
            "#,
            params![book_id],
        )?;

        Ok(())
    }

    pub fn remove_fts_index(&self, book_id: i64) -> Result<()> {
        self.conn
            .execute("DELETE FROM books_fts WHERE rowid = ?1", params![book_id])?;
        Ok(())
    }

    pub fn fts_index_count(&self) -> Result<i64> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM books_fts", [], |row| row.get(0))?;
        Ok(count)
    }

    pub fn search_fts(
        &self,
        keyword: &str,
        exact: bool,
        search_field: SearchField,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<FtsSearchResult>> {
        let match_clause = crate::search::build_fts_match_expr(keyword, exact, search_field);

        let sql = format!(
            r#"
            SELECT
                fts.rowid,
                fts.title,
                fts.author,
                b.category,
                b.word_count,
                b.created_at,
                bm25(books_fts) as rank,
                snippet(books_fts, 0, '>>>', '<<<', '...', 32) as title_snippet,
                snippet(books_fts, 1, '>>>', '<<<', '...', 32) as author_snippet,
                snippet(books_fts, 2, '>>>', '<<<', '...', 32) as content_snippet
            FROM books_fts as fts
            JOIN books b ON fts.rowid = b.id
            WHERE {}
            ORDER BY rank DESC
            LIMIT ?1 OFFSET ?2
            "#,
            match_clause
        );

        let mut stmt = self.conn.prepare(&sql)?;

        let raw_results: Vec<(
            i64,
            String,
            String,
            String,
            i64,
            i64,
            f64,
            String,
            String,
            String,
        )> = stmt
            .query_map(params![limit, offset], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, String>(8)?,
                    row.get::<_, String>(9)?,
                ))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        if raw_results.is_empty() {
            return Ok(Vec::new());
        }

        let min_rank = raw_results
            .iter()
            .map(|r| r.6)
            .fold(f64::INFINITY, f64::min);
        let max_rank = raw_results
            .iter()
            .map(|r| r.6)
            .fold(f64::NEG_INFINITY, f64::max);

        let results = raw_results
            .into_iter()
            .map(
                |(
                    book_id,
                    title,
                    author,
                    category,
                    word_count,
                    created_at,
                    rank,
                    title_snippet,
                    author_snippet,
                    content_snippet,
                )| {
                    let relevance_score =
                        crate::search::normalize_bm25_score(rank, min_rank, max_rank);

                    let title_matches = crate::search::count_matches(&title_snippet, ">>>", "<<<");
                    let author_matches =
                        crate::search::count_matches(&author_snippet, ">>>", "<<<");
                    let content_matches =
                        crate::search::count_matches(&content_snippet, ">>>", "<<<");

                    let snippet =
                        build_display_snippet(&title_snippet, &author_snippet, &content_snippet);

                    FtsSearchResult {
                        book_id,
                        title: crate::search::strip_highlight_markers(&title, ">>>", "<<<"),
                        author: crate::search::strip_highlight_markers(&author, ">>>", "<<<"),
                        category,
                        word_count,
                        created_at,
                        relevance_score,
                        title_matches,
                        author_matches,
                        content_matches,
                        snippet: crate::search::highlight_snippet(&snippet, ">>>", "<<<"),
                    }
                },
            )
            .collect();

        Ok(results)
    }

    pub fn search_fts_count(&self, keyword: &str, exact: bool) -> Result<i64> {
        let match_clause = crate::search::build_fts_match_expr(keyword, exact, SearchField::All);

        let sql = format!(
            "SELECT COUNT(*) FROM books_fts as fts WHERE {}",
            match_clause
        );

        let count: i64 = self.conn.query_row(&sql, [], |row| row.get(0))?;
        Ok(count)
    }
}

fn build_display_snippet(
    title_snippet: &str,
    author_snippet: &str,
    content_snippet: &str,
) -> String {
    let mut parts = Vec::new();

    let title_clean = crate::search::strip_highlight_markers(title_snippet, ">>>", "<<<");
    let _author_clean = crate::search::strip_highlight_markers(author_snippet, ">>>", "<<<");
    let content_clean = crate::search::strip_highlight_markers(content_snippet, ">>>", "<<<");

    if title_snippet.contains(">>>") {
        parts.push(format!("[标题] {}", title_snippet));
    }
    if author_snippet.contains(">>>") {
        parts.push(format!("[作者] {}", author_snippet));
    }
    if content_snippet.contains(">>>") && !content_clean.trim().is_empty() {
        parts.push(format!("[内容] {}", content_snippet));
    }

    if parts.is_empty() {
        if !content_clean.trim().is_empty() {
            format!("[内容] {}", content_snippet)
        } else if !title_clean.trim().is_empty() {
            format!("[标题] {}", title_snippet)
        } else {
            format!("[作者] {}", author_snippet)
        }
    } else {
        parts.join("\n")
    }
}
