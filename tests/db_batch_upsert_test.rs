use banzhu_spider::db::{BookRecord, ChapterRecord, Database, SectionRecord};

fn make_book(website_id: i64, title: &str) -> BookRecord {
    BookRecord {
        id: 0,
        website_book_id: Some(website_id),
        path_num: 12,
        title: title.to_string(),
        filename: format!("{}.txt", title),
        author: "作者".to_string(),
        category: "分类".to_string(),
        introduce: "简介".to_string(),
        likes: 100,
        word_count: 10000,
        page_count: 10,
        created_at: 0,
        updated_at: 0,
    }
}

#[test]
fn test_batch_upsert_books_inserts_new() {
    let db = Database::open_in_memory().unwrap();
    let books = vec![make_book(1001, "书1"), make_book(1002, "书2")];
    let n = db.batch_upsert_books(&books).unwrap();
    assert_eq!(n, 2);
    assert!(db.book_exists_by_website_id(1001).unwrap());
    assert!(db.book_exists_by_website_id(1002).unwrap());
}

#[test]
fn test_batch_upsert_books_replaces_existing() {
    let db = Database::open_in_memory().unwrap();
    db.batch_upsert_books(&[make_book(1001, "原标题")]).unwrap();
    // 同 website_book_id，不同 title
    db.batch_upsert_books(&[make_book(1001, "新标题")]).unwrap();
    let book = db.get_book_by_website_id(1001).unwrap().unwrap();
    assert_eq!(book.title, "新标题");
    assert_eq!(book.website_book_id, Some(1001));
}

#[test]
fn test_batch_upsert_chapters_via_website_id_join() {
    let db = Database::open_in_memory().unwrap();
    db.batch_upsert_books(&[make_book(1001, "书1")]).unwrap();

    let chapters = vec![
        (1001_i64, ChapterRecord {
            id: 0, book_id: 0, title: "第1章".to_string(),
            url: "https://x/1".to_string(), chapter_order: 1, word_count: 0,
        }),
        (1001_i64, ChapterRecord {
            id: 0, book_id: 0, title: "第2章".to_string(),
            url: "https://x/2".to_string(), chapter_order: 2, word_count: 0,
        }),
    ];
    let n = db.batch_upsert_chapters(&chapters).unwrap();
    assert_eq!(n, 2);

    // 验证 DB 实际状态：JOIN 是否正确解析 book_id、字段是否匹配输入
    let book = db.get_book_by_website_id(1001).unwrap().expect("book 应存在");
    let book_id = book.id;
    assert!(book_id > 0, "book_id 必须为正数（证明 JOIN 命中）");

    let rows = db.get_chapters_by_book(book_id).unwrap();
    assert_eq!(rows.len(), 2, "应插入 2 条章节");

    // get_chapters_by_book 按 chapter_order ASC 排序
    assert_eq!(rows[0].book_id, book_id, "JOIN 写入的 book_id 必须正确");
    assert_eq!(rows[0].title, "第1章");
    assert_eq!(rows[0].url, "https://x/1");
    assert_eq!(rows[0].chapter_order, 1);

    assert_eq!(rows[1].book_id, book_id);
    assert_eq!(rows[1].title, "第2章");
    assert_eq!(rows[1].url, "https://x/2");
    assert_eq!(rows[1].chapter_order, 2);
}

#[test]
fn test_batch_upsert_chapters_replaces_existing() {
    let db = Database::open_in_memory().unwrap();
    db.batch_upsert_books(&[make_book(1001, "书1")]).unwrap();

    // 第一次：插入 chapter_order=1, title="旧标题"
    db.batch_upsert_chapters(&[(1001, ChapterRecord {
        id: 0, book_id: 0, title: "旧标题".to_string(),
        url: "https://x/old".to_string(), chapter_order: 1, word_count: 0,
    })]).unwrap();

    // 第二次：相同 website_book_id + chapter_order，title 改为 "新标题"
    let n = db.batch_upsert_chapters(&[(1001, ChapterRecord {
        id: 0, book_id: 0, title: "新标题".to_string(),
        url: "https://x/new".to_string(), chapter_order: 1, word_count: 0,
    })]).unwrap();
    assert_eq!(n, 1, "返回值=输入切片长度");

    // 验证 REPLACE 语义：仍只有 1 条章节，标题被覆盖
    let book = db.get_book_by_website_id(1001).unwrap().unwrap();
    let rows = db.get_chapters_by_book(book.id).unwrap();
    assert_eq!(rows.len(), 1, "REPLACE 应替换而非追加");
    assert_eq!(rows[0].title, "新标题", "标题应被新值覆盖");
    assert_eq!(rows[0].url, "https://x/new");
    assert_eq!(rows[0].chapter_order, 1);
}

#[test]
fn test_batch_upsert_sections_via_join() {
    let db = Database::open_in_memory().unwrap();
    db.batch_upsert_books(&[make_book(1001, "书1")]).unwrap();
    db.batch_upsert_chapters(&[
        (1001, ChapterRecord { id: 0, book_id: 0, title: "第1章".into(), url: "u".into(), chapter_order: 1, word_count: 0 }),
    ]).unwrap();

    let sections = vec![
        (1001_i64, 1_i64, SectionRecord {
            id: 0, chapter_id: 0, book_id: 0, url: "https://x/s1".into(),
            content: "内容1".into(), section_order: 1,
        }),
    ];
    let n = db.batch_upsert_sections(&sections).unwrap();
    assert_eq!(n, 1);

    // 验证 DB 实际状态：JOIN 是否正确解析 book_id 与 chapter_id、字段是否匹配输入
    let book = db.get_book_by_website_id(1001).unwrap().expect("book 应存在");
    let book_id = book.id;
    assert!(book_id > 0, "book_id 必须为正数（证明 JOIN 命中 books）");

    let chapter = db
        .get_chapter_by_book_and_order(book_id, 1)
        .unwrap()
        .expect("chapter 应存在");
    let chapter_id = chapter.id;
    assert!(chapter_id > 0, "chapter_id 必须为正数（证明 JOIN 命中 chapters）");

    let rows = db.get_sections_by_chapter(chapter_id).unwrap();
    assert_eq!(rows.len(), 1, "应插入 1 条 section");

    let s = &rows[0];
    assert_eq!(s.chapter_id, chapter_id, "JOIN 写入的 chapter_id 必须正确");
    assert_eq!(s.book_id, book_id, "JOIN 写入的 book_id 必须正确");
    assert_eq!(s.url, "https://x/s1");
    assert_eq!(s.content, "内容1");
    assert_eq!(s.section_order, 1);
}
