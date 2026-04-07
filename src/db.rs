use crate::search::{FtsSearchResult, SearchField};
use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension};

const DB_NAME: &str = "banzhu.db";

const CREATE_BOOKS_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS books (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    num INTEGER NOT NULL DEFAULT 0,
    title TEXT NOT NULL UNIQUE,
    filename TEXT NOT NULL,
    author TEXT NOT NULL DEFAULT '',
    category TEXT NOT NULL DEFAULT '',
    introduce TEXT NOT NULL DEFAULT '',
    likes INTEGER NOT NULL DEFAULT 0,
    word_count INTEGER NOT NULL DEFAULT 0,
    page_count INTEGER NOT NULL DEFAULT 0,
    download_time TEXT NOT NULL DEFAULT ''
)"#;

const CREATE_CHAPTERS_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS chapters (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    book_id INTEGER NOT NULL,
    title TEXT NOT NULL,
    url TEXT NOT NULL DEFAULT '',
    chapter_order INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (book_id) REFERENCES books(id) ON DELETE CASCADE,
    UNIQUE(book_id, title)
)"#;

const CREATE_SECTIONS_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS sections (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    chapter_id INTEGER NOT NULL,
    book_id INTEGER NOT NULL,
    url TEXT NOT NULL DEFAULT '',
    content TEXT NOT NULL DEFAULT '',
    section_order INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (chapter_id) REFERENCES chapters(id) ON DELETE CASCADE,
    FOREIGN KEY (book_id) REFERENCES books(id) ON DELETE CASCADE
)"#;

const CREATE_INDEX: &str = r#"
CREATE INDEX IF NOT EXISTS idx_chapters_book_id ON chapters(book_id);
CREATE INDEX IF NOT EXISTS idx_sections_book_id ON sections(book_id);
CREATE INDEX IF NOT EXISTS idx_sections_chapter_id ON sections(chapter_id);
CREATE INDEX IF NOT EXISTS idx_books_title ON books(title);
"#;

const CREATE_FTS_TABLE: &str = r#"
CREATE VIRTUAL TABLE IF NOT EXISTS books_fts USING fts5(
    title,
    author,
    content,
    tokenize='simple disable_pinyin'
);
"#;

#[derive(Debug, Clone)]
pub struct BookRecord {
    pub id: i64,
    pub num: i64,
    pub title: String,
    pub filename: String,
    pub author: String,
    pub category: String,
    pub introduce: String,
    pub likes: i64,
    pub word_count: i64,
    pub page_count: i64,
    pub download_time: String,
}

#[derive(Debug, Clone)]
pub struct ChapterRecord {
    pub id: i64,
    pub book_id: i64,
    pub title: String,
    pub url: String,
    pub chapter_order: i64,
}

#[derive(Debug, Clone)]
pub struct SectionRecord {
    pub id: i64,
    pub chapter_id: i64,
    pub book_id: i64,
    pub url: String,
    pub content: String,
    pub section_order: i64,
}

#[derive(Debug, Clone)]
pub struct SearchRecord {
    pub id: i64,
    pub title: String,
    pub author: String,
    pub category: String,
    pub word_count: i64,
    pub chapter_count: i64,
    pub download_time: String,
}

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn new() -> Result<Self> {
        Self::open(DB_NAME)
    }

    pub fn open(path: &str) -> Result<Self> {
        let conn = Connection::open(path)?;
        let db = Database { conn };
        db.init_tables()?;
        db.init_fts()?;
        Ok(db)
    }

    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let db = Database { conn };
        db.init_tables()?;
        db.init_fts()?;
        Ok(db)
    }

    fn init_tables(&self) -> Result<()> {
        self.conn.execute_batch("PRAGMA journal_mode=WAL;")?;
        self.conn.execute_batch("PRAGMA foreign_keys=ON;")?;
        self.conn.execute(CREATE_BOOKS_TABLE, [])?;
        self.conn.execute(CREATE_CHAPTERS_TABLE, [])?;
        self.conn.execute(CREATE_SECTIONS_TABLE, [])?;
        self.conn.execute_batch(CREATE_INDEX)?;
        Ok(())
    }

    fn init_fts(&self) -> Result<()> {
        if let Err(e) = sqlite_simple_tokenizer::load(&self.conn) {
            eprintln!(
                "Warning: Failed to load simple tokenizer: {}, using simple fallback",
                e
            );
            self.conn.execute(
                "CREATE VIRTUAL TABLE IF NOT EXISTS books_fts USING fts5(title, author, content, tokenize='simple disable_pinyin');",
                [],
            )?;
            return Ok(());
        }
        self.conn.execute(CREATE_FTS_TABLE, [])?;
        Ok(())
    }

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
                b.download_time,
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
            String,
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
                    download_time,
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
                        download_time,
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

    pub fn insert_book(&self, book: &BookRecord) -> Result<i64> {
        self.conn.execute(
            "INSERT OR REPLACE INTO books (num, title, filename, author, category, introduce, likes, word_count, page_count, download_time)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                book.num,
                book.title,
                book.filename,
                book.author,
                book.category,
                book.introduce,
                book.likes,
                book.word_count,
                book.page_count,
                book.download_time,
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn insert_chapter(&self, chapter: &ChapterRecord) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO chapters (book_id, title, url, chapter_order)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                chapter.book_id,
                chapter.title,
                chapter.url,
                chapter.chapter_order,
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn insert_section(&self, section: &SectionRecord) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO sections (chapter_id, book_id, url, content, section_order)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                section.chapter_id,
                section.book_id,
                section.url,
                section.content,
                section.section_order,
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn get_book(&self, book_id: i64) -> Result<Option<BookRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, num, title, filename, author, category, introduce, likes, word_count, page_count, download_time
             FROM books WHERE id = ?1",
        )?;

        let result = stmt
            .query_row(params![book_id], |row| {
                Ok(BookRecord {
                    id: row.get(0)?,
                    num: row.get(1)?,
                    title: row.get(2)?,
                    filename: row.get(3)?,
                    author: row.get(4)?,
                    category: row.get(5)?,
                    introduce: row.get(6)?,
                    likes: row.get(7)?,
                    word_count: row.get(8)?,
                    page_count: row.get(9)?,
                    download_time: row.get(10)?,
                })
            })
            .optional()?;

        Ok(result)
    }

    pub fn book_exists(&self, book_id: i64) -> Result<bool> {
        let result: Option<i64> = self
            .conn
            .query_row(
                "SELECT id FROM books WHERE id = ?1",
                params![book_id],
                |row| row.get(0),
            )
            .optional()?;
        Ok(result.is_some())
    }

    pub fn book_exists_by_title(&self, title: &str) -> Result<bool> {
        let result: Option<String> = self
            .conn
            .query_row(
                "SELECT title FROM books WHERE title = ?1",
                params![title],
                |row| row.get(0),
            )
            .optional()?;
        Ok(result.is_some())
    }

    pub fn get_book_by_title(&self, title: &str) -> Result<Option<BookRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, num, title, filename, author, category, introduce, likes, word_count, page_count, download_time
             FROM books WHERE title = ?1",
        )?;

        let result = stmt
            .query_row(params![title], |row| {
                Ok(BookRecord {
                    id: row.get(0)?,
                    num: row.get(1)?,
                    title: row.get(2)?,
                    filename: row.get(3)?,
                    author: row.get(4)?,
                    category: row.get(5)?,
                    introduce: row.get(6)?,
                    likes: row.get(7)?,
                    word_count: row.get(8)?,
                    page_count: row.get(9)?,
                    download_time: row.get(10)?,
                })
            })
            .optional()?;

        Ok(result)
    }

    pub fn get_chapters_by_book(&self, book_id: i64) -> Result<Vec<ChapterRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, book_id, title, url, chapter_order
             FROM chapters WHERE book_id = ?1 ORDER BY chapter_order ASC",
        )?;

        let chapters = stmt
            .query_map(params![book_id], |row| {
                Ok(ChapterRecord {
                    id: row.get(0)?,
                    book_id: row.get(1)?,
                    title: row.get(2)?,
                    url: row.get(3)?,
                    chapter_order: row.get(4)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(chapters)
    }

    pub fn get_sections_by_chapter(&self, chapter_id: i64) -> Result<Vec<SectionRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, chapter_id, book_id, url, content, section_order
             FROM sections WHERE chapter_id = ?1 ORDER BY section_order ASC",
        )?;

        let sections = stmt
            .query_map(params![chapter_id], |row| {
                Ok(SectionRecord {
                    id: row.get(0)?,
                    chapter_id: row.get(1)?,
                    book_id: row.get(2)?,
                    url: row.get(3)?,
                    content: row.get(4)?,
                    section_order: row.get(5)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(sections)
    }

    pub fn get_sections_by_book(&self, book_id: i64) -> Result<Vec<SectionRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, chapter_id, book_id, url, content, section_order
             FROM sections WHERE book_id = ?1 ORDER BY chapter_id ASC, section_order ASC",
        )?;

        let sections = stmt
            .query_map(params![book_id], |row| {
                Ok(SectionRecord {
                    id: row.get(0)?,
                    chapter_id: row.get(1)?,
                    book_id: row.get(2)?,
                    url: row.get(3)?,
                    content: row.get(4)?,
                    section_order: row.get(5)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(sections)
    }

    pub fn search_books(&self, keyword: &str) -> Result<Vec<SearchRecord>> {
        let pattern = format!("%{}%", keyword);

        let mut stmt = self.conn.prepare(
            r#"
            SELECT b.id, b.title, b.author, b.category, b.word_count,
                   COUNT(DISTINCT c.id) as chapter_count,
                   b.download_time
            FROM books b
            LEFT JOIN chapters c ON b.id = c.book_id
            WHERE b.title LIKE ?1 OR b.author LIKE ?1
            GROUP BY b.id
            ORDER BY b.id ASC
            "#,
        )?;

        let results = stmt
            .query_map(params![pattern], |row| {
                Ok(SearchRecord {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    author: row.get(2)?,
                    category: row.get(3)?,
                    word_count: row.get(4)?,
                    chapter_count: row.get(5)?,
                    download_time: row.get(6)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(results)
    }

    pub fn get_chapter_by_book_and_order(
        &self,
        book_id: i64,
        chapter_order: i64,
    ) -> Result<Option<ChapterRecord>> {
        let result = self
            .conn
            .query_row(
                "SELECT id, book_id, title, url, chapter_order
                 FROM chapters WHERE book_id = ?1 AND chapter_order = ?2",
                params![book_id, chapter_order],
                |row| {
                    Ok(ChapterRecord {
                        id: row.get(0)?,
                        book_id: row.get(1)?,
                        title: row.get(2)?,
                        url: row.get(3)?,
                        chapter_order: row.get(4)?,
                    })
                },
            )
            .optional()?;

        Ok(result)
    }

    pub fn delete_book(&self, book_id: i64) -> Result<()> {
        self.conn
            .execute("DELETE FROM sections WHERE book_id = ?1", params![book_id])?;
        self.conn
            .execute("DELETE FROM chapters WHERE book_id = ?1", params![book_id])?;
        self.conn
            .execute("DELETE FROM books WHERE id = ?1", params![book_id])?;
        self.remove_fts_index(book_id)?;
        Ok(())
    }

    pub fn list_books(&self, limit: i64, offset: i64) -> Result<Vec<BookRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, num, title, filename, author, category, introduce, likes, word_count, page_count, download_time
             FROM books ORDER BY id ASC LIMIT ?1 OFFSET ?2",
        )?;

        let books = stmt
            .query_map(params![limit, offset], |row| {
                Ok(BookRecord {
                    id: row.get(0)?,
                    num: row.get(1)?,
                    title: row.get(2)?,
                    filename: row.get(3)?,
                    author: row.get(4)?,
                    category: row.get(5)?,
                    introduce: row.get(6)?,
                    likes: row.get(7)?,
                    word_count: row.get(8)?,
                    page_count: row.get(9)?,
                    download_time: row.get(10)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(books)
    }

    pub fn count_books(&self) -> Result<i64> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM books", [], |row| row.get(0))?;
        Ok(count)
    }

    pub fn save_book_with_chapters(
        &self,
        book: &BookRecord,
        chapters: &[(ChapterRecord, Vec<SectionRecord>)],
    ) -> Result<i64> {
        let tx = self.conn.unchecked_transaction()?;

        self.conn.execute(
            "INSERT OR REPLACE INTO books (num, title, filename, author, category, introduce, likes, word_count, page_count, download_time)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                book.num,
                book.title,
                book.filename,
                book.author,
                book.category,
                book.introduce,
                book.likes,
                book.word_count,
                book.page_count,
                book.download_time,
            ],
        )?;
        let book_id = self.conn.last_insert_rowid();

        self.conn
            .execute("DELETE FROM sections WHERE book_id = ?1", params![book_id])?;
        self.conn
            .execute("DELETE FROM chapters WHERE book_id = ?1", params![book_id])?;

        for (chapter, sections) in chapters {
            self.conn.execute(
                "INSERT INTO chapters (book_id, title, url, chapter_order)
                 VALUES (?1, ?2, ?3, ?4)",
                params![book_id, chapter.title, chapter.url, chapter.chapter_order,],
            )?;
            let chapter_id = self.conn.last_insert_rowid();

            for section in sections {
                self.conn.execute(
                    "INSERT INTO sections (chapter_id, book_id, url, content, section_order)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![
                        chapter_id,
                        book_id,
                        section.url,
                        section.content,
                        section.section_order,
                    ],
                )?;
            }
        }

        tx.commit()?;

        self.update_fts_index(book_id)?;

        Ok(book_id)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_db() -> Database {
        Database::open_in_memory().unwrap()
    }

    fn insert_test_book(db: &Database, title: &str, author: &str, content: &str) -> i64 {
        let book = BookRecord {
            id: 0,
            num: 0,
            title: title.to_string(),
            filename: title.to_string(),
            author: author.to_string(),
            category: "玄幻".to_string(),
            introduce: String::new(),
            likes: 100,
            word_count: content.len() as i64,
            page_count: 1,
            download_time: "2025-01-01 00:00:00".to_string(),
        };

        let chapters = vec![(
            ChapterRecord {
                id: 0,
                book_id: 0,
                title: "第一章".to_string(),
                url: String::new(),
                chapter_order: 1,
            },
            vec![SectionRecord {
                id: 0,
                chapter_id: 0,
                book_id: 0,
                url: String::new(),
                content: content.to_string(),
                section_order: 1,
            }],
        )];

        db.save_book_with_chapters(&book, &chapters).unwrap()
    }

    #[test]
    fn test_database_init() {
        let db = Database::open_in_memory();
        assert!(db.is_ok());
    }

    #[test]
    fn test_fts_init() {
        let db = create_test_db();
        let count = db.fts_index_count().unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_insert_and_get_book() {
        let db = Database::open_in_memory().unwrap();

        let book = BookRecord {
            id: 0,
            num: 0,
            title: "测试小说".to_string(),
            filename: "测试小说".to_string(),
            author: "测试作者".to_string(),
            category: "玄幻".to_string(),
            introduce: "这是一本测试小说".to_string(),
            likes: 100,
            word_count: 500000,
            page_count: 5,
            download_time: "2025-01-01 00:00:00".to_string(),
        };

        let book_id = db.insert_book(&book).unwrap();

        let result = db.get_book(book_id).unwrap().unwrap();
        assert_eq!(result.title, "测试小说");
        assert_eq!(result.author, "测试作者");
        assert_eq!(result.likes, 100);
    }

    #[test]
    fn test_book_exists() {
        let db = Database::open_in_memory().unwrap();

        assert!(!db.book_exists(1).unwrap());
        assert!(!db.book_exists_by_title("测试").unwrap());

        let book = BookRecord {
            id: 0,
            num: 0,
            title: "测试".to_string(),
            filename: "测试".to_string(),
            author: "".to_string(),
            category: "".to_string(),
            introduce: "".to_string(),
            likes: 0,
            word_count: 0,
            page_count: 0,
            download_time: "".to_string(),
        };
        let book_id = db.insert_book(&book).unwrap();

        assert!(db.book_exists(book_id).unwrap());
        assert!(db.book_exists_by_title("测试").unwrap());
    }

    #[test]
    fn test_insert_and_get_chapters() {
        let db = Database::open_in_memory().unwrap();

        let book = BookRecord {
            id: 0,
            num: 0,
            title: "测试".to_string(),
            filename: "测试".to_string(),
            author: "".to_string(),
            category: "".to_string(),
            introduce: "".to_string(),
            likes: 0,
            word_count: 0,
            page_count: 0,
            download_time: "".to_string(),
        };
        let book_id = db.insert_book(&book).unwrap();

        let chapter = ChapterRecord {
            id: 0,
            book_id,
            title: "第一章".to_string(),
            url: "http://example.com/ch1".to_string(),
            chapter_order: 1,
        };
        let _chapter_id = db.insert_chapter(&chapter).unwrap();

        let chapters = db.get_chapters_by_book(book_id).unwrap();
        assert_eq!(chapters.len(), 1);
        assert_eq!(chapters[0].title, "第一章");
    }

    #[test]
    fn test_insert_and_get_sections() {
        let db = Database::open_in_memory().unwrap();

        let book = BookRecord {
            id: 0,
            num: 0,
            title: "测试".to_string(),
            filename: "测试".to_string(),
            author: "".to_string(),
            category: "".to_string(),
            introduce: "".to_string(),
            likes: 0,
            word_count: 0,
            page_count: 0,
            download_time: "".to_string(),
        };
        let book_id = db.insert_book(&book).unwrap();

        let chapter = ChapterRecord {
            id: 0,
            book_id,
            title: "第一章".to_string(),
            url: "http://example.com/ch1".to_string(),
            chapter_order: 1,
        };
        let chapter_id = db.insert_chapter(&chapter).unwrap();

        let section = SectionRecord {
            id: 0,
            chapter_id,
            book_id,
            url: "http://example.com/ch1/1.html".to_string(),
            content: "这是章节内容".to_string(),
            section_order: 1,
        };
        db.insert_section(&section).unwrap();

        let sections = db.get_sections_by_chapter(chapter_id).unwrap();
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].content, "这是章节内容");
    }

    #[test]
    fn test_search_books() {
        let db = Database::open_in_memory().unwrap();

        let book1 = BookRecord {
            id: 0,
            num: 0,
            title: "斗破苍穹".to_string(),
            filename: "斗破苍穹".to_string(),
            author: "天蚕土豆".to_string(),
            category: "玄幻".to_string(),
            introduce: "".to_string(),
            likes: 1000,
            word_count: 5000000,
            page_count: 10,
            download_time: "2025-01-01 00:00:00".to_string(),
        };

        let book2 = BookRecord {
            id: 0,
            num: 0,
            title: "斗罗大陆".to_string(),
            filename: "斗罗大陆".to_string(),
            author: "唐家三少".to_string(),
            category: "玄幻".to_string(),
            introduce: "".to_string(),
            likes: 800,
            word_count: 3000000,
            page_count: 8,
            download_time: "2025-01-02 00:00:00".to_string(),
        };

        db.insert_book(&book1).unwrap();
        db.insert_book(&book2).unwrap();

        let book1 = db.get_book_by_title("斗破苍穹").unwrap().unwrap();

        let chapter = ChapterRecord {
            id: 0,
            book_id: book1.id,
            title: "第一章".to_string(),
            url: "".to_string(),
            chapter_order: 1,
        };
        db.insert_chapter(&chapter).unwrap();

        let results = db.search_books("斗").unwrap();
        assert_eq!(results.len(), 2);

        let results = db.search_books("天蚕").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "斗破苍穹");
    }

    #[test]
    fn test_delete_book() {
        let db = Database::open_in_memory().unwrap();

        let book = BookRecord {
            id: 0,
            num: 0,
            title: "测试".to_string(),
            filename: "测试".to_string(),
            author: "".to_string(),
            category: "".to_string(),
            introduce: "".to_string(),
            likes: 0,
            word_count: 0,
            page_count: 0,
            download_time: "".to_string(),
        };
        let book_id = db.insert_book(&book).unwrap();
        assert!(db.book_exists(book_id).unwrap());
        assert!(db.book_exists_by_title("测试").unwrap());

        db.delete_book(book_id).unwrap();
        assert!(!db.book_exists(book_id).unwrap());
        assert!(!db.book_exists_by_title("测试").unwrap());
    }

    #[test]
    fn test_save_book_with_chapters() {
        let db = Database::open_in_memory().unwrap();

        let book = BookRecord {
            id: 0,
            num: 0,
            title: "测试小说".to_string(),
            filename: "测试小说".to_string(),
            author: "作者".to_string(),
            category: "玄幻".to_string(),
            introduce: "简介".to_string(),
            likes: 50,
            word_count: 100000,
            page_count: 3,
            download_time: "2025-01-01 00:00:00".to_string(),
        };

        let chapters = vec![
            (
                ChapterRecord {
                    id: 0,
                    book_id: 0,
                    title: "第一章".to_string(),
                    url: "http://example.com/1".to_string(),
                    chapter_order: 1,
                },
                vec![SectionRecord {
                    id: 0,
                    chapter_id: 0,
                    book_id: 0,
                    url: "http://example.com/1/1.html".to_string(),
                    content: "第一章内容".to_string(),
                    section_order: 1,
                }],
            ),
            (
                ChapterRecord {
                    id: 0,
                    book_id: 0,
                    title: "第二章".to_string(),
                    url: "http://example.com/2".to_string(),
                    chapter_order: 2,
                },
                vec![SectionRecord {
                    id: 0,
                    chapter_id: 0,
                    book_id: 0,
                    url: "http://example.com/2/1.html".to_string(),
                    content: "第二章内容".to_string(),
                    section_order: 1,
                }],
            ),
        ];

        let book_id = db.save_book_with_chapters(&book, &chapters).unwrap();

        let result = db.get_book(book_id).unwrap().unwrap();
        assert_eq!(result.title, "测试小说");

        let chapters_result = db.get_chapters_by_book(book_id).unwrap();
        assert_eq!(chapters_result.len(), 2);

        let sections = db.get_sections_by_book(book_id).unwrap();
        assert_eq!(sections.len(), 2);
    }

    #[test]
    fn test_get_chapter_by_book_and_order() {
        let db = Database::open_in_memory().unwrap();

        let book = BookRecord {
            id: 0,
            num: 0,
            title: "测试".to_string(),
            filename: "测试".to_string(),
            author: "".to_string(),
            category: "".to_string(),
            introduce: "".to_string(),
            likes: 0,
            word_count: 0,
            page_count: 0,
            download_time: "".to_string(),
        };
        let book_id = db.insert_book(&book).unwrap();

        let chapter = ChapterRecord {
            id: 0,
            book_id,
            title: "第三章".to_string(),
            url: "".to_string(),
            chapter_order: 3,
        };
        db.insert_chapter(&chapter).unwrap();

        let result = db
            .get_chapter_by_book_and_order(book_id, 3)
            .unwrap()
            .unwrap();
        assert_eq!(result.title, "第三章");

        let none = db.get_chapter_by_book_and_order(book_id, 99).unwrap();
        assert!(none.is_none());
    }

    #[test]
    fn test_fts_index_auto_update() {
        let db = create_test_db();

        insert_test_book(&db, "斗破苍穹", "天蚕土豆", "萧炎在斗气大陆修炼成长的故事");

        let fts_count = db.fts_index_count().unwrap();
        assert_eq!(fts_count, 1);
    }

    #[test]
    fn test_fts_rebuild_index() {
        let db = create_test_db();

        insert_test_book(&db, "斗破苍穹", "天蚕土豆", "萧炎修炼");
        insert_test_book(&db, "斗罗大陆", "唐家三少", "唐三修炼");

        let count = db.rebuild_fts_index().unwrap();
        assert_eq!(count, 2);

        let fts_count = db.fts_index_count().unwrap();
        assert_eq!(fts_count, 2);
    }

    #[test]
    fn test_fts_search_basic() {
        let db = create_test_db();

        insert_test_book(&db, "斗破苍穹", "天蚕土豆", "萧炎在斗气大陆修炼成长的故事");
        insert_test_book(&db, "斗罗大陆", "唐家三少", "唐三在斗罗大陆的冒险经历");

        let results = db.search_fts("斗", false, SearchField::All, 10, 0).unwrap();
        assert!(!results.is_empty());
    }

    #[test]
    fn test_fts_search_by_title() {
        let db = create_test_db();

        insert_test_book(&db, "斗破苍穹", "天蚕土豆", "萧炎修炼");
        insert_test_book(&db, "凡人修仙传", "忘语", "韩立修仙");

        let results = db
            .search_fts("斗破", false, SearchField::Title, 10, 0)
            .unwrap();
        assert!(!results.is_empty());
        assert!(results[0].title.contains("斗破"));
    }

    #[test]
    fn test_fts_search_by_author() {
        let db = create_test_db();

        insert_test_book(&db, "斗破苍穹", "天蚕土豆", "萧炎修炼");
        insert_test_book(&db, "凡人修仙传", "忘语", "韩立修仙");

        let results_all = db.search_fts("天", false, SearchField::All, 10, 0).unwrap();
        assert!(!results_all.is_empty());

        let results_author = db
            .search_fts("天", false, SearchField::Author, 10, 0)
            .unwrap();
        if !results_author.is_empty() {
            assert!(results_author[0].author.contains("天"));
        }
    }

    #[test]
    fn test_fts_search_by_content() {
        let db = create_test_db();

        insert_test_book(&db, "斗破苍穹", "天蚕土豆", "萧炎在斗气大陆修炼成长");
        insert_test_book(&db, "凡人修仙传", "忘语", "韩立在修仙界成长");

        let results = db
            .search_fts("修炼", false, SearchField::Content, 10, 0)
            .unwrap();
        assert!(!results.is_empty());
    }

    #[test]
    fn test_fts_search_relevance_score() {
        let db = create_test_db();

        insert_test_book(&db, "斗破苍穹", "天蚕土豆", "斗破苍穹斗破苍穹斗破");
        insert_test_book(&db, "斗罗大陆", "唐家三少", "唐三修炼");

        let results = db
            .search_fts("斗破", false, SearchField::All, 10, 0)
            .unwrap();
        assert!(!results.is_empty());

        for result in &results {
            assert!(result.relevance_score >= 0.0 && result.relevance_score <= 100.0);
        }
    }

    #[test]
    fn test_fts_search_no_results() {
        let db = create_test_db();

        insert_test_book(&db, "斗破苍穹", "天蚕土豆", "萧炎修炼");

        let results = db
            .search_fts("不存在的关键词xyz", false, SearchField::All, 10, 0)
            .unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_fts_search_count() {
        let db = create_test_db();

        insert_test_book(&db, "斗破苍穹", "天蚕土豆", "萧炎修炼");
        insert_test_book(&db, "斗罗大陆", "唐家三少", "唐三修炼");

        let count = db.search_fts_count("斗", false).unwrap();
        assert!(count >= 2);
    }

    #[test]
    fn test_fts_delete_removes_index() {
        let db = create_test_db();

        let book_id = insert_test_book(&db, "测试删除", "作者", "内容");
        assert_eq!(db.fts_index_count().unwrap(), 1);

        db.delete_book(book_id).unwrap();
        assert_eq!(db.fts_index_count().unwrap(), 0);
    }

    #[test]
    fn test_fts_update_index() {
        let db = create_test_db();

        let book_id = insert_test_book(&db, "测试更新", "作者", "旧内容");
        assert_eq!(db.fts_index_count().unwrap(), 1);

        db.update_fts_index(book_id).unwrap();
        assert_eq!(db.fts_index_count().unwrap(), 1);
    }

    #[test]
    fn test_fts_remove_index() {
        let db = create_test_db();

        let book_id = insert_test_book(&db, "测试移除", "作者", "内容");
        assert_eq!(db.fts_index_count().unwrap(), 1);

        db.remove_fts_index(book_id).unwrap();
        assert_eq!(db.fts_index_count().unwrap(), 0);
    }

    #[test]
    fn test_fts_search_phrase() {
        let db = create_test_db();

        insert_test_book(&db, "斗破苍穹", "天蚕土豆", "萧炎在斗气大陆修炼成长的故事");

        let results = db
            .search_fts("斗气", true, SearchField::All, 10, 0)
            .unwrap();
        assert!(!results.is_empty());
    }

    #[test]
    fn test_fts_search_prefix() {
        let db = create_test_db();

        insert_test_book(&db, "斗破苍穹", "天蚕土豆", "萧炎修炼成长");

        let results = db
            .search_fts("修*", false, SearchField::All, 10, 0)
            .unwrap();
        assert!(!results.is_empty());
    }

    #[test]
    fn test_fts_search_pagination() {
        let db = create_test_db();

        for i in 0..5 {
            insert_test_book(
                &db,
                &format!("测试小说{}", i),
                &format!("作者{}", i),
                &format!("这是第{}本小说的内容，包含修炼和成长", i),
            );
        }

        let results_page1 = db
            .search_fts("修炼", false, SearchField::All, 2, 0)
            .unwrap();
        assert!(results_page1.len() <= 2);

        let results_page2 = db
            .search_fts("修炼", false, SearchField::All, 2, 2)
            .unwrap();
        assert!(results_page2.len() <= 2);
    }

    #[test]
    fn test_fts_search_result_fields() {
        let db = create_test_db();

        insert_test_book(&db, "斗破苍穹", "天蚕土豆", "萧炎修炼成长的故事");

        let results = db
            .search_fts("斗破", false, SearchField::All, 10, 0)
            .unwrap();
        assert!(!results.is_empty());

        let result = &results[0];
        assert!(!result.title.is_empty());
        assert!(!result.author.is_empty());
        assert!(result.relevance_score >= 0.0);
    }

    #[test]
    fn test_fts_search_special_chars() {
        let db = create_test_db();

        insert_test_book(&db, "测试小说", "作者", "内容包含特殊字符！@#￥%");

        let results = db
            .search_fts("测试", false, SearchField::All, 10, 0)
            .unwrap();
        assert!(!results.is_empty());
    }

    #[test]
    fn test_fts_search_long_content() {
        let db = create_test_db();

        let long_content = "这是一段很长的内容。".repeat(1000);
        insert_test_book(&db, "长篇小说", "作者", &long_content);

        let results = db
            .search_fts("长篇", false, SearchField::All, 10, 0)
            .unwrap();
        assert!(!results.is_empty());
    }
}
