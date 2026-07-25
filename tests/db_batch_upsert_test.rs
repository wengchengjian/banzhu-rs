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
}
