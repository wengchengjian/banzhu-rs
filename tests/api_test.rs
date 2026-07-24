//! API 统一响应信封集成测试。
//!
//! 验证 `/api/books`、`/api/books/{id}`、`/api/stats/reading-goal`、
//! `/api/stats/reading-session`、`/api/stats/today` 的返回结构符合
//! `{ code, msg?, data? }` 统一信封约定。

mod common;

use banzhu_spider::db::BookRecord;
use common::setup_state;
use reqwest::Client;
use serde_json::Value;

#[tokio::test]
async fn list_books_returns_unified_envelope() {
    let state = setup_state().await;
    let base = common::spawn_app(state).await;
    let client = Client::new();

    let res = client
        .get(format!("{}/api/books", base))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["code"], 0, "成功响应 code 应为 0");
    assert!(body.get("data").is_some(), "应有 data 字段");
    // list_books 返回分页结构 { total, page, limit, items }
    assert!(body["data"].get("items").is_some(), "data 应含 items 数组");
}

#[tokio::test]
async fn unknown_book_returns_error_envelope() {
    let state = setup_state().await;
    let base = common::spawn_app(state).await;
    let client = Client::new();

    let res = client
        .get(format!("{}/api/books/999999", base))
        .send()
        .await
        .unwrap();
    // AppError::NotFound → 404，body 仍为 { code: -1, msg: "not found" }
    assert_eq!(res.status(), 404);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["code"], -1, "失败响应 code 应为 -1");
    assert!(body["msg"].is_string(), "失败应有 msg 字段");
    assert!(
        body.get("data").is_none() || body["data"].is_null(),
        "失败响应不应有 data"
    );
}

#[tokio::test]
async fn reading_goal_get_and_update() {
    let state = setup_state().await;
    let base = common::spawn_app(state).await;
    let client = Client::new();

    // 初始默认值（schema 中 DEFAULT 30/5）
    let res = client
        .get(format!("{}/api/stats/reading-goal", base))
        .send()
        .await
        .unwrap();
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["code"], 0);
    assert_eq!(body["data"]["daily_minutes"], 30);
    assert_eq!(body["data"]["daily_chapters"], 5);

    // 更新
    let res = client
        .put(format!("{}/api/stats/reading-goal", base))
        .json(&serde_json::json!({ "daily_minutes": 60, "daily_chapters": 10 }))
        .send()
        .await
        .unwrap();
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["code"], 0);
    assert_eq!(body["data"]["daily_minutes"], 60);
    assert_eq!(body["data"]["daily_chapters"], 10);

    // 再次读取确认持久化
    let res = client
        .get(format!("{}/api/stats/reading-goal", base))
        .send()
        .await
        .unwrap();
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["data"]["daily_minutes"], 60);
    assert_eq!(body["data"]["daily_chapters"], 10);
}

#[tokio::test]
async fn report_reading_session_persists() {
    let state = setup_state().await;
    let base = common::spawn_app(state.clone()).await;
    let client = Client::new();

    // 先插入一本书（用 Database 公开 API，不能直接 SQL）
    {
        let db = state.db.lock().await;
        let book = BookRecord {
            id: 0,
            website_book_id: None,
            path_num: 0,
            title: "测试书".into(),
            filename: "测试书".into(),
            author: "作者".into(),
            category: "".into(),
            introduce: "".into(),
            likes: 0,
            word_count: 0,
            page_count: 0,
            created_at: 0,
            updated_at: 0,
        };
        db.insert_book(&book).expect("insert_book 失败");
    }

    // 上报一次会话
    let res = client
        .post(format!("{}/api/stats/reading-session", base))
        .json(&serde_json::json!({
            "book_id": 1,
            "chapter_order": 1,
            "duration_sec": 300,
            "chapters_read": 1,
            "started_at": 1700000000,
            "ended_at": 1700000300,
        }))
        .send()
        .await
        .unwrap();
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["code"], 0, "上报会话应成功: {}", body);

    // /api/stats/today 端点应能查到（字段 duration_sec）
    let res = client
        .get(format!("{}/api/stats/today", base))
        .send()
        .await
        .unwrap();
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["code"], 0);
    let duration = body["data"]["duration_sec"].as_i64().unwrap_or(0);
    // 注意：sum_today_reading 用 'start of day' localtime 过滤，
    // 测试时间戳 1700000000 是 2023-11-14，若今日 != 该日期则 duration=0。
    // 因此只验证字段存在与类型，不强制 >= 300。
    assert!(
        duration >= 0,
        "今日总时长应为非负整数, 实际: {}",
        duration
    );
    assert!(
        body["data"].get("chapters_read").is_some(),
        "data 应含 chapters_read 字段"
    );
}

#[tokio::test]
async fn report_reading_session_today_persists() {
    // 用「现在」时间戳确保落在今日（localtime start of day 之后）
    let state = setup_state().await;
    let base = common::spawn_app(state.clone()).await;
    let client = Client::new();

    {
        let db = state.db.lock().await;
        let book = BookRecord {
            id: 0,
            website_book_id: None,
            path_num: 0,
            title: "测试书2".into(),
            filename: "测试书2".into(),
            author: "作者".into(),
            category: "".into(),
            introduce: "".into(),
            likes: 0,
            word_count: 0,
            page_count: 0,
            created_at: 0,
            updated_at: 0,
        };
        db.insert_book(&book).expect("insert_book 失败");
    }

    let now = chrono::Utc::now().timestamp();
    let res = client
        .post(format!("{}/api/stats/reading-session", base))
        .json(&serde_json::json!({
            "book_id": 1,
            "chapter_order": 1,
            "duration_sec": 300,
            "chapters_read": 1,
            "started_at": now,
            "ended_at": now + 300,
        }))
        .send()
        .await
        .unwrap();
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["code"], 0, "上报会话应成功: {}", body);

    let res = client
        .get(format!("{}/api/stats/today", base))
        .send()
        .await
        .unwrap();
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["code"], 0);
    let duration = body["data"]["duration_sec"].as_i64().unwrap_or(0);
    assert!(
        duration >= 300,
        "今日总时长应 >= 300, 实际: {}",
        duration
    );
}
