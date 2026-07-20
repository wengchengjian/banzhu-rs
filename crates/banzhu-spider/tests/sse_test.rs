//! SSE 端到端集成测试。
//!
//! 验证 `/api/crawl/stream` 的三类行为：
//! 1. 重连后立即推送 `task:full` 初始事件
//! 2. `Last-Event-ID` 头触发遗漏日志补发
//! 3. EventBus 实时广播事件能被 SSE 流接收
//!
//! 注意：SSE 是无限流，不能用 `Response::bytes()` 一次性读取（会挂起），
//! 必须用 `bytes_stream()` 增量读取并设超时。

mod common;

use banzhu_spider::event::CrawlEvent;
use common::{setup_state, spawn_app};
use futures::StreamExt;
use reqwest::Client;
use tokio::time::{timeout, Duration};

/// 从 SSE 响应流中增量读取字节，直到 buffer 包含目标字符串或总超时。
async fn read_until_contains(
    res: reqwest::Response,
    needle: &str,
    overall_timeout: Duration,
) -> String {
    let mut stream = res.bytes_stream();
    let mut buf = String::new();
    let deadline = tokio::time::Instant::now() + overall_timeout;
    while tokio::time::Instant::now() < deadline {
        match timeout(Duration::from_millis(500), stream.next()).await {
            Ok(Some(Ok(chunk))) => {
                buf.push_str(&String::from_utf8_lossy(&chunk));
                if buf.contains(needle) {
                    return buf;
                }
            }
            _ => continue,
        }
    }
    buf
}

#[tokio::test]
async fn sse_initial_sends_task_full_when_no_tasks() {
    let state = setup_state().await;
    let base = spawn_app(state).await;
    let client = Client::new();

    let res = client
        .get(format!("{}/api/crawl/stream", base))
        .header("Accept", "text/event-stream")
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    assert_eq!(
        res.headers().get("content-type").unwrap(),
        "text/event-stream"
    );

    // 读取流直到包含 task:full 事件（axum 序列化为 `event:task:full`）
    let text = read_until_contains(res, "task:full", Duration::from_secs(3)).await;
    assert!(
        text.contains("event:task:full") || text.contains("event: task:full"),
        "SSE 应推送初始 task:full 事件, 实际: {}",
        &text[..text.len().min(500)]
    );
}

#[tokio::test]
async fn sse_replay_missed_logs_via_last_event_id() {
    let state = setup_state().await;
    let base = spawn_app(state.clone()).await;
    let client = Client::new();

    // 先写入 3 条日志（用 Database 公开 API）
    {
        let db = state.db.lock().await;
        db.insert_crawl_log("INFO", "日志 1").unwrap();
        db.insert_crawl_log("INFO", "日志 2").unwrap();
        db.insert_crawl_log("INFO", "日志 3").unwrap();
    }

    // 带 Last-Event-ID: 1 重连，应补发 id=2, id=3 的日志
    let res = client
        .get(format!("{}/api/crawl/stream", base))
        .header("Last-Event-ID", "1")
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);

    // 读取直到看到「日志 3」（最后一条补发）
    let text = read_until_contains(res, "日志 3", Duration::from_secs(3)).await;
    assert!(
        text.contains("日志 2"),
        "应补发 id=2 的日志, 实际: {}",
        &text[..text.len().min(500)]
    );
    assert!(
        text.contains("日志 3"),
        "应补发 id=3 的日志, 实际: {}",
        &text[..text.len().min(500)]
    );
    assert!(
        !text.contains("日志 1"),
        "不应补发 id<=last_event_id 的日志, 实际: {}",
        &text[..text.len().min(500)]
    );
}

#[tokio::test]
async fn sse_broadcasts_live_event() {
    let state = setup_state().await;
    let base = spawn_app(state.clone()).await;
    let client = Client::new();

    // 启动 SSE 订阅
    let res = client
        .get(format!("{}/api/crawl/stream", base))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);

    // 等待服务器完成 subscribe() + 推送 initial events
    tokio::time::sleep(Duration::from_millis(400)).await;

    // 在另一个任务中触发 broadcast 事件
    let event_bus = state.event_bus.clone();
    event_bus.emit(CrawlEvent::Status {
        running: true,
        current_page: 0,
        pages_limit: 0,
        books_found: 0,
        books_downloaded: 0,
        books_failed: 0,
        books_skipped: 0,
        message: "测试实时事件".into(),
    });

    // 继续读取流，应能收到实时 status 事件
    let text = read_until_contains(res, "测试实时事件", Duration::from_secs(3)).await;
    assert!(
        text.contains("event:status")
            || text.contains("event: status")
            || text.contains("测试实时事件"),
        "应收到实时 status 事件, 实际: {}",
        &text[..text.len().min(500)]
    );
}
