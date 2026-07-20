use super::*;
use crate::web::ApiResponse;
use serde::Deserialize;

// ─── Bookshelf & Reading Progress ────────────────────────────────────────────

#[derive(Deserialize)]
pub(crate) struct ShelfQuery {
    pub group: Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct AddShelfBody {
    pub book_id: i64,
    pub group: Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct UpdateGroupBody {
    pub group: String,
}

#[derive(Deserialize)]
pub(crate) struct ProgressBody {
    pub chapter_order: i64,
    pub page_index: i64,
}

// GET /api/bookshelf?group=reading
pub(crate) async fn get_bookshelf(
    State(state): State<Arc<AppState>>,
    Query(q): Query<ShelfQuery>,
) -> Json<ApiResponse<Vec<crate::db::BookshelfRecord>>> {
    let db = state.db.lock().await;
    match db.get_bookshelf(q.group.as_deref()) {
        Ok(rows) => ApiResponse::ok(rows),
        Err(e) => ApiResponse::err(format!("查询失败: {}", e)),
    }
}

// POST /api/bookshelf  {book_id, group}
pub(crate) async fn add_to_bookshelf(
    State(state): State<Arc<AppState>>,
    Json(body): Json<AddShelfBody>,
) -> Json<ApiResponse<Value>> {
    let group = body.group.clone().unwrap_or_else(|| "默认".to_string());
    let db = state.db.lock().await;
    match db.add_to_bookshelf(body.book_id, &group) {
        Ok(()) => ok_response(json!({ "book_id": body.book_id, "group": group })),
        Err(e) => err_response(&format!("加入书架失败: {}", e)),
    }
}

// PUT /api/bookshelf/:bookId  {group}
pub(crate) async fn update_shelf_group(
    State(state): State<Arc<AppState>>,
    Path(book_id): Path<i64>,
    Json(body): Json<UpdateGroupBody>,
) -> Json<ApiResponse<Value>> {
    let db = state.db.lock().await;
    match db.update_bookshelf_group(book_id, &body.group) {
        Ok(()) => ok_response(json!({ "book_id": book_id, "group": body.group })),
        Err(e) => err_response(&format!("更新分组失败: {}", e)),
    }
}

// DELETE /api/bookshelf/:bookId
pub(crate) async fn remove_from_bookshelf(
    State(state): State<Arc<AppState>>,
    Path(book_id): Path<i64>,
) -> Json<ApiResponse<Value>> {
    let db = state.db.lock().await;
    match db.remove_from_bookshelf(book_id) {
        Ok(()) => ok_response(json!({ "book_id": book_id, "removed": true })),
        Err(e) => err_response(&format!("移除失败: {}", e)),
    }
}

// GET /api/progress/:bookId
pub(crate) async fn get_progress(
    State(state): State<Arc<AppState>>,
    Path(book_id): Path<i64>,
) -> Json<ApiResponse<Option<crate::db::ReadingProgressRecord>>> {
    let db = state.db.lock().await;
    match db.get_progress(book_id) {
        Ok(progress) => ApiResponse::ok(progress),
        Err(e) => ApiResponse::err(format!("查询失败: {}", e)),
    }
}

// PUT /api/progress/:bookId  {chapter_order, page_index}
pub(crate) async fn update_progress(
    State(state): State<Arc<AppState>>,
    Path(book_id): Path<i64>,
    Json(body): Json<ProgressBody>,
) -> Json<ApiResponse<Value>> {
    let db = state.db.lock().await;
    match db.update_progress(book_id, body.chapter_order, body.page_index) {
        Ok(()) => ok_response(json!({
            "book_id": book_id,
            "chapter_order": body.chapter_order,
            "page_index": body.page_index,
        })),
        Err(e) => err_response(&format!("更新进度失败: {}", e)),
    }
}
