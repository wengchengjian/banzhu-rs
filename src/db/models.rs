//! Data record structs that map to database rows.
use serde::Serialize;
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../frontend/src/types/api/")]
pub struct BookRecord {
    #[ts(type = "number")]
    pub id: i64,
    #[ts(type = "number | null")]
    pub website_book_id: Option<i64>,
    #[ts(type = "number")]
    pub path_num: i64,
    pub title: String,
    pub filename: String,
    pub author: String,
    pub category: String,
    pub introduce: String,
    #[ts(type = "number")]
    pub likes: i64,
    #[ts(type = "number")]
    pub word_count: i64,
    #[ts(type = "number")]
    pub page_count: i64,
    #[ts(type = "number")]
    pub created_at: i64,
    #[ts(type = "number")]
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../frontend/src/types/api/")]
pub struct ChapterRecord {
    #[ts(type = "number")]
    pub id: i64,
    #[ts(type = "number")]
    pub book_id: i64,
    pub title: String,
    pub url: String,
    #[ts(type = "number")]
    pub chapter_order: i64,
    #[ts(type = "number")]
    pub word_count: i64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../frontend/src/types/api/")]
pub struct SectionRecord {
    #[ts(type = "number")]
    pub id: i64,
    #[ts(type = "number")]
    pub chapter_id: i64,
    #[ts(type = "number")]
    pub book_id: i64,
    pub url: String,
    pub content: String,
    #[ts(type = "number")]
    pub section_order: i64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../frontend/src/types/api/")]
pub struct BookshelfRecord {
    #[ts(type = "number")]
    pub id: i64,
    #[ts(type = "number")]
    pub book_id: i64,
    pub group_name: String,
    #[ts(type = "number")]
    pub added_at: i64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../frontend/src/types/api/")]
pub struct ReadingProgressRecord {
    #[ts(type = "number")]
    pub id: i64,
    #[ts(type = "number")]
    pub book_id: i64,
    #[ts(type = "number")]
    pub chapter_order: i64,
    #[ts(type = "number")]
    pub page_index: i64,
    #[ts(type = "number")]
    pub last_read_at: i64,
    #[ts(type = "number")]
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../frontend/src/types/api/")]
pub struct CrawlLogRecord {
    #[ts(type = "number")]
    pub id: i64,
    pub level: String,
    pub message: String,
    #[ts(type = "number")]
    pub created_at: i64,
}

/// 单本书的爬取任务状态记录
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../frontend/src/types/api/")]
pub struct CrawlTaskRecord {
    #[ts(type = "number")]
    pub id: i64,
    #[ts(type = "number")]
    pub website_book_id: i64,
    #[ts(type = "number | null")]
    pub book_id: Option<i64>,
    pub title: String,
    pub status: String,
    #[ts(type = "number")]
    pub progress: i64,
    #[ts(type = "number")]
    pub chapters_total: i64,
    #[ts(type = "number")]
    pub chapters_done: i64,
    pub error_message: String,
    pub trigger: String,
    #[ts(type = "number | null")]
    pub started_at: Option<i64>,
    #[ts(type = "number | null")]
    pub finished_at: Option<i64>,
    #[ts(type = "number")]
    pub created_at: i64,
    #[ts(type = "number")]
    pub updated_at: i64,
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

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../frontend/src/types/api/")]
#[serde(rename_all = "snake_case")]
pub struct ReadingSessionRecord {
    #[ts(type = "number")]
    pub id: i64,
    #[ts(type = "number")]
    pub book_id: i64,
    #[ts(type = "number")]
    pub chapter_order: i64,
    #[ts(type = "number")]
    pub duration_sec: i64,
    #[ts(type = "number")]
    pub chapters_read: i64,
    #[ts(type = "number")]
    pub started_at: i64,
    #[ts(type = "number")]
    pub ended_at: i64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../frontend/src/types/api/")]
#[serde(rename_all = "snake_case")]
pub struct ReadingGoalRecord {
    #[ts(type = "number")]
    pub id: i64,
    #[ts(type = "number")]
    pub daily_minutes: i64,
    #[ts(type = "number")]
    pub daily_chapters: i64,
    #[ts(type = "number")]
    pub updated_at: i64,
}
