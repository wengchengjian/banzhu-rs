use crate::appconfig;
use crate::db::Database;
use crate::scheduler::Scheduler;
use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{header, Request, StatusCode, Uri},
    middleware::{self, Next},
    response::{IntoResponse, Json, Response},
    routing::{get, post, put},
    Router,
};
use rust_embed::RustEmbed;
use config::Config;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;

mod books;
mod crawl;
mod export;
mod search;
mod shelf;
mod stats;

use books::{book_chapters, book_detail, categories, chapter_content, delete_book, list_books, stats as books_stats};
use crawl::{
    crawl_logs, crawl_manual, crawl_schedule, crawl_status, crawl_stream, crawl_tasks,
    crawl_trigger, delete_tasks, retry_failed, update_crawl_schedule,
};
use export::export_book;
use search::search;
use shelf::{
    add_to_bookshelf, get_bookshelf, get_progress, remove_from_bookshelf, update_progress,
    update_shelf_group,
};

// ─── 统一响应类型 ─────────────────────────────────────────────────────────────

#[derive(Serialize)]
pub(crate) struct ApiResponse<T: Serialize> {
    pub code: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub msg: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn ok(data: T) -> Json<Self> {
        Json(Self { code: 0, msg: None, data: Some(data) })
    }

    pub fn err(msg: impl Into<String>) -> Json<Self> {
        Json(Self { code: -1, msg: Some(msg.into()), data: None })
    }
}

/// 向后兼容：接受 Value 的快捷方式
pub(crate) fn ok_response(data: Value) -> Json<ApiResponse<Value>> {
    ApiResponse::ok(data)
}

// ─── 请求日志中间件 ───────────────────────────────────────────────────────────

async fn log_request(req: Request<Body>, next: Next) -> Response {
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    let start = Instant::now();

    let response = next.run(req).await;

    let status = response.status();
    let elapsed = start.elapsed();
    let level = if status.is_success() { "INFO" } else { "WARN" };

    log::log!(
        target: "api",
        match level { "WARN" => log::Level::Warn, _ => log::Level::Info },
        "{} {} {} {:.1}ms",
        method,
        path,
        status.as_u16(),
        elapsed.as_secs_f64() * 1000.0
    );

    response
}

// ─── Shared state ────────────────────────────────────────────────────────────

pub struct AppState {
    pub db: Arc<Mutex<Database>>,
    pub scheduler: Arc<Scheduler>,
    pub event_bus: crate::event::EventBus,
}

// ─── Query params ────────────────────────────────────────────────────────────

#[derive(Deserialize, Default)]
pub(crate) struct BooksQuery {
    pub(crate) page: Option<i64>,
    pub(crate) limit: Option<i64>,
    pub(crate) category: Option<String>,
}

#[derive(Deserialize, Default)]
pub(crate) struct SearchQuery {
    pub(crate) q: Option<String>,
    pub(crate) field: Option<String>,
    pub(crate) page: Option<i64>,
    pub(crate) limit: Option<i64>,
    pub(crate) exact: Option<bool>,
}

// ─── 前端静态资源（rust-embed） ───────────────────────────────────────────────

#[derive(RustEmbed)]
#[folder = "frontend/dist/"]
struct FrontendAsset;

pub async fn static_handler(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };

    if let Some(file) = FrontendAsset::get(path) {
        return file_response(path, file);
    }
    // SPA fallback
    if let Some(file) = FrontendAsset::get("index.html") {
        return file_response("index.html", file);
    }
    (StatusCode::NOT_FOUND, "Not Found").into_response()
}

fn file_response(path: &str, file: rust_embed::EmbeddedFile) -> Response {
    let mime = mime_guess::from_path(path).first_or_octet_stream();
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, mime.as_ref())],
        Body::from(file.data.into_owned()),
    ).into_response()
}

// ─── Startup ─────────────────────────────────────────────────────────────────

/// 构造 API Router（供 run_web 和集成测试复用）
pub fn build_router(state: Arc<AppState>) -> Router {
    Router::new()
        // 书籍
        .route("/api/books", get(list_books))
        .route("/api/books/{id}", get(book_detail).delete(delete_book))
        .route("/api/books/{id}/chapters", get(book_chapters))
        .route("/api/books/{id}/chapters/{order}", get(chapter_content))
        // 搜索
        .route("/api/search", get(search))
        // 分类 & 统计
        .route("/api/categories", get(categories))
        .route("/api/stats", get(books_stats))
        // 阅读统计
        .route("/api/stats/heatmap", get(stats::heatmap))
        .route("/api/stats/reading-timeline", get(stats::reading_timeline))
        .route("/api/stats/reading-session", post(stats::report_session))
        .route("/api/stats/reading-goal", get(stats::get_reading_goal).put(stats::update_reading_goal))
        .route("/api/stats/today", get(stats::today_reading))
        .route("/api/stats/reading-history", get(stats::reading_history))
        // 书架 & 阅读进度
        .route("/api/bookshelf", get(get_bookshelf).post(add_to_bookshelf))
        .route(
            "/api/bookshelf/{bookId}",
            put(update_shelf_group).delete(remove_from_bookshelf),
        )
        .route("/api/progress/{bookId}", get(get_progress).put(update_progress))
        // 导出
        .route("/api/export/{bookId}", get(export_book))
        // 爬虫控制
        .route("/api/crawl/trigger", post(crawl_trigger))
        .route("/api/crawl/status", get(crawl_status))
        .route("/api/crawl/schedule", get(crawl_schedule).put(update_crawl_schedule))
        .route("/api/crawl/manual", post(crawl_manual))
        .route("/api/crawl/logs", get(crawl_logs))
        .route(
            "/api/crawl/tasks",
            get(crawl_tasks).delete(delete_tasks),
        )
        .route("/api/crawl/retry-failed", post(retry_failed))
        .route("/api/crawl/stream", get(crawl_stream))
        // API 404
        .route("/api/{*path}", get(|| async {
            ApiResponse::<serde_json::Value>::err("接口不存在")
        }))
        // 静态文件 + SPA fallback (rust-embed 嵌入 frontend/dist/)
        .fallback(crate::web::static_handler)
        // 请求日志
        .layer(middleware::from_fn(log_request))
        .with_state(state)
}

pub async fn run_web() -> anyhow::Result<()> {
    let db = Arc::new(Mutex::new(appconfig::open_db()?));
    log::info!("数据库已连接: {}", appconfig::get_db_path().unwrap_or_default());

    let config_path = "spider.toml";
    let config = Config::builder()
        .add_source(config::File::with_name(config_path))
        .build()?;
    let config = Arc::new(config);

    let root_url = config
        .get_string("root_url")
        .unwrap_or_else(|_| "https://www.bz11111111.com/".to_string());
    log::info!("目标站点: {}", root_url);
    log::info!("定时爬取: enabled={}, schedule={}", 
        config.get_bool("cron.enabled").unwrap_or(true),
        config.get_string("cron.schedule").unwrap_or_else(|_| "0 */6 * * *".into()));

    // 旧 wreq5 BanzhuSpider 已删除，待迁移到 wisp 框架后在此重建 spider。
    let event_bus = crate::event::EventBus::new(256);
    let scheduler = Arc::new(Scheduler::new(
        db.clone(),
        config.clone(),
        event_bus.clone(),
    ));

    let state = Arc::new(AppState {
        db: db.clone(),
        scheduler: scheduler.clone(),
        event_bus,
    });

    // 启动时跑一次增量爬取（在主 runtime 上跑，关闭时自动取消）
    let scheduler_clone = scheduler.clone();
    let crawl_task = tokio::spawn(async move {
        if let Err(e) = scheduler_clone.crawl_once().await {
            log::error!("Initial crawl failed: {}", e);
        }
    });

    let app = build_router(state);

    // 获取可用端口：从配置端口开始，若被占用则自增查找
    let config_port = config.get_int("server.port").unwrap_or(4567) as u16;
    let port = wisp::utils::find_available_port(config_port, 100)
        .unwrap_or_else(|| {
            log::warn!("端口 {}-{} 均被占用，使用系统随机端口", config_port, config_port + 100);
            wisp::utils::get_random_port().expect("无法获取可用端口")
        });
    if port != config_port {
        log::info!("端口 {} 被占用，使用端口 {}", config_port, port);
    }

    let addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    log::info!("API 已启动: http://localhost:{}", port);

    // 保存 scheduler 引用供关闭时使用
    let scheduler_for_shutdown = scheduler.clone();

    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            tokio::signal::ctrl_c()
                .await
                .expect("failed to listen for ctrl_c");
            log::info!("收到关闭信号，正在优雅退出...");
            // 通知爬虫停止
            scheduler_for_shutdown.shutdown().await;
        })
        .await?;

    // 等待爬虫任务结束（已收到 shutdown 信号，应快速退出）
    log::info!("等待爬虫任务结束...");
    let _ = crawl_task.await;
    log::info!("爬虫任务已结束");

    Ok(())
}

// ─── Scheduler 辅助 ─────────────────────────────────────────────────────────

impl Scheduler {
    pub(crate) fn config(&self) -> &Arc<Config> {
        &self.config
    }
}
