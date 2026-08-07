//! 端到端验证：章节多页（chapterPages）功能。
//! mock server 提供带 2 个分页的章节页，跑 crawl_once 后验证 DB 中该章节有 2 条 sections。
//!
//! 运行：cargo test --test multipage_e2e -- --ignored --nocapture

mod common;

use banzhu_spider::db::Database;
use banzhu_spider::event::EventBus;
use banzhu_spider::scheduler::Scheduler;
use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::test]
#[ignore = "e2e 耗时较长：手动运行 cargo test --test multipage_e2e -- --ignored"]
async fn test_multipage_section_writes_two_rows() {
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

    // 2. 构造 config
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

    // 3. DB + Scheduler
    let db = Arc::new(Mutex::new(Database::open_in_memory().unwrap()));
    let event_bus = EventBus::new(128);
    let scheduler = Scheduler::new(db.clone(), Arc::new(config), event_bus);

    // 4. 执行 crawl_once
    scheduler.crawl_once().await.expect("crawl_once should succeed");

    // 5. 验证：章节 23456 应有 2 条 sections（多页）
    let db_guard = db.lock().await;
    let book = db_guard
        .get_book_by_website_id(12345)
        .expect("get_book 应成功")
        .expect("书 12345 应存在");

    // 找到 23456 章节
    let chapters = db_guard
        .get_chapters_by_book(book.id)
        .expect("chapters 查询应成功");
    let chapter = chapters
        .iter()
        .find(|c| c.url == format!("{root_url}/12/12345_1/23456.html"))
        .expect("章节 23456 应存在");

    let sections = db_guard
        .get_sections_by_chapter(chapter.id)
        .expect("sections 查询应成功");
    println!(
        "章节 {} sections 数 = {}",
        chapter.title,
        sections.len()
    );
    for s in &sections {
        println!("  section_order={} content_len={} url={}", s.section_order, s.content.len(), s.url);
    }

    assert_eq!(
        sections.len(),
        2,
        "多页章节应入库 2 条 sections，实际 {}",
        sections.len()
    );
    // 两页内容都应正确入库
    assert!(
        sections.iter().any(|s| s.content.contains("第 1 页正文")),
        "第 1 页正文应入库"
    );
    assert!(
        sections.iter().any(|s| s.content.contains("第 2 页正文")),
        "第 2 页正文应入库"
    );
}
