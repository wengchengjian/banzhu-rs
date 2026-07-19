use super::*;
use crate::web::ApiResponse;
use crate::search::SearchField;

// ─── Search ──────────────────────────────────────────────────────────────────

pub(crate) async fn search(
    State(state): State<Arc<AppState>>,
    Query(params): Query<SearchQuery>,
) -> Json<ApiResponse<Value>> {
    let keyword = match params.q.as_ref().map(|s| s.trim()) {
        Some(k) if !k.is_empty() => k,
        _ => return err_response("缺少搜索关键词 q"),
    };

    let page = params.page.unwrap_or(1).max(1);
    let limit = params.limit.unwrap_or(20).min(100);
    let offset = (page - 1) * limit;
    let exact = params.exact.unwrap_or(false);

    let search_field = match params.field.as_deref() {
        Some("title") => SearchField::Title,
        Some("author") => SearchField::Author,
        Some("content") => SearchField::Content,
        _ => SearchField::All,
    };

    let db = state.db.lock().await;

    let total = match db.search_fts_count(keyword, exact) {
        Ok(t) => t,
        Err(e) => return err_response(&format!("搜索失败: {}", e)),
    };

    let results = match db.search_fts(keyword, exact, search_field, limit, offset) {
        Ok(r) => r,
        Err(e) => return err_response(&format!("搜索失败: {}", e)),
    };

    let items: Vec<Value> = results
        .into_iter()
        .map(|r| {
            // 清理 snippet 中的 ANSI 高亮标记
            let snippet = r.snippet
                .replace("\x1b[33m", ">>>")
                .replace("\x1b[0m", "<<<");
            json!({
                "book_id": r.book_id,
                "title": r.title,
                "author": r.author,
                "category": r.category,
                "word_count": r.word_count,
                "relevance_score": r.relevance_score,
                "snippet": snippet,
                "created_at": r.created_at,
            })
        })
        .collect();

    ok_response(json!({
        "total": total,
        "page": page,
        "limit": limit,
        "items": items,
    }))
}
