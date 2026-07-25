//! CRUD operations for books, chapters, sections, bookshelf, reading progress, and crawl logs.

use crate::db::models::{
    BookRecord, BookshelfRecord, ChapterRecord, CrawlLogRecord, CrawlTaskRecord,
    ReadingGoalRecord, ReadingProgressRecord, SearchRecord, SectionRecord,
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

    /// 按网站 book_id 获取书籍
    pub fn get_book_by_website_id(&self, website_id: i64) -> Result<Option<BookRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, website_book_id, path_num, title, filename, author, category, introduce, likes, word_count, page_count, created_at, updated_at
             FROM books WHERE website_book_id = ?1",
        )?;
        let result = stmt
            .query_row(params![website_id], |row| {
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

    pub fn list_books(&self, limit: i64, offset: i64, category: Option<&str>) -> Result<Vec<BookRecord>> {
        let sql = match category {
            Some(_) => "SELECT id, website_book_id, path_num, title, filename, author, category, introduce, likes, word_count, page_count, created_at, updated_at
             FROM books WHERE category = ?1 ORDER BY id ASC LIMIT ?2 OFFSET ?3",
            None => "SELECT id, website_book_id, path_num, title, filename, author, category, introduce, likes, word_count, page_count, created_at, updated_at
             FROM books ORDER BY id ASC LIMIT ?1 OFFSET ?2",
        };
        let mut stmt = self.conn.prepare(sql)?;
        let books = match category {
            Some(c) => stmt
                .query_map(params![c, limit, offset], map_book_row)?
                .collect::<std::result::Result<Vec<_>, _>>()?,
            None => stmt
                .query_map(params![limit, offset], map_book_row)?
                .collect::<std::result::Result<Vec<_>, _>>()?,
        };
        Ok(books)
    }

    pub fn count_books(&self) -> Result<i64> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM books", [], |row| row.get(0))?;
        Ok(count)
    }

    pub fn count_books_by_category(&self, category: &str) -> Result<i64> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM books WHERE category = ?1",
            params![category],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    /// 统计指定书的章节数（直接 SQL COUNT，避免拉取全表）
    pub fn count_chapters_by_book(&self, book_id: i64) -> Result<i64> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM chapters WHERE book_id = ?1",
            params![book_id],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    /// 全站总章节数
    pub fn count_all_chapters(&self) -> Result<i64> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM chapters", [], |row| row.get(0))?;
        Ok(count)
    }

    /// 全站总字数
    pub fn sum_all_word_count(&self) -> Result<i64> {
        let total: i64 = self.conn.query_row(
            "SELECT COALESCE(SUM(word_count), 0) FROM books",
            [],
            |row| row.get(0),
        )?;
        Ok(total)
    }

    /// 分类分布：返回 (category_name, count) 列表
    pub fn category_distribution(&self) -> Result<Vec<(String, i64)>> {
        let mut stmt = self.conn.prepare(
            "SELECT category, COUNT(*) as cnt FROM books
             WHERE category != ''
             GROUP BY category ORDER BY cnt DESC",
        )?;
        let rows = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows)
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

    /// 批量 upsert 书籍记录（按 website_book_id 幂等）。
    ///
    /// **返回值语义**：返回值 = 已处理的输入条目数（输入切片长度），
    /// 不区分实际是 INSERT 还是 REPLACE。
    /// 调用方如需确知受影响行数，应另行查询数据库。
    pub fn batch_upsert_books(&self, books: &[BookRecord]) -> Result<usize> {
        let now = chrono::Utc::now().timestamp();
        let tx = self.conn.unchecked_transaction()?;
        let mut count = 0;
        for book in books {
            tx.execute(
                "INSERT OR REPLACE INTO books
                 (website_book_id, path_num, title, filename, author, category, introduce,
                  likes, word_count, page_count, created_at, updated_at)
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
            count += 1;
        }
        tx.commit()?;
        Ok(count)
    }

    /// 批量 upsert 章节记录。
    /// chapters: Vec<(website_book_id, ChapterRecord)> —— 方法内部 JOIN books 解析 book_id
    ///
    /// **返回值语义**：返回值 = 已处理的输入条目数（输入切片长度），
    /// 不区分实际是 INSERT、REPLACE 还是 no-op。
    /// **JOIN 不命中的行为**：当 `website_book_id` 在 books 表中不存在时，
    /// `SELECT ... FROM books b WHERE b.website_book_id = ?` 返回 0 行，
    /// INSERT 退化为 no-op，但 `count` 仍会自增。
    /// 调用方如需确知实际写入行数，应另行查询数据库（例如 `get_chapters_by_book`）。
    pub fn batch_upsert_chapters(&self, chapters: &[(i64, ChapterRecord)]) -> Result<usize> {
        let tx = self.conn.unchecked_transaction()?;
        let mut count = 0;
        for (website_book_id, ch) in chapters {
            tx.execute(
                "INSERT OR REPLACE INTO chapters (book_id, title, url, chapter_order, word_count)
                 SELECT b.id, ?1, ?2, ?3, ?4
                 FROM books b
                 WHERE b.website_book_id = ?5",
                params![ch.title, ch.url, ch.chapter_order, ch.word_count, website_book_id],
            )?;
            count += 1;
        }
        tx.commit()?;
        Ok(count)
    }

    /// 批量 upsert section 记录。
    /// sections: Vec<(website_book_id, chapter_order, SectionRecord)> ——
    /// 方法内部 JOIN books + chapters 解析 chapter_id 和 book_id
    ///
    /// **返回值语义**：返回值 = 已处理的输入条目数（输入切片长度），
    /// 不区分实际是 INSERT、REPLACE 还是 no-op。
    /// **JOIN 不命中的行为**：当 `website_book_id` 不在 books 表、或对应 book
    /// 下不存在指定 `chapter_order` 的章节时，`SELECT ... FROM chapters c JOIN
    /// books b ...` 返回 0 行，INSERT 退化为 no-op，但 `count` 仍会自增。
    /// 调用方如需确知实际写入行数，应另行查询数据库（例如
    /// `get_sections_by_chapter` / `get_sections_by_book`）。
    pub fn batch_upsert_sections(&self, sections: &[(i64, i64, SectionRecord)]) -> Result<usize> {
        let tx = self.conn.unchecked_transaction()?;
        let mut count = 0;
        for (website_book_id, chapter_order, sec) in sections {
            tx.execute(
                "INSERT OR REPLACE INTO sections (chapter_id, book_id, url, content, section_order)
                 SELECT c.id, c.book_id, ?1, ?2, ?3
                 FROM chapters c
                 JOIN books b ON b.id = c.book_id
                 WHERE b.website_book_id = ?4 AND c.chapter_order = ?5",
                params![sec.url, sec.content, sec.section_order, website_book_id, chapter_order],
            )?;
            count += 1;
        }
        tx.commit()?;
        Ok(count)
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
                "SELECT id, book_id, chapter_order, page_index, last_read_at, updated_at FROM reading_progress WHERE book_id = ?1",
                params![book_id],
                |row| {
                    Ok(ReadingProgressRecord {
                        id: row.get(0)?,
                        book_id: row.get(1)?,
                        chapter_order: row.get(2)?,
                        page_index: row.get(3)?,
                        last_read_at: row.get(4)?,
                        updated_at: row.get(5)?,
                    })
                },
            )
            .optional()?;
        Ok(result)
    }

    pub fn update_progress(&self, book_id: i64, chapter_order: i64, page_index: i64) -> Result<()> {
        let now = chrono::Utc::now().timestamp();
        self.conn.execute(
            "INSERT OR REPLACE INTO reading_progress (book_id, chapter_order, page_index, last_read_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?4)",
            params![book_id, chapter_order, page_index, now],
        )?;
        Ok(())
    }

    // ─── Crawl Logs CRUD ─────────────────────────────────────────────────────

    pub fn insert_crawl_log(&self, level: &str, message: &str) -> Result<i64> {
        let now = chrono::Utc::now().timestamp();
        self.conn.execute(
            "INSERT INTO crawl_logs (level, message, created_at) VALUES (?1, ?2, ?3)",
            params![level, message, now],
        )?;
        let new_id = self.conn.last_insert_rowid();
        // Keep max 500 rows
        self.conn.execute(
            "DELETE FROM crawl_logs WHERE id NOT IN (SELECT id FROM crawl_logs ORDER BY id DESC LIMIT 500)",
            [],
        )?;
        Ok(new_id)
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

    // ─── Crawl Tasks CRUD ────────────────────────────────────────────────────

    /// 插入或刷新一条爬取任务（按 website_book_id UPSERT）。
    /// 适用于开始爬取前的初始化。
    pub fn upsert_crawl_task_pending(
        &self,
        website_book_id: i64,
        title: &str,
        trigger: &str,
    ) -> Result<()> {
        let now = chrono::Utc::now().timestamp();
        self.conn.execute(
            r#"INSERT INTO crawl_tasks
                (website_book_id, title, status, progress, trigger, created_at, updated_at)
              VALUES (?1, ?2, 'pending', 0, ?3, ?4, ?4)
              ON CONFLICT(website_book_id) DO UPDATE SET
                title = excluded.title,
                status = 'pending',
                progress = 0,
                error_message = '',
                trigger = excluded.trigger,
                chapters_total = 0,
                chapters_done = 0,
                started_at = NULL,
                finished_at = NULL,
                updated_at = excluded.updated_at"#,
            params![website_book_id, title, trigger, now],
        )?;
        Ok(())
    }

    /// 标记任务为运行中
    pub fn mark_crawl_task_running(&self, website_book_id: i64) -> Result<()> {
        let now = chrono::Utc::now().timestamp();
        self.conn.execute(
            r#"UPDATE crawl_tasks
               SET status = 'running', progress = 0, started_at = ?2, finished_at = NULL, updated_at = ?2
               WHERE website_book_id = ?1"#,
            params![website_book_id, now],
        )?;
        Ok(())
    }

    /// 更新任务进度（章节级别）
    pub fn update_crawl_task_progress(
        &self,
        website_book_id: i64,
        chapters_total: i64,
        chapters_done: i64,
    ) -> Result<()> {
        let now = chrono::Utc::now().timestamp();
        let total = chapters_total.max(1);
        let progress = ((chapters_done as f64 / total as f64) * 100.0) as i64;
        self.conn.execute(
            r#"UPDATE crawl_tasks
               SET chapters_total = ?2, chapters_done = ?3, progress = ?4, updated_at = ?5
               WHERE website_book_id = ?1"#,
            params![
                website_book_id,
                chapters_total,
                chapters_done,
                progress,
                now
            ],
        )?;
        Ok(())
    }

    /// 标记任务成功完成
    pub fn mark_crawl_task_success(
        &self,
        website_book_id: i64,
        book_id: Option<i64>,
        chapters_done: i64,
    ) -> Result<()> {
        let now = chrono::Utc::now().timestamp();
        self.conn.execute(
            r#"UPDATE crawl_tasks
               SET status = 'success', progress = 100, book_id = ?2,
                   chapters_done = ?3, finished_at = ?4, error_message = '', updated_at = ?4
               WHERE website_book_id = ?1"#,
            params![website_book_id, book_id, chapters_done, now],
        )?;
        Ok(())
    }

    /// 标记任务失败
    pub fn mark_crawl_task_failed(
        &self,
        website_book_id: i64,
        error_message: &str,
    ) -> Result<()> {
        let now = chrono::Utc::now().timestamp();
        self.conn.execute(
            r#"UPDATE crawl_tasks
               SET status = 'failed', finished_at = ?3, error_message = ?2, updated_at = ?3
               WHERE website_book_id = ?1"#,
            params![website_book_id, error_message, now],
        )?;
        Ok(())
    }

    /// 标记任务跳过（已存在且无更新）
    pub fn mark_crawl_task_skipped(&self, website_book_id: i64) -> Result<()> {
        let now = chrono::Utc::now().timestamp();
        self.conn.execute(
            r#"UPDATE crawl_tasks
               SET status = 'skipped', progress = 100, finished_at = ?2, updated_at = ?2
               WHERE website_book_id = ?1"#,
            params![website_book_id, now],
        )?;
        Ok(())
    }

    /// 按状态查询爬取任务（status=None 表示全部）
    pub fn list_crawl_tasks(
        &self,
        status: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<CrawlTaskRecord>> {
        let sql = match status {
            Some(_) => r#"SELECT id, website_book_id, book_id, title, status, progress,
                                 chapters_total, chapters_done, error_message, trigger,
                                 started_at, finished_at, created_at, updated_at
                          FROM crawl_tasks WHERE status = ?1
                          ORDER BY updated_at DESC LIMIT ?2 OFFSET ?3"#,
            None => r#"SELECT id, website_book_id, book_id, title, status, progress,
                              chapters_total, chapters_done, error_message, trigger,
                              started_at, finished_at, created_at, updated_at
                       FROM crawl_tasks
                       ORDER BY updated_at DESC LIMIT ?2 OFFSET ?3"#,
        };
        let mut stmt = self.conn.prepare(sql)?;
        let rows = match status {
            Some(s) => stmt
                .query_map(params![s, limit, offset], map_crawl_task)?
                .collect::<std::result::Result<Vec<_>, _>>()?,
            None => stmt
                .query_map(params![limit, offset], map_crawl_task)?
                .collect::<std::result::Result<Vec<_>, _>>()?,
        };
        Ok(rows)
    }

    /// 统计各状态的任务数
    pub fn count_crawl_tasks_by_status(&self) -> Result<CrawlTaskStatusCount> {
        let mut stmt = self.conn.prepare(
            r#"SELECT
                SUM(CASE WHEN status='pending' THEN 1 ELSE 0 END) as pending,
                SUM(CASE WHEN status='running' THEN 1 ELSE 0 END) as running,
                SUM(CASE WHEN status='success' THEN 1 ELSE 0 END) as success,
                SUM(CASE WHEN status='failed' THEN 1 ELSE 0 END) as failed,
                SUM(CASE WHEN status='skipped' THEN 1 ELSE 0 END) as skipped,
                COUNT(*) as total
               FROM crawl_tasks"#,
        )?;
        let result = stmt.query_row([], |row| {
            Ok(CrawlTaskStatusCount {
                pending: row.get::<_, Option<i64>>(0)?.unwrap_or(0),
                running: row.get::<_, Option<i64>>(1)?.unwrap_or(0),
                success: row.get::<_, Option<i64>>(2)?.unwrap_or(0),
                failed: row.get::<_, Option<i64>>(3)?.unwrap_or(0),
                skipped: row.get::<_, Option<i64>>(4)?.unwrap_or(0),
                total: row.get::<_, Option<i64>>(5)?.unwrap_or(0),
            })
        })?;
        Ok(result)
    }

    /// 获取单个爬取任务（按 website_book_id）
    pub fn get_crawl_task(&self, website_book_id: i64) -> Result<Option<CrawlTaskRecord>> {
        let result = self
            .conn
            .query_row(
                r#"SELECT id, website_book_id, book_id, title, status, progress,
                          chapters_total, chapters_done, error_message, trigger,
                          started_at, finished_at, created_at, updated_at
                   FROM crawl_tasks WHERE website_book_id = ?1"#,
                params![website_book_id],
                map_crawl_task,
            )
            .optional()?;
        Ok(result)
    }

    // ─── Reading Sessions CRUD ──────────────────────────────────────────────

    pub fn insert_reading_session(
        &self,
        book_id: i64,
        chapter_order: i64,
        duration_sec: i64,
        chapters_read: i64,
        started_at: i64,
        ended_at: i64,
    ) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO reading_sessions (book_id, chapter_order, duration_sec, chapters_read, started_at, ended_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![book_id, chapter_order, duration_sec, chapters_read, started_at, ended_at],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn sum_today_reading(&self) -> Result<(i64, i64)> {
        let row = self.conn.query_row(
            "SELECT COALESCE(SUM(duration_sec), 0) AS duration,
                    COALESCE(SUM(chapters_read), 0) AS chapters
             FROM reading_sessions
             WHERE started_at >= strftime('%s', 'now', 'start of day', 'localtime')",
            [],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
        )?;
        Ok(row)
    }

    pub fn heatmap_data(&self, year: i32) -> Result<Vec<(String, i64, i64)>> {
        let start = format!("{}-01-01T00:00:00", year);
        let end = format!("{}-01-01T00:00:00", year + 1);
        let mut stmt = self.conn.prepare(
            "SELECT date(started_at, 'unixepoch', 'localtime') AS date,
                    COALESCE(SUM(duration_sec), 0) AS duration,
                    COALESCE(SUM(chapters_read), 0) AS chapters
             FROM reading_sessions
             WHERE started_at >= strftime('%s', ?1)
               AND started_at <  strftime('%s', ?2)
             GROUP BY date",
        )?;
        let rows = stmt
            .query_map(params![start, end], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn reading_timeline(&self, days: i32) -> Result<Vec<(String, i64, i64)>> {
        let mut stmt = self.conn.prepare(
            "SELECT date(started_at, 'unixepoch', 'localtime') AS date,
                    COALESCE(SUM(duration_sec), 0) AS duration,
                    COALESCE(SUM(chapters_read), 0) AS chapters
             FROM reading_sessions
             WHERE started_at >= strftime('%s', 'now', ?1)
             GROUP BY date
             ORDER BY date ASC",
        )?;
        let rows = stmt
            .query_map(params![format!("-{} days", days)], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn reading_history(&self, limit: i64) -> Result<Vec<ReadingHistoryRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT rs.book_id, b.title AS book_title,
                    MAX(rs.started_at) AS last_read_at,
                    MAX(rs.chapter_order) AS last_chapter_order,
                    COALESCE(SUM(rs.duration_sec), 0) AS total_duration_sec,
                    COALESCE(SUM(rs.chapters_read), 0) AS chapters_read
             FROM reading_sessions rs
             LEFT JOIN books b ON b.id = rs.book_id
             GROUP BY rs.book_id
             ORDER BY last_read_at DESC
             LIMIT ?1",
        )?;
        let rows = stmt
            .query_map(params![limit], |row| {
                Ok(ReadingHistoryRow {
                    book_id: row.get(0)?,
                    book_title: row.get(1)?,
                    last_read_at: row.get(2)?,
                    last_chapter_order: row.get(3)?,
                    total_duration_sec: row.get(4)?,
                    chapters_read: row.get(5)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    // ─── Reading Goals CRUD ─────────────────────────────────────────────────

    pub fn get_reading_goal(&self) -> Result<ReadingGoalRecord> {
        let row = self.conn.query_row(
            "SELECT id, daily_minutes, daily_chapters, updated_at FROM reading_goals WHERE id = 1",
            [],
            |row| {
                Ok(ReadingGoalRecord {
                    id: row.get(0)?,
                    daily_minutes: row.get(1)?,
                    daily_chapters: row.get(2)?,
                    updated_at: row.get(3)?,
                })
            },
        )?;
        Ok(row)
    }

    pub fn update_reading_goal(&self, daily_minutes: i64, daily_chapters: i64) -> Result<()> {
        self.conn.execute(
            "UPDATE reading_goals SET daily_minutes = ?1, daily_chapters = ?2, updated_at = unixepoch() WHERE id = 1",
            params![daily_minutes, daily_chapters],
        )?;
        Ok(())
    }

    // ─── Crawl Tasks 辅助方法（plan Task 6 新增） ──────────────────────────

    /// 重置 failed/success 状态的任务为 pending（用于重试）
    pub fn reset_task_status(&self, website_book_id: i64) -> Result<u64> {
        let now = chrono::Utc::now().timestamp();
        let res = self.conn.execute(
            "UPDATE crawl_tasks
             SET status = 'pending', error_message = '', progress = 0,
                 started_at = NULL, finished_at = NULL, updated_at = ?2
             WHERE website_book_id = ?1 AND status IN ('failed', 'success')",
            params![website_book_id, now],
        )?;
        Ok(res as u64)
    }

    /// 按状态删除任务
    pub fn delete_tasks_by_status(&self, status: &str) -> Result<u64> {
        let res = self.conn.execute(
            "DELETE FROM crawl_tasks WHERE status = ?1",
            params![status],
        )?;
        Ok(res as u64)
    }

    /// 获取 id 大于 after_id 的日志（用于 SSE 增量推送）
    pub fn list_logs_after(&self, after_id: i64, limit: i64) -> Result<Vec<CrawlLogRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, level, message, created_at FROM crawl_logs WHERE id > ?1 ORDER BY id ASC LIMIT ?2",
        )?;
        let logs = stmt
            .query_map(params![after_id, limit], |row| {
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

#[derive(Debug, Clone, serde::Serialize, Default)]
pub struct CrawlTaskStatusCount {
    pub pending: i64,
    pub running: i64,
    pub success: i64,
    pub failed: i64,
    pub skipped: i64,
    pub total: i64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ReadingHistoryRow {
    pub book_id: i64,
    pub book_title: Option<String>,
    pub last_read_at: i64,
    pub last_chapter_order: i64,
    pub total_duration_sec: i64,
    pub chapters_read: i64,
}

fn map_crawl_task(row: &rusqlite::Row) -> rusqlite::Result<CrawlTaskRecord> {
    Ok(CrawlTaskRecord {
        id: row.get(0)?,
        website_book_id: row.get(1)?,
        book_id: row.get(2)?,
        title: row.get(3)?,
        status: row.get(4)?,
        progress: row.get(5)?,
        chapters_total: row.get(6)?,
        chapters_done: row.get(7)?,
        error_message: row.get(8)?,
        trigger: row.get(9)?,
        started_at: row.get(10)?,
        finished_at: row.get(11)?,
        created_at: row.get(12)?,
        updated_at: row.get(13)?,
    })
}

fn map_book_row(row: &rusqlite::Row) -> rusqlite::Result<BookRecord> {
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
}
