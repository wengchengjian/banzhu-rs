use banzhu_spider::spider::pipeline::partition_items;
use serde_json::json;

#[test]
fn test_partition_items_separates_by_type() {
    let items = vec![
        json!({
            "type": "book", "website_book_id": 1001, "path_num": 12,
            "title": "书1", "filename": "书1.txt", "author": "作者",
            "category": "分类", "introduce": "简介", "likes": 100,
            "word_count": 10000, "page_count": 10
        }),
        json!({
            "type": "chapter", "website_book_id": 1001,
            "title": "第1章", "url": "https://x/1", "chapter_order": 1
        }),
        json!({
            "type": "section", "website_book_id": 1001, "chapter_order": 1,
            "section_order": 1, "url": "https://x/s1", "content": "内容"
        }),
        json!({"type": "unknown"}),
    ];

    let (books, chapters, sections) = partition_items(items);
    assert_eq!(books.len(), 1);
    assert_eq!(books[0].website_book_id, Some(1001));
    assert_eq!(chapters.len(), 1);
    assert_eq!(chapters[0].0, 1001);
    assert_eq!(chapters[0].1.chapter_order, 1);
    assert_eq!(sections.len(), 1);
    assert_eq!(sections[0].0, 1001);
    assert_eq!(sections[0].1, 1);
    assert_eq!(sections[0].2.section_order, 1);
}
