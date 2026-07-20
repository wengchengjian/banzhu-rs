use super::*;
use crate::error::{AppError, AppResult};
use crate::web::ApiResponse;
use serde::Deserialize;

// ─── Crawl tasks query ──────────────────────────────────────────────────────

#[derive(Deserialize, Default)]
pub(crate) struct CrawlTasksQuery {
    pub status: Option<String>,
    pub page: Option<i64>,
    pub limit: Option<i64>,
}

// GET /api/crawl/tasks?status=&page=&limit=
pub(crate) async fn crawl_tasks(
    State(state): State<Arc<AppState>>,
    Query(q): Query<CrawlTasksQuery>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let page = q.page.unwrap_or(1).max(1);
    let limit = q.limit.unwrap_or(50).clamp(1, 200);
    let offset = (page - 1) * limit;

    let status = q.status.as_deref().filter(|s| !s.is_empty());

    let db = state.db.lock().await;
    let tasks = db.list_crawl_tasks(status, limit, offset)?;
    let stats = db.count_crawl_tasks_by_status().unwrap_or_default();

    let items: Vec<Value> = tasks
        .into_iter()
        .map(|t| {
            json!({
                "id": t.id,
                "website_book_id": t.website_book_id,
                "book_id": t.book_id,
                "title": t.title,
                "status": t.status,
                "progress": t.progress,
                "chapters_total": t.chapters_total,
                "chapters_done": t.chapters_done,
                "error_message": t.error_message,
                "trigger": t.trigger,
                "started_at": t.started_at,
                "finished_at": t.finished_at,
                "created_at": t.created_at,
                "updated_at": t.updated_at,
            })
        })
        .collect();

    Ok(ok_response(json!({
        "items": items,
        "total": items.len(),
        "page": page,
        "limit": limit,
        "status_count": {
            "pending": stats.pending,
            "running": stats.running,
            "success": stats.success,
            "failed": stats.failed,
            "skipped": stats.skipped,
            "total": stats.total,
        },
    })))
}

// POST /api/crawl/retry/:bookId  (bookId 为 website_book_id)
pub(crate) async fn crawl_retry(
    State(state): State<Arc<AppState>>,
    Path(website_book_id): Path<i64>,
) -> AppResult<Json<ApiResponse<Value>>> {
    if website_book_id <= 0 {
        return Err(AppError::BadRequest("无效的 book_id".into()));
    }

    let scheduler = state.scheduler.clone();

    // 异步执行，避免阻塞 API 响应
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new()
            .expect("Failed to create tokio runtime for retry");
        if let Err(e) = rt.block_on(scheduler.retry_book(website_book_id as u32)) {
            log::error!("Retry crawl failed for book_id={}: {}", website_book_id, e);
        }
    });

    Ok(ok_response(json!({
        "message": "重新爬取任务已提交",
        "book_id": website_book_id,
    })))
}

// ─── Crawl control ───────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub(crate) struct CrawlTriggerBody {
    #[allow(dead_code)]
    pub(crate) pages: Option<u32>,
}

pub(crate) async fn crawl_trigger(
    State(state): State<Arc<AppState>>,
    Json(_body): Json<CrawlTriggerBody>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let scheduler = state.scheduler.clone();

    {
        let mut status = scheduler.status.lock().await;
        if status.running {
            return Err(AppError::BadRequest("爬虫正在运行中".into()));
        }
        // 在持锁期间设置 running，防止并发请求同时通过检查
        status.running = true;
    }

    // spawn_blocking 因为 scraper::Html 不是 Send
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new()
            .expect("Failed to create tokio runtime for crawl");
        if let Err(e) = rt.block_on(scheduler.crawl_once()) {
            log::error!("Manual crawl failed: {}", e);
        }
    });

    Ok(ok_response(json!({ "message": "爬取任务已触发" })))
}

pub(crate) async fn crawl_status(State(state): State<Arc<AppState>>) -> AppResult<Json<ApiResponse<Value>>> {
    let status = state.scheduler.status.lock().await;
    Ok(ok_response(serde_json::to_value(status.clone()).unwrap_or(json!({}))))
}

pub(crate) async fn crawl_schedule(State(state): State<Arc<AppState>>) -> AppResult<Json<ApiResponse<Value>>> {
    let config = &state.scheduler.config();
    let schedule = config.get_string("cron.schedule").unwrap_or_default();
    let enabled = config.get_bool("cron.enabled").unwrap_or(true);
    let pages_limit = config.get_int("cron.pages_limit").unwrap_or(50);

    Ok(ok_response(json!({
        "schedule": schedule,
        "enabled": enabled,
        "pages_limit": pages_limit,
    })))
}

pub(crate) async fn update_crawl_schedule(
    State(_state): State<Arc<AppState>>,
    Json(body): Json<Value>,
) -> AppResult<Json<ApiResponse<Value>>> {
    // ponytail: 允许运行时修改但重启后丢失（spider.toml 不回写），需要持久化时再实现
    Ok(ok_response(json!({ "message": "Schedule updated (runtime only)", "updated": body })))
}

// ─── Manual crawl & logs ─────────────────────────────────────────────────────

#[derive(Deserialize)]
pub(crate) struct ManualCrawlBody {
    pub url: String,
}

#[derive(Deserialize, Default)]
pub(crate) struct LogsQuery {
    pub limit: Option<i64>,
}

/// 从书籍目录 URL 中解析网站 book_id。
/// 支持形如 "https://site.com/52/52024/" 的链接，取最后一段数字（52024）。
fn parse_book_id_from_url(url: &str) -> Option<u32> {
    url.trim()
        .trim_end_matches('/')
        .rsplit('/')
        .find_map(|seg| seg.parse::<u32>().ok())
}

// POST /api/crawl/manual  {url: "https://.../52/52024/"}
pub(crate) async fn crawl_manual(
    State(state): State<Arc<AppState>>,
    Json(body): Json<ManualCrawlBody>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let book_id = match parse_book_id_from_url(&body.url) {
        Some(id) => id,
        None => return Err(AppError::BadRequest("无法从 URL 解析出 book_id".into())),
    };

    let scheduler = state.scheduler.clone();

    // spawn 独立线程 + runtime（scraper::Html 非 Send，沿用 crawl_trigger 的模式）
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new()
            .expect("Failed to create tokio runtime for manual crawl");
        if let Err(e) = rt.block_on(scheduler.crawl_book(book_id)) {
            log::error!("Manual crawl failed for book_id={}: {}", book_id, e);
        }
    });

    Ok(ok_response(json!({ "message": "爬取任务已提交", "book_id": book_id })))
}

// GET /api/crawl/logs?limit=100
pub(crate) async fn crawl_logs(
    State(state): State<Arc<AppState>>,
    Query(q): Query<LogsQuery>,
) -> AppResult<Json<ApiResponse<Vec<crate::db::CrawlLogRecord>>>> {
    let limit = q.limit.unwrap_or(100).clamp(1, 500);
    let db = state.db.lock().await;
    let logs = db.get_crawl_logs(limit)?;
    Ok(ApiResponse::ok(logs))
}
