use super::*;
use crate::error::{AppError, AppResult};
use crate::web::ApiResponse;

// ─── Books ───────────────────────────────────────────────────────────────────

pub(crate) async fn list_books(
    State(state): State<Arc<AppState>>,
    Query(params): Query<BooksQuery>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let page = params.page.unwrap_or(1).max(1);
    let limit = params.limit.unwrap_or(20).min(100);
    let offset = (page - 1) * limit;

    let db = state.db.lock().await;

    let category = params.category.as_deref().filter(|c| !c.is_empty());

    let total = match category {
        Some(c) => db.count_books_by_category(c)?,
        None => db.count_books()?,
    };

    let books = db.list_books(limit, offset, category)?;

    // 批量查询章节数，避免 N+1
    let mut items = Vec::with_capacity(books.len());
    for b in books {
        let chapter_count = db.count_chapters_by_book(b.id).unwrap_or(0);
        items.push(json!({
            "id": b.id,
            "title": b.title,
            "author": b.author,
            "category": b.category,
            "word_count": b.word_count,
            "likes": b.likes,
            "chapter_count": chapter_count,
            "created_at": b.created_at,
        }));
    }

    Ok(ok_response(json!({
        "total": total,
        "page": page,
        "limit": limit,
        "items": items,
    })))
}

pub(crate) async fn book_detail(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let db = state.db.lock().await;

    let book = db.get_book(id)?.ok_or(AppError::NotFound)?;
    let chapter_count = db.count_chapters_by_book(id).unwrap_or(0);

    Ok(ok_response(json!({
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
    })))
}

// DELETE /api/books/:id
pub(crate) async fn delete_book(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let db = state.db.lock().await;

    // 先校验存在，再删除
    db.get_book(id)?.ok_or(AppError::NotFound)?;
    db.delete_book(id)?;

    Ok(ok_response(json!({ "deleted": id })))
}

pub(crate) async fn book_chapters(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let db = state.db.lock().await;

    let chapters = db.get_chapters_by_book(id)?;
    let total = chapters.len() as i64;
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

    Ok(ok_response(json!({ "items": items, "total": total })))
}

pub(crate) async fn chapter_content(
    State(state): State<Arc<AppState>>,
    Path((book_id, order)): Path<(i64, i64)>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let db = state.db.lock().await;

    let chapter = db
        .get_chapter_by_book_and_order(book_id, order)?
        .ok_or(AppError::NotFound)?;

    let sections = db.get_sections_by_chapter(chapter.id)?;

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

    Ok(ok_response(json!({
        "chapter_id": chapter.id,
        "title": chapter.title,
        "order": chapter.chapter_order,
        "book_id": book_id,
        "content": content,
        "prev_order": prev_order,
        "next_order": next_order,
    })))
}

// ─── Categories & Stats ──────────────────────────────────────────────────────

pub(crate) async fn categories(State(state): State<Arc<AppState>>) -> AppResult<Json<ApiResponse<Value>>> {
    let db = state.db.lock().await;

    let cats = db.list_categories()?;
    Ok(ok_response(json!({ "categories": cats })))
}

pub(crate) async fn stats(State(state): State<Arc<AppState>>) -> AppResult<Json<ApiResponse<Value>>> {
    let db = state.db.lock().await;

    let total_books = db.count_books().unwrap_or(0);
    let total_chapters = db.count_all_chapters().unwrap_or(0);
    let total_words = db.sum_all_word_count().unwrap_or(0);

    let categories = db.list_categories().unwrap_or_default();
    let total_categories = categories.len() as i64;

    // 分类分布（带书籍数）
    let distribution = db
        .category_distribution()
        .unwrap_or_default()
        .into_iter()
        .map(|(name, count)| json!({ "name": name, "count": count }))
        .collect::<Vec<_>>();

    // 爬取任务状态统计
    let crawl_task_stats = db.count_crawl_tasks_by_status().unwrap_or_default();

    Ok(ok_response(json!({
        "total_books": total_books,
        "total_chapters": total_chapters,
        "total_words": total_words,
        "total_categories": total_categories,
        "category_distribution": distribution,
        "crawl_tasks": crawl_task_stats,
    })))
}
