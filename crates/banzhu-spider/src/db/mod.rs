//! SQLite persistence layer for the banzhu spider.
//!
//! The [`Database`] type lives here together with connection setup
//! (`open`, `open_in_memory`, `init_tables`, `init_fts`). CRUD operations are
//! implemented in [`crud`], full-text-search helpers in [`fts`], the SQL schema
//! constants in [`schema`], and the row record structs in [`models`].

mod crud;
mod fts;
mod models;
mod schema;

pub use models::*;

use anyhow::Result;
use rusqlite::Connection;
use schema::CREATE_FTS_TABLE;

const DB_NAME: &str = "banzhu.db";

pub struct Database {
    pub(crate) conn: Connection,
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
        self.conn.busy_timeout(std::time::Duration::from_secs(5))?;
        self.conn.execute(schema::CREATE_BOOKS_TABLE, [])?;
        self.conn.execute(schema::CREATE_CHAPTERS_TABLE, [])?;
        self.conn.execute(schema::CREATE_SECTIONS_TABLE, [])?;
        self.conn.execute(schema::CREATE_BOOKSHELF_TABLE, [])?;
        self.conn.execute(schema::CREATE_READING_PROGRESS_TABLE, [])?;
        self.conn.execute(schema::CREATE_CRAWL_LOGS_TABLE, [])?;
        self.conn.execute_batch(schema::CREATE_INDEX)?;
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::SearchField;

    fn create_test_db() -> Database {
        Database::open_in_memory().unwrap()
    }

    fn insert_test_book(db: &Database, title: &str, author: &str, content: &str) -> i64 {
        let book = BookRecord {
            id: 0,
            website_book_id: None,
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
            website_book_id: None,
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
            website_book_id: None,
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
            website_book_id: None,
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
            website_book_id: None,
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
            website_book_id: None,
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
            website_book_id: None,
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
            website_book_id: None,
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
            website_book_id: None,
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
            website_book_id: None,
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
