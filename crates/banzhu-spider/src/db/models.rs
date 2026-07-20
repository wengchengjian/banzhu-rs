//! Data record structs that map to database rows.
use serde::Serialize;
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../frontend/src/types/api/")]
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

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../frontend/src/types/api/")]
pub struct ChapterRecord {
    pub id: i64,
    pub book_id: i64,
    pub title: String,
    pub url: String,
    pub chapter_order: i64,
    pub word_count: i64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../frontend/src/types/api/")]
pub struct SectionRecord {
    pub id: i64,
    pub chapter_id: i64,
    pub book_id: i64,
    pub url: String,
    pub content: String,
    pub section_order: i64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../frontend/src/types/api/")]
pub struct BookshelfRecord {
    pub id: i64,
    pub book_id: i64,
    pub group_name: String,
    pub added_at: i64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../frontend/src/types/api/")]
pub struct ReadingProgressRecord {
    pub id: i64,
    pub book_id: i64,
    pub chapter_order: i64,
    pub page_index: i64,
    pub last_read_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../frontend/src/types/api/")]
pub struct CrawlLogRecord {
    pub id: i64,
    pub level: String,
    pub message: String,
    pub created_at: i64,
}

/// 单本书的爬取任务状态记录
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../frontend/src/types/api/")]
pub struct CrawlTaskRecord {
    pub id: i64,
    pub website_book_id: i64,
    pub book_id: Option<i64>,
    pub title: String,
    pub status: String,
    pub progress: i64,
    pub chapters_total: i64,
    pub chapters_done: i64,
    pub error_message: String,
    pub trigger: String,
    pub started_at: Option<i64>,
    pub finished_at: Option<i64>,
    pub created_at: i64,
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
    pub id: i64,
    pub book_id: i64,
    pub chapter_order: i64,
    pub duration_sec: i64,
    pub chapters_read: i64,
    pub started_at: i64,
    pub ended_at: i64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../frontend/src/types/api/")]
#[serde(rename_all = "snake_case")]
pub struct ReadingGoalRecord {
    pub id: i64,
    pub daily_minutes: i64,
    pub daily_chapters: i64,
    pub updated_at: i64,
}
