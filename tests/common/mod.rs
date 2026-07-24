//! 集成测试共享代码：构造内存数据库 + 启动测试 HTTP 服务器。

use banzhu_spider::banzhuspider::BanzhuSpider;
use banzhu_spider::db::Database;
use banzhu_spider::event::EventBus;
use banzhu_spider::scheduler::Scheduler;
use banzhu_spider::web::{build_router, AppState};
use config::Config;
use std::sync::Arc;
use tokio::sync::Mutex;

/// 构造测试用 AppState + 内存数据库。
///
/// `Database::open_in_memory()` 会自动调用 `init_tables + init_fts`，
/// 无需手动执行 schema。
pub async fn setup_state() -> Arc<AppState> {
    let db = Database::open_in_memory().expect("open_in_memory 失败");
    let db = Arc::new(Mutex::new(db));

    let config = Arc::new(
        Config::builder()
            .build()
            .expect("Config build 失败"),
    );
    let spider = Arc::new(BanzhuSpider::new("https://example.com/".into(), config.clone()));
    let event_bus = EventBus::new(256);
    let scheduler = Arc::new(Scheduler::new(
        spider,
        db.clone(),
        config,
        event_bus.clone(),
    ));

    Arc::new(AppState {
        db,
        scheduler,
        event_bus,
    })
}

/// 启动测试 HTTP 服务器，返回 base URL（如 `http://127.0.0.1:0` 实际端口）。
pub async fn spawn_app(state: Arc<AppState>) -> String {
    let app = build_router(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind 失败");
    let addr = listener.local_addr().expect("local_addr 失败");
    tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, app).await {
            eprintln!("测试服务器异常: {}", e);
        }
    });
    format!("http://{}", addr)
}
