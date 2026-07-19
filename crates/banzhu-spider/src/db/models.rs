//! Data record structs that map to database rows.
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct BookRecord {
    pub id: i64,
    pub website_book_id: Option<i64>,
    pub path_num: i64,
    pub title: String,
    pub filename: String,
    pub author: String,
    pub category: String,
    pub introduce: String,
    pub likes: i64,
    pub word_count: i64,
    pub page_count: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChapterRecord {
    pub id: i64,
    pub book_id: i64,
    pub title: String,
    pub url: String,
    pub chapter_order: i64,
    pub word_count: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct SectionRecord {
    pub id: i64,
    pub chapter_id: i64,
    pub book_id: i64,
    pub url: String,
    pub content: String,
    pub section_order: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct BookshelfRecord {
    pub id: i64,
    pub book_id: i64,
    pub group_name: String,
    pub added_at: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReadingProgressRecord {
    pub id: i64,
    pub book_id: i64,
    pub chapter_order: i64,
    pub page_index: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct CrawlLogRecord {
    pub id: i64,
    pub level: String,
    pub message: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct SearchRecord {
    pub id: i64,
    pub title: String,
    pub author: String,
    pub category: String,
    pub word_count: i64,
    pub chapter_count: i64,
    pub created_at: i64,
}
