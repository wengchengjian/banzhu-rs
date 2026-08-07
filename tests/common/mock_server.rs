//! axum mock server：模拟 banzhu 站点结构，供 wisp Engine 集成测试使用。
//!
//! HTML 结构严格匹配 `src/spider/callbacks.rs` + `src/spider/parse.rs` 的选择器：
//! - `list_handler`：`li.column-2` 下的 `a.name`，href 形如 `/12/12345/`
//! - `book_detail_handler` → `parse_book_info`：`.pagelistbox .page` + `h1` + `.bd` + `.info`
//! - `chapter_handler` → `parse_chapter_list`：第二个 `.chapter-list` 下的 `.bd .list li a`
//! - `section_handler` → `try_section_data1`：`.page-content p`（不触发 `needs_section_post`）

use axum::{response::Html, routing::get, Router};

/// 构造 mock axum Router，覆盖 list / book_detail / chapter / section 四类路由。
pub fn make_mock_app() -> Router {
    Router::new()
        .route("/shuku/0-lastupdate-0-1.html", get(|| async { Html(LIST_PAGE) }))
        .route("/shuku/0-lastupdate-0-2.html", get(|| async { Html(LIST_PAGE_EMPTY) }))
        .route("/shuku/0-lastupdate-0-3.html", get(|| async { Html(LIST_PAGE_EMPTY) }))
        .route("/shuku/0-lastupdate-0-4.html", get(|| async { Html(LIST_PAGE_EMPTY) }))
        .route("/12/12345/", get(|| async { Html(BOOK_DETAIL) }))
        .route("/12/12345_1/", get(|| async { Html(CHAPTER_PAGE) }))
        .route("/12/12345_1/23456.html", get(|| async { Html(SECTION_PAGE_MULTI) }))
        .route("/12/12345_1/23456_1.html", get(|| async { Html(SECTION_PAGE) }))
        .route("/12/12345_1/23456_2.html", get(|| async { Html(SECTION_PAGE_2) }))
}

/// 非空列表页：含一本 book（book_num=12, book_id=12345）。
const LIST_PAGE: &str = r#"<!DOCTYPE html><html><body>
<ul class="txt-list">
<li class="column-2">
<a class="name" href="/12/12345/">书名1</a>
</li>
</ul></body></html>"#;

/// 空列表页：无 `li.column-2`，触发 `EmptyPageTracker::record_empty`。
const LIST_PAGE_EMPTY: &str = r#"<!DOCTYPE html><html><body></body></html>"#;

/// 书籍详情页：`.pagelistbox .page` 文本匹配 PAGE_REGEX `(第1/1页)当前10条/页`。
const BOOK_DETAIL: &str = r#"<!DOCTYPE html><html><body>
<div class="pagelistbox"><span class="page">(第1/1页)当前10条/页</span></div>
<h1>书名1</h1>
<div class="bd">简介内容</div>
<div class="info">作者：张三<br>分类：玄幻<br>字数：100000<br>喜欢：200</div>
</body></html>"#;

/// 章节分页页：含两个 `.chapter-list`，第二个下的 `.bd .list li a` 会被解析。
/// 注意：`<ul>` 必须带 `class="list"` 以匹配 `.bd .list li a` 选择器。
const CHAPTER_PAGE: &str = r#"<!DOCTYPE html><html><body>
<div class="chapter-list">..</div>
<div class="chapter-list">
<div class="bd"><ul class="list">
<li><a href="/12/12345_1/23456.html">第1章</a></li>
</ul></div>
</div>
</body></html>"#;

/// 章节正文页：策略 1（`.page-content p`），不触发 `needs_section_post`。
const SECTION_PAGE: &str = r#"<!DOCTYPE html><html><body>
<div class="page-content"><p>正文内容</p></div>
</body></html>"#;

/// 多页章节首页（`23456.html`）：`.chapterPages` 含 2 个分页链接，
/// 触发 section_handler 检测多页并 follow `23456_2.html`。
const SECTION_PAGE_MULTI: &str = r#"<!DOCTYPE html><html><body>
<div class="page-content"><p>第 1 页正文</p></div>
<center class="chapterPages">
<a href="23456_1.html" class="curr">【1】</a>
<a href="23456_2.html">【2】</a>
</center>
</body></html>"#;

/// 多页章节第 2 页（`23456_2.html`）：仅正文，无分页链接（已是最后一页）。
const SECTION_PAGE_2: &str = r#"<!DOCTYPE html><html><body>
<div class="page-content"><p>第 2 页正文</p></div>
</body></html>"#;
