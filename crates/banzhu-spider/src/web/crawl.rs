use super::*;
use crate::web::ApiResponse;
use serde::Deserialize;

// ─── Crawl control ───────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub(crate) struct CrawlTriggerBody {
    #[allow(dead_code)]
    pub(crate) pages: Option<u32>,
}

pub(crate) async fn crawl_trigger(
    State(state): State<Arc<AppState>>,
    Json(_body): Json<CrawlTriggerBody>,
) -> Json<ApiResponse<Value>> {
    let scheduler = state.scheduler.clone();

    {
        let mut status = scheduler.status.lock().await;
        if status.running {
            return err_response("爬虫正在运行中");
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

    ok_response(json!({ "message": "爬取任务已触发" }))
}

pub(crate) async fn crawl_status(State(state): State<Arc<AppState>>) -> Json<ApiResponse<Value>> {
    let status = state.scheduler.status.lock().await;
    ok_response(serde_json::to_value(status.clone()).unwrap_or(json!({})))
}

pub(crate) async fn crawl_schedule(State(state): State<Arc<AppState>>) -> Json<ApiResponse<Value>> {
    let config = &state.scheduler.config();
    let schedule = config.get_string("cron.schedule").unwrap_or_default();
    let enabled = config.get_bool("cron.enabled").unwrap_or(true);
    let pages_limit = config.get_int("cron.pages_limit").unwrap_or(50);

    ok_response(json!({
        "schedule": schedule,
        "enabled": enabled,
        "pages_limit": pages_limit,
    }))
}

pub(crate) async fn update_crawl_schedule(
    State(_state): State<Arc<AppState>>,
    Json(body): Json<Value>,
) -> Json<ApiResponse<Value>> {
    // ponytail: 允许运行时修改但重启后丢失（spider.toml 不回写），需要持久化时再实现
    ok_response(json!({ "message": "Schedule updated (runtime only)", "updated": body }))
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
) -> Json<ApiResponse<Value>> {
    let book_id = match parse_book_id_from_url(&body.url) {
        Some(id) => id,
        None => return err_response("无法从 URL 解析出 book_id"),
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

    ok_response(json!({ "message": "爬取任务已提交", "book_id": book_id }))
}

// GET /api/crawl/logs?limit=100
pub(crate) async fn crawl_logs(
    State(state): State<Arc<AppState>>,
    Query(q): Query<LogsQuery>,
) -> Json<ApiResponse<Vec<crate::db::CrawlLogRecord>>> {
    let limit = q.limit.unwrap_or(100).clamp(1, 500);
    let db = state.db.lock().await;
    match db.get_crawl_logs(limit) {
        Ok(logs) => ApiResponse::ok(logs),
        Err(e) => ApiResponse::err(format!("查询失败: {}", e)),
    }
}
