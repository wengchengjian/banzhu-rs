//! wisp Engine 端到端集成测试。
//!
//! 用 axum mock server 模拟 banzhu 站点结构，验证 wisp Engine 全流程
//! （list → book_detail → chapter → section）能正确工作：
//! 1. 列表页解析书籍并 follow 到详情页
//! 2. 详情页解析书籍元数据并 follow 到章节分页
//! 3. 章节分页解析章节列表并 follow 到正文页
//! 4. 正文页策略 1 提取内容
//! 5. items 经 BatchItemPipeline 写入 DB

mod common;

use banzhu_spider::db::Database;
use banzhu_spider::event::EventBus;
use banzhu_spider::scheduler::CrawlStatus;
use banzhu_spider::spider;
use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::test]
async fn test_wisp_engine_end_to_end_with_mock_server() {
    // 1. 启动 mock server
    let app = common::mock_server::make_mock_app();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    let root_url = format!("http://{}", addr);

    // 2. 构造 in-memory DB + EventBus + CrawlStatus
    let db = Arc::new(Mutex::new(Database::open_in_memory().unwrap()));
    let event_bus = EventBus::new(128);
    let status = Arc::new(Mutex::new(CrawlStatus::default()));

    // 3. 构造 spider（pages_limit=1：只爬第一页列表，由 list → detail → chapter → section 链式 follow 完成全流程）
    let img_dict = Arc::new(spider::init_img_fanpa_dict());
    let font_dict = Arc::new(spider::init_font_fanpa_dict());
    let config = Arc::new(config::Config::builder().build().unwrap());
    let spider = spider::build_spider(
        root_url,
        1,
        db.clone(),
        config,
        event_bus,
        status,
        img_dict,
        font_dict,
    );

    // 4. 运行 engine（Http 模式避免触发浏览器）
    let engine = wisp::crawl::Engine::infra()
        .max_concurrent(2)
        .max_pages(10)
        .download_delay(std::time::Duration::from_millis(10))
        .obey_robots(false)
        .fetch_mode(wisp::fetcher::FetchMode::Http)
        .build()
        .unwrap();

    let (stats, _items) = engine.run(spider).await.expect("engine run should succeed");

    // 5. 验证 stats：至少爬取 4 页（list + book_detail + chapter + section）
    assert!(
        stats.pages_crawled >= 4,
        "至少应爬取 4 页 (list+detail+chapter+section)，实际: {}",
        stats.pages_crawled
    );

    // 6. 验证 DB 写入：书 12345 应已写入
    let db_guard = db.lock().await;
    let book_exists = db_guard.book_exists_by_website_id(12345).unwrap_or(false);
    assert!(book_exists, "书 12345 应已写入 DB");
}
