//! BanzhuDbPipeline：基于 wisp `AsyncBatchItemPipeline` 的异步 DB 写入管道。
//!
//! 主循环 `process_item` 只做缓冲 + 推入 channel（无 DB IO）；后台单任务
//! 消费 channel 批量写库。DB 写慢不再占用爬虫并发槽，爬取速度不降。

use crate::db::{BookRecord, ChapterRecord, Database, SectionRecord};
use crate::event::{CrawlEvent, EventBus};
use crate::scheduler::CrawlStatus;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;
use wisp::crawl::middleware::AsyncBatchItemPipeline;

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
    let mut unmatched = 0usize;
    for item in items {
        let Some(typ) = item.get("type").and_then(|t| t.as_str()) else {
            unmatched += 1;
            continue;
        };
        match typ {
            "book" => {
                let b = BookRecord {
                    id: 0,
                    website_book_id: item
                        .get("website_book_id")
                        .and_then(|v| v.as_i64()),
                    path_num: item
                        .get("path_num")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0),
                    title: item
                        .get("title")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string(),
                    filename: item
                        .get("filename")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string(),
                    author: item
                        .get("author")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string(),
                    category: item
                        .get("category")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string(),
                    introduce: item
                        .get("introduce")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
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
                books.push(b);
            }
            "chapter" => {
                let c = ChapterRecord {
                    id: 0,
                    book_id: 0,
                    title: item
                        .get("title")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string(),
                    url: item
                        .get("url")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string(),
                    chapter_order: item
                        .get("chapter_order")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0),
                    word_count: 0,
                };
                let wid = item
                    .get("website_book_id")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                chapters.push((wid, c));
            }
            "section" => {
                let s = SectionRecord {
                    id: 0,
                    chapter_id: 0,
                    book_id: 0,
                    url: item
                        .get("url")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string(),
                    content: item
                        .get("content")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string(),
                    section_order: item
                        .get("section_order")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0),
                };
                let wid = item
                    .get("website_book_id")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                let cid = item
                    .get("chapter_id")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                sections.push((wid, cid, s));
            }
            _ => unmatched += 1,
        }
    }
    if unmatched > 0 {
        log::warn!("[pipeline] partition: 共 {} 条 items 未匹配任何 type", unmatched);
    }
    (books, chapters, sections)
}

/// 构造异步 DB 写入 pipeline（batch=50）。
///
/// flush 闭包在 wisp 后台任务中执行，主循环只缓冲 + 推 channel，DB 写不阻塞爬取。
pub fn build_banzhu_pipeline(
    db: Arc<Mutex<Database>>,
    event_bus: EventBus,
    status: Arc<Mutex<CrawlStatus>>,
) -> AsyncBatchItemPipeline {
    log::info!("[pipeline] build_banzhu_pipeline: 创建 AsyncBatchItemPipeline (batch_size=50)");
    AsyncBatchItemPipeline::new(50, move |items| {
        let db = db.clone();
        let event_bus = event_bus.clone();
        let status = status.clone();
        async move {
            // wisp 新 API：flush 闭包收到 Vec<Item<Value>>，先取出 payload 再按 type 分流
            let items: Vec<Value> = items.into_iter().map(|i| i.into_value()).collect();
            let total = items.len();
            log::info!("[pipeline] flush 触发: 收到 {} 条 items", total);

            let (books, chapters, sections) = partition_items(items);
            log::info!(
                "[pipeline] partition 结果: books={}, chapters={}, sections={}",
                books.len(),
                chapters.len(),
                sections.len()
            );

            if books.is_empty() && chapters.is_empty() && sections.is_empty() {
                log::warn!("[pipeline] flush 中 {} 条 items 全部未匹配任何 type，已丢弃", total);
                return Ok(());
            }

            let mut failed_website_ids: Vec<i64> = vec![];

            let db_guard = db.lock().await;
            if !books.is_empty() {
                log::debug!("[pipeline] 写入 books: {} 条", books.len());
                match db_guard.batch_upsert_books(&books) {
                    Ok(n) => {
                        log::info!("[pipeline] batch_upsert_books 成功: 写入 {} 条", n);
                        status.lock().await.books_downloaded += n as u32;
                    }
                    Err(e) => {
                        log::error!("[pipeline] batch_upsert_books 失败: {e}");
                        status.lock().await.books_failed += books.len() as u32;
                        failed_website_ids.extend(books.iter().filter_map(|b| b.website_book_id));
                    }
                }
            }
            if !chapters.is_empty() {
                log::debug!("[pipeline] 写入 chapters: {} 条", chapters.len());
                match db_guard.batch_upsert_chapters(&chapters) {
                    Ok(n) => log::info!("[pipeline] batch_upsert_chapters 成功: 处理 {} 条", n),
                    Err(e) => {
                        log::error!("[pipeline] batch_upsert_chapters 失败: {e}");
                        status.lock().await.books_failed += chapters.len() as u32;
                        failed_website_ids.extend(chapters.iter().map(|(wid, _)| *wid));
                    }
                }
            }
            if !sections.is_empty() {
                log::debug!("[pipeline] 写入 sections: {} 条", sections.len());
                match db_guard.batch_upsert_sections(&sections) {
                    Ok(n) => log::info!("[pipeline] batch_upsert_sections 成功: 处理 {} 条", n),
                    Err(e) => {
                        log::error!("[pipeline] batch_upsert_sections 失败: {e}");
                        status.lock().await.books_failed += sections.len() as u32;
                        failed_website_ids.extend(sections.iter().map(|(wid, _, _)| *wid));
                    }
                }
            }

            if !failed_website_ids.is_empty() {
                log::warn!(
                    "[pipeline] {} 个 website_book_id 标记为失败: {:?}",
                    failed_website_ids.len(),
                    &failed_website_ids[..failed_website_ids.len().min(5)]
                );
            }

            for wid in &failed_website_ids {
                let _ = db_guard.mark_crawl_task_failed(*wid, "batch upsert failed");
            }
            drop(db_guard);

            let s = status.lock().await.clone();
            event_bus.emit(CrawlEvent::Status {
                running: s.running,
                current_page: s.current_page as i64,
                pages_limit: s.pages_limit as i64,
                books_found: s.books_found as i64,
                books_downloaded: s.books_downloaded as i64,
                books_failed: s.books_failed as i64,
                books_skipped: s.books_skipped as i64,
                message: s.message.clone(),
            });
            log::info!(
                "[pipeline] flush 完成: downloaded={}, failed={}",
                s.books_downloaded,
                s.books_failed
            );
            Ok(())
        }
    })
}
