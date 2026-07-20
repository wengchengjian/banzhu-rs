use crate::appconfig;
use crate::banzhuspider::BanzhuSpider;
use crate::db::Database;
use crate::scheduler::Scheduler;
use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{Request, StatusCode},
    middleware::{self, Next},
    response::{Json, Response},
    routing::{get, post, put},
    Router,
};
use tower_http::services::{ServeDir, ServeFile};
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

use books::{book_chapters, book_detail, categories, chapter_content, delete_book, list_books, stats};
use crawl::{crawl_logs, crawl_manual, crawl_schedule, crawl_status, crawl_trigger, update_crawl_schedule};
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

pub(crate) fn err_response(msg: &str) -> Json<ApiResponse<Value>> {
    ApiResponse::err(msg)
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

pub(crate) struct AppState {
    pub(crate) db: Arc<Mutex<Database>>,
    pub(crate) scheduler: Arc<Scheduler>,
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

// ─── Startup ─────────────────────────────────────────────────────────────────

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

    let spider = Arc::new(BanzhuSpider::new(root_url, config.clone()));

    let scheduler = Arc::new(Scheduler::new(spider, db.clone(), config.clone()));

    let state = Arc::new(AppState {
        db: db.clone(),
        scheduler: scheduler.clone(),
    });

    // 启动时跑一次增量爬取
    let scheduler_clone = scheduler.clone();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new()
            .expect("Failed to create tokio runtime for crawl");
        if let Err(e) = rt.block_on(scheduler_clone.crawl_once()) {
            log::error!("Initial crawl failed: {}", e);
        }
    });

    let app = Router::new()
        // 书籍
        .route("/api/books", get(list_books))
        .route("/api/books/{id}", get(book_detail).delete(delete_book))
        .route("/api/books/{id}/chapters", get(book_chapters))
        .route("/api/books/{id}/chapters/{order}", get(chapter_content))
        // 搜索
        .route("/api/search", get(search))
        // 分类 & 统计
        .route("/api/categories", get(categories))
        .route("/api/stats", get(stats))
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
        // API 404
        .route("/api/{*path}", get(|| async {
            ApiResponse::<serde_json::Value>::err("接口不存在")
        }))
        // 静态文件 + SPA fallback (非文件路由返回 index.html)
        .fallback_service(
            ServeDir::new(concat!(env!("CARGO_MANIFEST_DIR"), "/static"))
                .not_found_service(ServeFile::new(concat!(env!("CARGO_MANIFEST_DIR"), "/static/index.html")))
        )
        // 请求日志
        .layer(middleware::from_fn(log_request))
        .with_state(state);

    let port = config.get_int("server.port").unwrap_or(3000);
    let addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    log::info!("API 已启动: http://localhost:{}", port);
    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            tokio::signal::ctrl_c()
                .await
                .expect("failed to listen for ctrl_c");
            log::info!("收到关闭信号，正在优雅退出...");
        })
        .await?;
    Ok(())
}

// ─── Scheduler 辅助 ─────────────────────────────────────────────────────────

impl Scheduler {
    pub(crate) fn config(&self) -> &Arc<Config> {
        &self.config
    }
}
