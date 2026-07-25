//! BanzhuDbPipeline：基于 wisp BatchItemPipeline，按 type 分流写 DB。

use crate::db::{BookRecord, ChapterRecord, Database, SectionRecord};
use crate::event::{CrawlEvent, EventBus};
use crate::scheduler::CrawlStatus;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;
use wisp::crawl::middleware::BatchItemPipeline;

/// 按 item["type"] 分流到 (books, chapters, sections)。
///
/// 直接从 JSON 字段手动构造 *Record，不使用 `serde_json::from_value`：
/// *Record 没有 `Deserialize` derive（仅 `Serialize`），且 callback 上下文携带
/// 的字段集与 struct 字段集不完全对应（id/book_id/chapter_id 由 JOIN 解析）。
/// id/book_id/chapter_id/word_count/created_at/updated_at 等不写入 DB 或由 DB
/// 自动填充的字段统一置 0，最终通过 `batch_upsert_*` 的 JOIN 解析。
pub fn partition_items(
    items: Vec<Value>,
) -> (
    Vec<BookRecord>,
    Vec<(i64, ChapterRecord)>,
    Vec<(i64, i64, SectionRecord)>,
) {
    let mut books = vec![];
    let mut chapters = vec![];
    let mut sections = vec![];

    for item in items {
        match item.get("type").and_then(|v| v.as_str()) {
            Some("book") => {
                let book = BookRecord {
                    id: 0,
                    website_book_id: item.get("website_book_id").and_then(|v| v.as_i64()),
                    path_num: item.get("path_num").and_then(|v| v.as_i64()).unwrap_or(0),
                    title: item
                        .get("title")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    filename: item
                        .get("filename")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    author: item
                        .get("author")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    category: item
                        .get("category")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    introduce: item
                        .get("introduce")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    likes: item.get("likes").and_then(|v| v.as_i64()).unwrap_or(0),
                    word_count: item
                        .get("word_count")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0),
                    page_count: item
                        .get("page_count")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0),
                    created_at: 0,
                    updated_at: 0,
                };
                books.push(book);
            }
            Some("chapter") => {
                let website_book_id = item
                    .get("website_book_id")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                let chapter = ChapterRecord {
                    id: 0,
                    book_id: 0,
                    title: item
                        .get("title")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    url: item
                        .get("url")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    chapter_order: item
                        .get("chapter_order")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0),
                    word_count: 0,
                };
                chapters.push((website_book_id, chapter));
            }
            Some("section") => {
                let website_book_id = item
                    .get("website_book_id")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                let chapter_order = item
                    .get("chapter_order")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                let section = SectionRecord {
                    id: 0,
                    chapter_id: 0,
                    book_id: 0,
                    url: item
                        .get("url")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    content: item
                        .get("content")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    section_order: item
                        .get("section_order")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0),
                };
                sections.push((website_book_id, chapter_order, section));
            }
            _ => {}
        }
    }
    (books, chapters, sections)
}

/// 构造写 DB 的 BatchItemPipeline（batch=50）。
///
/// 闭包捕获 `Arc<Mutex<Database>>` / `EventBus` / `Arc<Mutex<CrawlStatus>>`，
/// 满足 `Send + Sync + 'static` 约束；每次 flush 内部 clone 这三份 handle。
pub fn build_banzhu_pipeline(
    db: Arc<Mutex<Database>>,
    event_bus: EventBus,
    status: Arc<Mutex<CrawlStatus>>,
) -> BatchItemPipeline {
    BatchItemPipeline::new(50, move |items| {
        let db = db.clone();
        let event_bus = event_bus.clone();
        let status = status.clone();
        async move {
            let (books, chapters, sections) = partition_items(items);

            let mut failed_website_ids: Vec<i64> = vec![];

            let db_guard = db.lock().await;
            if !books.is_empty() {
                match db_guard.batch_upsert_books(&books) {
                    Ok(n) => {
                        status.lock().await.books_downloaded += n as u32;
                    }
                    Err(e) => {
                        log::error!("batch_upsert_books failed: {e}");
                        failed_website_ids.extend(books.iter().filter_map(|b| b.website_book_id));
                    }
                }
            }
            if !chapters.is_empty() {
                if let Err(e) = db_guard.batch_upsert_chapters(&chapters) {
                    log::error!("batch_upsert_chapters failed: {e}");
                    failed_website_ids.extend(chapters.iter().map(|(wid, _)| *wid));
                }
            }
            if !sections.is_empty() {
                if let Err(e) = db_guard.batch_upsert_sections(&sections) {
                    log::error!("batch_upsert_sections failed: {e}");
                    failed_website_ids.extend(sections.iter().map(|(wid, _, _)| *wid));
                }
            }

            for wid in &failed_website_ids {
                let _ = db_guard.mark_crawl_task_failed(*wid, "batch upsert failed");
            }
            drop(db_guard);

            let s = status.lock().await.clone();
            event_bus.emit(CrawlEvent::Status {
                running: true,
                current_page: s.current_page as i64,
                pages_limit: s.pages_limit as i64,
                books_found: s.books_found as i64,
                books_downloaded: s.books_downloaded as i64,
                books_failed: s.books_failed as i64,
                books_skipped: s.books_skipped as i64,
                message: s.message.clone(),
            });
        }
    })
}
