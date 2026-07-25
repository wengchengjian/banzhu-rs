use banzhu_spider::db::{BookRecord, Database};

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
