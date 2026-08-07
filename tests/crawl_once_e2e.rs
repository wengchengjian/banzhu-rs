//! 临时全流程测试：通过 `Scheduler::crawl_once` 走完整调度路径
//! （配置读取 → 字典加载 → spider 构建 → engine 构建 → run → 收尾标记）。
//! 用 mock server 模拟站点，验证 crawl_once 端到端能写入 DB。

mod common;

use banzhu_spider::db::Database;
use banzhu_spider::event::EventBus;
use banzhu_spider::scheduler::Scheduler;
use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::test]
#[ignore = "e2e 耗时较长：手动运行 cargo test --test crawl_once_e2e -- --ignored"]
async fn test_crawl_once_end_to_end() {
    // 初始化 logger（读取 RUST_LOG，便于观察 crawl_once 的 debug 日志）
    let _ = env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info"),
    )
    .try_init();

    // 1. 启动 mock server
    let app = common::mock_server::make_mock_app();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    let root_url = format!("http://{}", addr);

    // 2. 构造 config 指向 mock server
    let config = config::Config::builder()
        .set_default("root_url", root_url.as_str())
        .unwrap()
        .set_default("cron.enabled", true)
        .unwrap()
        .set_default("cron.pages_limit", 1i64)
        .unwrap()
        .set_default("cron.book_concurrency", 2i64)
        .unwrap()
        .set_default("wisp.download_delay_ms", 1i64)
        .unwrap()
        .set_default("wisp.obey_robots", false)
        .unwrap()
        .set_default("wisp.fetch_mode", "http")
        .unwrap()
        .set_default("wisp.stealth.headless", false)
        .unwrap()
        .set_default("wisp.stealth.challenge_timeout_secs", 30i64)
        .unwrap()
        .set_default("wisp.stealth.human_mode", false)
        .unwrap()
        .set_default("wisp.stealth.cf_cookie_ttl_secs", 60i64)
        .unwrap()
        .build()
        .unwrap();

    // 3. 构造 DB + Scheduler
    let db = Arc::new(Mutex::new(Database::open_in_memory().unwrap()));
    let event_bus = EventBus::new(128);
    let scheduler = Scheduler::new(db.clone(), Arc::new(config), event_bus);

    // 4. 执行完整 crawl_once
    scheduler.crawl_once().await.expect("crawl_once should succeed");

    // 5. 验证 DB 写入：书 12345 应已写入
    let db_guard = db.lock().await;
    let book_exists = db_guard.book_exists_by_website_id(12345).unwrap_or(false);
    assert!(book_exists, "书 12345 应已通过 crawl_once 写入 DB");

    // 6. 验证状态复位
    let status = scheduler.status.lock().await;
    assert!(!status.running, "crawl_once 结束后 running 应复位为 false");
    assert!(
        !status.message.is_empty(),
        "crawl_once 结束应写入 summary message"
    );
}
