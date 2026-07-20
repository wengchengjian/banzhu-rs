use super::*;
use crate::error::AppResult;
use crate::web::ApiResponse;
use chrono::Datelike;
use serde::Deserialize;
use serde_json::{json, Value};

// ─── Reading Stats ───────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub(crate) struct HeatmapParams {
    pub year: Option<i32>,
}

/// GET /api/stats/heatmap?year=2026
/// 返回年度阅读热力图数据：[{date, duration_sec, chapters_read}]
pub(crate) async fn heatmap(
    State(state): State<Arc<AppState>>,
    Query(params): Query<HeatmapParams>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let year = params.year.unwrap_or_else(|| chrono::Local::now().year());
    let db = state.db.lock().await;
    let rows = db.heatmap_data(year)?;
    let points: Vec<Value> = rows
        .into_iter()
        .map(|(date, duration_sec, chapters_read)| {
            json!({
                "date": date,
                "duration_sec": duration_sec,
                "chapters_read": chapters_read,
            })
        })
        .collect();
    Ok(ok_response(json!({ "items": points })))
}

#[derive(Deserialize)]
pub(crate) struct TimelineParams {
    pub days: Option<i64>,
}

/// GET /api/stats/reading-timeline?days=7
/// 返回最近 N 天的阅读时间线：[{date, duration_sec, chapters_read}]
pub(crate) async fn reading_timeline(
    State(state): State<Arc<AppState>>,
    Query(params): Query<TimelineParams>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let days = params.days.unwrap_or(7).max(1).min(365) as i32;
    let db = state.db.lock().await;
    let rows = db.reading_timeline(days)?;
    let points: Vec<Value> = rows
        .into_iter()
        .map(|(date, duration_sec, chapters_read)| {
            json!({
                "date": date,
                "duration_sec": duration_sec,
                "chapters_read": chapters_read,
            })
        })
        .collect();
    Ok(ok_response(json!({ "items": points })))
}

#[derive(Deserialize)]
pub(crate) struct ReportSessionBody {
    pub book_id: i64,
    pub chapter_order: i64,
    pub duration_sec: i64,
    pub chapters_read: i64,
    pub started_at: i64,
    pub ended_at: i64,
}

/// POST /api/stats/reading-session
/// 上报一次阅读会话
pub(crate) async fn report_session(
    State(state): State<Arc<AppState>>,
    Json(body): Json<ReportSessionBody>,
) -> AppResult<Json<ApiResponse<Value>>> {
    // 基本校验
    if body.duration_sec <= 0 {
        return Err(crate::error::AppError::BadRequest(
            "duration_sec 必须大于 0".into(),
        ));
    }
    if body.book_id <= 0 {
        return Err(crate::error::AppError::BadRequest("book_id 无效".into()));
    }
    let db = state.db.lock().await;
    let session_id = db.insert_reading_session(
        body.book_id,
        body.chapter_order,
        body.duration_sec,
        body.chapters_read,
        body.started_at,
        body.ended_at,
    )?;
    Ok(ok_response(json!({
        "ok": true,
        "session_id": session_id,
    })))
}

/// GET /api/stats/reading-goal
/// 获取阅读目标配置
pub(crate) async fn get_reading_goal(
    State(state): State<Arc<AppState>>,
) -> AppResult<Json<ApiResponse<crate::db::ReadingGoalRecord>>> {
    let db = state.db.lock().await;
    let goal = db.get_reading_goal()?;
    Ok(ApiResponse::ok(goal))
}

#[derive(Deserialize)]
pub(crate) struct UpdateGoalBody {
    pub daily_minutes: i64,
    pub daily_chapters: i64,
}

/// PUT /api/stats/reading-goal
/// 更新阅读目标，返回更新后的目标
pub(crate) async fn update_reading_goal(
    State(state): State<Arc<AppState>>,
    Json(body): Json<UpdateGoalBody>,
) -> AppResult<Json<ApiResponse<crate::db::ReadingGoalRecord>>> {
    if body.daily_minutes < 0 || body.daily_chapters < 0 {
        return Err(crate::error::AppError::BadRequest(
            "daily_minutes 和 daily_chapters 不能为负数".into(),
        ));
    }
    let db = state.db.lock().await;
    db.update_reading_goal(body.daily_minutes, body.daily_chapters)?;
    let goal = db.get_reading_goal()?;
    Ok(ApiResponse::ok(goal))
}

/// GET /api/stats/today
/// 今日阅读聚合（duration_sec + chapters_read）
pub(crate) async fn today_reading(
    State(state): State<Arc<AppState>>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let db = state.db.lock().await;
    let (duration, chapters) = db.sum_today_reading()?;
    Ok(ok_response(json!({
        "duration_sec": duration,
        "chapters_read": chapters,
    })))
}

#[derive(Deserialize)]
pub(crate) struct HistoryParams {
    pub limit: Option<i64>,
}

/// GET /api/stats/reading-history?limit=20
/// 阅读历史（按最近阅读时间排序）
pub(crate) async fn reading_history(
    State(state): State<Arc<AppState>>,
    Query(params): Query<HistoryParams>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let limit = params.limit.unwrap_or(20).clamp(1, 100);
    let db = state.db.lock().await;
    let rows = db.reading_history(limit)?;
    let items: Vec<Value> = rows
        .into_iter()
        .map(|r| {
            json!({
                "book_id": r.book_id,
                "book_title": r.book_title,
                "last_read_at": r.last_read_at,
                "last_chapter_order": r.last_chapter_order,
                "total_duration_sec": r.total_duration_sec,
                "total_chapters": r.chapters_read,
            })
        })
        .collect();
    Ok(ok_response(json!({ "items": items })))
}
