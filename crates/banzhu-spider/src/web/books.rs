use super::*;
use crate::web::ApiResponse;

// ─── Books ───────────────────────────────────────────────────────────────────

pub(crate) async fn list_books(
    State(state): State<Arc<AppState>>,
    Query(params): Query<BooksQuery>,
) -> Json<ApiResponse<Value>> {
    let page = params.page.unwrap_or(1).max(1);
    let limit = params.limit.unwrap_or(20).min(100);
    let offset = (page - 1) * limit;

    let db = state.db.lock().await;

    let total = match db.count_books() {
        Ok(t) => t,
        Err(e) => return err_response(&format!("查询失败: {}", e)),
    };

    let books = match db.list_books(limit, offset) {
        Ok(b) => b,
        Err(e) => return err_response(&format!("查询失败: {}", e)),
    };

    let items: Vec<Value> = books
        .into_iter()
        .filter(|b| {
            params
                .category
                .as_ref()
                .map_or(true, |c| b.category.contains(c))
        })
        .map(|b| {
            json!({
                "id": b.id,
                "title": b.title,
                "author": b.author,
                "category": b.category,
                "word_count": b.word_count,
                "likes": b.likes,
                "chapter_count": b.page_count,
                "created_at": b.created_at,
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

pub(crate) async fn book_detail(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Json<ApiResponse<Value>> {
    let db = state.db.lock().await;

    match db.get_book(id) {
        Ok(Some(book)) => {
            let chapter_count = db
                .get_chapters_by_book(id)
                .map(|c| c.len() as i64)
                .unwrap_or(0);

            ok_response(json!({
                "id": book.id,
                "title": book.title,
                "author": book.author,
                "category": book.category,
                "introduce": book.introduce,
                "word_count": book.word_count,
                "likes": book.likes,
                "chapter_count": chapter_count,
                "status": if book.page_count > 0 { "连载中" } else { "已完结" },
                "created_at": book.created_at,
            }))
        }
        Ok(None) => err_response("书籍不存在"),
        Err(e) => err_response(&format!("查询失败: {}", e)),
    }
}

pub(crate) async fn book_chapters(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Json<ApiResponse<Value>> {
    let db = state.db.lock().await;

    match db.get_chapters_by_book(id) {
        Ok(chapters) => {
            let items: Vec<Value> = chapters
                .into_iter()
                .map(|ch| {
                    json!({
                        "id": ch.id,
                        "title": ch.title,
                        "order": ch.chapter_order,
                    })
                })
                .collect();

            ok_response(json!({ "items": items, "total": items.len() }))
        }
        Err(e) => err_response(&format!("查询失败: {}", e)),
    }
}

pub(crate) async fn chapter_content(
    State(state): State<Arc<AppState>>,
    Path((book_id, order)): Path<(i64, i64)>,
) -> Json<ApiResponse<Value>> {
    let db = state.db.lock().await;

    let chapter = match db.get_chapter_by_book_and_order(book_id, order) {
        Ok(Some(ch)) => ch,
        Ok(None) => return err_response("章节不存在"),
        Err(e) => return err_response(&format!("查询失败: {}", e)),
    };

    let sections = match db.get_sections_by_chapter(chapter.id) {
        Ok(s) => s,
        Err(e) => return err_response(&format!("查询失败: {}", e)),
    };

    let content: String = sections.iter().map(|s| s.content.as_str()).collect();

    let all_chapters = db.get_chapters_by_book(book_id).unwrap_or_default();

    let prev_order = all_chapters
        .iter()
        .filter(|c| c.chapter_order < order)
        .max_by_key(|c| c.chapter_order)
        .map(|c| c.chapter_order);

    let next_order = all_chapters
        .iter()
        .filter(|c| c.chapter_order > order)
        .min_by_key(|c| c.chapter_order)
        .map(|c| c.chapter_order);

    ok_response(json!({
        "chapter_id": chapter.id,
        "title": chapter.title,
        "order": chapter.chapter_order,
        "book_id": book_id,
        "content": content,
        "prev_order": prev_order,
        "next_order": next_order,
    }))
}

// ─── Categories & Stats ──────────────────────────────────────────────────────

pub(crate) async fn categories(State(state): State<Arc<AppState>>) -> Json<ApiResponse<Value>> {
    let db = state.db.lock().await;

    match db.list_categories() {
        Ok(cats) => ok_response(json!({ "categories": cats })),
        Err(e) => err_response(&format!("查询失败: {}", e)),
    }
}

pub(crate) async fn stats(State(state): State<Arc<AppState>>) -> Json<ApiResponse<Value>> {
    let db = state.db.lock().await;

    let total_books = db.count_books().unwrap_or(0);

    let categories = db.list_categories().unwrap_or_default();
    let total_categories = categories.len() as i64;

    // ponytail: 简化分类分布统计，需要时再加详细实现
    let mut distribution = Vec::new();
    for cat in &categories {
        distribution.push(json!({
            "name": cat,
            "count": 0,
        }));
    }

    ok_response(json!({
        "total_books": total_books,
        "total_categories": total_categories,
        "category_distribution": distribution,
    }))
}
