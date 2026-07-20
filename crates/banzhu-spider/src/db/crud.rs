//! CRUD operations for books, chapters, sections, bookshelf, reading progress, and crawl logs.

use crate::db::models::{
    BookRecord, BookshelfRecord, ChapterRecord, CrawlLogRecord, ReadingProgressRecord,
    SearchRecord, SectionRecord,
};
use crate::db::Database;
use anyhow::Result;
use rusqlite::{params, OptionalExtension};

impl Database {
    pub fn insert_book(&self, book: &BookRecord) -> Result<i64> {
        let now = chrono::Utc::now().timestamp();
        self.conn.execute(
            "INSERT OR REPLACE INTO books (website_book_id, path_num, title, filename, author, category, introduce, likes, word_count, page_count, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                book.website_book_id,
                book.path_num,
                book.title,
                book.filename,
                book.author,
                book.category,
                book.introduce,
                book.likes,
                book.word_count,
                book.page_count,
                now,
                now,
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn insert_chapter(&self, chapter: &ChapterRecord) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO chapters (book_id, title, url, chapter_order, word_count)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                chapter.book_id,
                chapter.title,
                chapter.url,
                chapter.chapter_order,
                chapter.word_count,
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
            "SELECT id, website_book_id, path_num, title, filename, author, category, introduce, likes, word_count, page_count, created_at, updated_at
             FROM books WHERE id = ?1",
        )?;

        let result = stmt
            .query_row(params![book_id], |row| {
                Ok(BookRecord {
                    id: row.get(0)?,
                    website_book_id: row.get(1)?,
                    path_num: row.get(2)?,
                    title: row.get(3)?,
                    filename: row.get(4)?,
                    author: row.get(5)?,
                    category: row.get(6)?,
                    introduce: row.get(7)?,
                    likes: row.get(8)?,
                    word_count: row.get(9)?,
                    page_count: row.get(10)?,
                    created_at: row.get(11)?,
                    updated_at: row.get(12)?,
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

    /// 按网站 book_id 检查是否已存在
    pub fn book_exists_by_website_id(&self, website_id: i64) -> Result<bool> {
        let result: Option<i64> = self
            .conn
            .query_row(
                "SELECT id FROM books WHERE website_book_id = ?1",
                params![website_id],
                |row| row.get(0),
            )
            .optional()?;
        Ok(result.is_some())
    }

    /// 按网站 book_id 获取已有章节数
    pub fn get_chapters_count_by_website_id(&self, website_id: i64) -> Result<usize> {
        let book: Option<i64> = self
            .conn
            .query_row(
                "SELECT id FROM books WHERE website_book_id = ?1",
                params![website_id],
                |row| row.get(0),
            )
            .optional()?;
        match book {
            Some(book_id) => {
                let count: i64 = self.conn.query_row(
                    "SELECT COUNT(*) FROM chapters WHERE book_id = ?1",
                    params![book_id],
                    |row| row.get(0),
                )?;
                Ok(count as usize)
            }
            None => Ok(0),
        }
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
            "SELECT id, website_book_id, path_num, title, filename, author, category, introduce, likes, word_count, page_count, created_at, updated_at
             FROM books WHERE title = ?1",
        )?;

        let result = stmt
            .query_row(params![title], |row| {
                Ok(BookRecord {
                    id: row.get(0)?,
                    website_book_id: row.get(1)?,
                    path_num: row.get(2)?,
                    title: row.get(3)?,
                    filename: row.get(4)?,
                    author: row.get(5)?,
                    category: row.get(6)?,
                    introduce: row.get(7)?,
                    likes: row.get(8)?,
                    word_count: row.get(9)?,
                    page_count: row.get(10)?,
                    created_at: row.get(11)?,
                    updated_at: row.get(12)?,
                })
            })
            .optional()?;

        Ok(result)
    }

    pub fn get_chapters_by_book(&self, book_id: i64) -> Result<Vec<ChapterRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, book_id, title, url, chapter_order, word_count
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
                    word_count: row.get(5)?,
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
                   b.created_at
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
                    created_at: row.get(6)?,
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
                "SELECT id, book_id, title, url, chapter_order, word_count
                 FROM chapters WHERE book_id = ?1 AND chapter_order = ?2",
                params![book_id, chapter_order],
                |row| {
                    Ok(ChapterRecord {
                        id: row.get(0)?,
                        book_id: row.get(1)?,
                        title: row.get(2)?,
                        url: row.get(3)?,
                        chapter_order: row.get(4)?,
                        word_count: row.get(5)?,
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
            "SELECT id, website_book_id, path_num, title, filename, author, category, introduce, likes, word_count, page_count, created_at, updated_at
             FROM books ORDER BY id ASC LIMIT ?1 OFFSET ?2",
        )?;

        let books = stmt
            .query_map(params![limit, offset], |row| {
                Ok(BookRecord {
                    id: row.get(0)?,
                    website_book_id: row.get(1)?,
                    path_num: row.get(2)?,
                    title: row.get(3)?,
                    filename: row.get(4)?,
                    author: row.get(5)?,
                    category: row.get(6)?,
                    introduce: row.get(7)?,
                    likes: row.get(8)?,
                    word_count: row.get(9)?,
                    page_count: row.get(10)?,
                    created_at: row.get(11)?,
                    updated_at: row.get(12)?,
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

    pub fn list_categories(&self) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT DISTINCT category FROM books WHERE category != '' ORDER BY category"
        )?;
        let categories = stmt
            .query_map([], |row| row.get(0))?
            .collect::<std::result::Result<Vec<String>, _>>()?;
        Ok(categories)
    }

    pub fn save_book_with_chapters(
        &self,
        book: &BookRecord,
        chapters: &[(ChapterRecord, Vec<SectionRecord>)],
    ) -> Result<i64> {
        let tx = self.conn.unchecked_transaction()?;
        let now = chrono::Utc::now().timestamp();

        self.conn.execute(
            "INSERT OR REPLACE INTO books (website_book_id, path_num, title, filename, author, category, introduce, likes, word_count, page_count, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                book.website_book_id,
                book.path_num,
                book.title,
                book.filename,
                book.author,
                book.category,
                book.introduce,
                book.likes,
                book.word_count,
                book.page_count,
                now,
                now,
            ],
        )?;
        let book_id = self.conn.last_insert_rowid();

        self.conn
            .execute("DELETE FROM sections WHERE book_id = ?1", params![book_id])?;
        self.conn
            .execute("DELETE FROM chapters WHERE book_id = ?1", params![book_id])?;

        for (chapter, sections) in chapters {
            self.conn.execute(
                "INSERT INTO chapters (book_id, title, url, chapter_order, word_count)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![book_id, chapter.title, chapter.url, chapter.chapter_order, chapter.word_count],
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

    // ─── Bookshelf CRUD ──────────────────────────────────────────────────────

    pub fn add_to_bookshelf(&self, book_id: i64, group: &str) -> Result<()> {
        let now = chrono::Utc::now().timestamp();
        self.conn.execute(
            "INSERT OR REPLACE INTO bookshelf (book_id, group_name, added_at) VALUES (?1, ?2, ?3)",
            params![book_id, group, now],
        )?;
        Ok(())
    }

    pub fn get_bookshelf(&self, group: Option<&str>) -> Result<Vec<BookshelfRecord>> {
        let sql = match group {
            Some(_) => "SELECT id, book_id, group_name, added_at FROM bookshelf WHERE group_name = ?1 ORDER BY added_at DESC",
            None => "SELECT id, book_id, group_name, added_at FROM bookshelf ORDER BY added_at DESC",
        };
        let mut stmt = self.conn.prepare(sql)?;
        let rows = match group {
            Some(g) => stmt
                .query_map(params![g], |row| {
                    Ok(BookshelfRecord {
                        id: row.get(0)?,
                        book_id: row.get(1)?,
                        group_name: row.get(2)?,
                        added_at: row.get(3)?,
                    })
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?,
            None => stmt
                .query_map([], |row| {
                    Ok(BookshelfRecord {
                        id: row.get(0)?,
                        book_id: row.get(1)?,
                        group_name: row.get(2)?,
                        added_at: row.get(3)?,
                    })
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?,
        };
        Ok(rows)
    }

    pub fn update_bookshelf_group(&self, book_id: i64, group: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE bookshelf SET group_name = ?2 WHERE book_id = ?1",
            params![book_id, group],
        )?;
        Ok(())
    }

    pub fn remove_from_bookshelf(&self, book_id: i64) -> Result<()> {
        self.conn
            .execute("DELETE FROM bookshelf WHERE book_id = ?1", params![book_id])?;
        Ok(())
    }

    // ─── Reading Progress CRUD ───────────────────────────────────────────────

    pub fn get_progress(&self, book_id: i64) -> Result<Option<ReadingProgressRecord>> {
        let result = self
            .conn
            .query_row(
                "SELECT id, book_id, chapter_order, page_index, updated_at FROM reading_progress WHERE book_id = ?1",
                params![book_id],
                |row| {
                    let updated_at: i64 = row.get(4)?;
                    Ok(ReadingProgressRecord {
                        id: row.get(0)?,
                        book_id: row.get(1)?,
                        chapter_order: row.get(2)?,
                        page_index: row.get(3)?,
                        // Task 6 will ALTER TABLE to add last_read_at column; reuse updated_at for now
                        last_read_at: updated_at,
                        updated_at,
                    })
                },
            )
            .optional()?;
        Ok(result)
    }

    pub fn update_progress(&self, book_id: i64, chapter_order: i64, page_index: i64) -> Result<()> {
        let now = chrono::Utc::now().timestamp();
        self.conn.execute(
            "INSERT OR REPLACE INTO reading_progress (book_id, chapter_order, page_index, updated_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![book_id, chapter_order, page_index, now],
        )?;
        Ok(())
    }

    // ─── Crawl Logs CRUD ─────────────────────────────────────────────────────

    pub fn insert_crawl_log(&self, level: &str, message: &str) -> Result<()> {
        let now = chrono::Utc::now().timestamp();
        self.conn.execute(
            "INSERT INTO crawl_logs (level, message, created_at) VALUES (?1, ?2, ?3)",
            params![level, message, now],
        )?;
        // Keep max 500 rows
        self.conn.execute(
            "DELETE FROM crawl_logs WHERE id NOT IN (SELECT id FROM crawl_logs ORDER BY id DESC LIMIT 500)",
            [],
        )?;
        Ok(())
    }

    pub fn get_crawl_logs(&self, limit: i64) -> Result<Vec<CrawlLogRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, level, message, created_at FROM crawl_logs ORDER BY id DESC LIMIT ?1",
        )?;
        let logs = stmt
            .query_map(params![limit], |row| {
                Ok(CrawlLogRecord {
                    id: row.get(0)?,
                    level: row.get(1)?,
                    message: row.get(2)?,
                    created_at: row.get(3)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(logs)
    }
}
