//! SpiderBuilder callback 工厂函数。
//!
//! 5 个工厂函数对应 5 个 `on(label, ...)` 回调，捕获各自所需状态：
//! - `list_handler`：列表页 → 书籍列表 → follow `book_detail`
//! - `book_detail_handler`：详情页 → Book item + follow `chapter`
//! - `chapter_handler`：章节分页 → Chapter items + follow `section`
//! - `section_handler`：章节页 → 4 策略 → Section item 或 follow `section_post`
//! - `section_post_handler`：POST 响应 → format_content_html → Section item
//!
//! 闭包返回 `Pin<Box<dyn Future + Send>>` 以满足 wisp `SpiderBuilder::on` 的
//! `F: Fn(Response) -> Fut` 约束（无法用具名 async 块类型）。

use crate::db::Database;
use crate::spider::parse;
use crate::spider::stop::EmptyPageTracker;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Mutex;
use wisp::fetcher::{Request, Response};

/// `on("default")`：列表页 → 解析书籍 → follow 到 `book_detail`。
///
/// 解析 `li.column-2 > a.name`，从 href 中拆出 `book_num/book_id`，
/// 携带 meta（book_id/book_num/title/author）follow 到 `book_detail`。
/// 空页/非空页通过 `EmptyPageTracker` 共享原子计数器上报（用于 `until` 终止条件）。
pub fn list_handler(
    tracker: EmptyPageTracker,
) -> impl Fn(Response) -> Pin<Box<dyn Future<Output = (Vec<Value>, Vec<Request>)> + Send + 'static>>
    + Send + Sync + 'static {
    move |resp| {
        let tracker = tracker.clone();
        Box::pin(async move {
            let doc = resp.parse();
            let mut follows = Vec::new();
            let mut found_any = false;
            for li in doc.select("li.column-2").iter() {
                let Some(name_el) = li.select_one("a.name") else {
                    continue;
                };
                let Some(href) = name_el.attr("href") else {
                    continue;
                };
                let parts: Vec<&str> = href.trim_matches('/').split('/').collect();
                if parts.len() < 2 {
                    continue;
                }
                let book_num: u64 = parts[0].parse().unwrap_or(0);
                let book_id: u64 = parts[1].parse().unwrap_or(0);
                let title = name_el.text().trim().to_string();
                if title.is_empty() || book_id == 0 {
                    continue;
                }
                found_any = true;
                if let Some(req) = resp.follow_meta(
                    &href,
                    json!({
                        "book_id": book_id,
                        "book_num": book_num,
                        "title": title,
                        "author": "",
                    }),
                ) {
                    follows.push(req.with_callback("book_detail"));
                }
            }
            if found_any {
                tracker.record_non_empty();
            } else {
                tracker.record_empty();
            }
            (Vec::new(), follows)
        })
    }
}

/// `on("book_detail")`：书籍详情页 → emit Book item + follow 章节分页。
///
/// 调用 `parse::parse_book_info` 提取元数据；用 meta 中的 `book_num/book_id`
/// 覆盖 `Book.num/Book.id`（`parse_book_info` 返回 0，仅用 `book_id` 作错误信息）。
/// 查询 DB 已有章节数计算起始页 `start_page`（增量爬取），follow `start_page..=book.page`
/// 到 `chapter` 回调。
pub fn book_detail_handler(
    root_url: String,
    db: Arc<Mutex<Database>>,
) -> impl Fn(Response) -> Pin<Box<dyn Future<Output = (Vec<Value>, Vec<Request>)> + Send + 'static>>
    + Send + Sync + 'static {
    move |resp| {
        let root_url = root_url.clone();
        let db = db.clone();
        let meta = resp.request.meta.clone();
        Box::pin(async move {
            let book_id = meta.get("book_id").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
            let book_num = meta.get("book_num").and_then(|v| v.as_u64()).unwrap_or(0) as usize;

            let html_str = match resp.text() {
                Ok(s) => s,
                Err(e) => {
                    log::warn!("book_detail_handler: 读取响应失败 (book_id={book_id}): {e}");
                    return (Vec::new(), Vec::new());
                }
            };

            let mut book = match parse::parse_book_info(book_id, &html_str) {
                Ok(b) => b,
                Err(e) => {
                    log::warn!("book_detail_handler: parse_book_info 失败 (book_id={book_id}): {e}");
                    return (Vec::new(), Vec::new());
                }
            };
            // parse_book_info 返回 num=0/id=0，用 meta 覆盖
            book.num = book_num;
            book.id = book_id;

            // 增量查询：已有章节数 → 起始页（每页 10 章）
            let existing_chapters = {
                let db = db.lock().await;
                db.get_chapters_count_by_website_id(book_id as i64).unwrap_or(0)
            };
            let start_page: u8 = if existing_chapters > 0 {
                ((existing_chapters / 10) as u8)
                    .min(book.page.saturating_sub(1))
                    .saturating_add(1)
            } else {
                1
            };

            // follow 章节分页 URL: {root_url}/{num}/{id}_{page}/
            let mut follows = Vec::new();
            for page in start_page..=book.page {
                let page_url = format!("{}/{}/{}_{}/", root_url, book.num, book.id, page);
                let req = Request::get(&page_url)
                    .with_callback("chapter")
                    .with_meta(json!({
                        "book_id": book_id,
                        "book_num": book.num,
                        "title": book.title,
                        "author": book.author,
                    }));
                follows.push(req);
            }

            let book_item = json!({
                "type": "book",
                "website_book_id": book_id,
                "path_num": book.num as i64,
                "title": book.title,
                "filename": book.filename,
                "author": book.author,
                "category": book.category,
                "introduce": book.introduce,
                "likes": book.likes as i64,
                "word_count": book.count as i64,
                "page_count": book.page as i64,
            });
            (vec![book_item], follows)
        })
    }
}

/// `on("chapter")`：章节分页 → emit Chapter items + follow 各章节 URL 到 `section`。
///
/// 调用 `parse::parse_chapter_list(html, root_url)` 取第二个 `.chapter-list` 下的
/// `.bd .list li a`，去重保序。每个章节 emit 一个 chapter_item（chapter_order 从 1 起），
/// 并 follow `ch.url` 到 `section` 回调，meta 携带 `book_id/chapter_order/chapter_title`。
pub fn chapter_handler(
    root_url: String,
) -> impl Fn(Response) -> Pin<Box<dyn Future<Output = (Vec<Value>, Vec<Request>)> + Send + 'static>>
    + Send + Sync + 'static {
    move |resp| {
        let root_url = root_url.clone();
        let meta = resp.request.meta.clone();
        Box::pin(async move {
            let book_id = meta.get("book_id").and_then(|v| v.as_u64()).unwrap_or(0);

            let html_str = match resp.text() {
                Ok(s) => s,
                Err(e) => {
                    log::warn!("chapter_handler: 读取响应失败 (book_id={book_id}): {e}");
                    return (Vec::new(), Vec::new());
                }
            };

            let chapters = match parse::parse_chapter_list(&html_str, &root_url) {
                Ok(c) => c,
                Err(e) => {
                    log::warn!("chapter_handler: parse_chapter_list 失败 (book_id={book_id}): {e}");
                    return (Vec::new(), Vec::new());
                }
            };

            let mut items = Vec::with_capacity(chapters.len());
            let mut follows = Vec::with_capacity(chapters.len());
            for (idx, ch) in chapters.iter().enumerate() {
                let chapter_order = (idx + 1) as i64;
                items.push(json!({
                    "type": "chapter",
                    "website_book_id": book_id,
                    "title": ch.title,
                    "url": ch.url,
                    "chapter_order": chapter_order,
                }));
                let req = Request::get(&ch.url)
                    .with_callback("section")
                    .with_meta(json!({
                        "book_id": book_id,
                        "chapter_order": chapter_order,
                        "chapter_title": ch.title,
                    }));
                follows.push(req);
            }
            (items, follows)
        })
    }
}

/// `on("section")`：章节页 → 4 策略尝试 → emit Section item 或 follow `section_post`。
///
/// 若 `needs_section_post(html)` 为真，POST `j=1` 到 `section_post` 回调
/// （Content-Type: application/x-www-form-urlencoded）；
/// 否则按 1/3/4 顺序尝试解密策略，emit 单个 Section item（section_order=1）。
pub fn section_handler(
    img_dict: Arc<HashMap<String, String>>,
    font_dict: Arc<HashMap<String, String>>,
) -> impl Fn(Response) -> Pin<Box<dyn Future<Output = (Vec<Value>, Vec<Request>)> + Send + 'static>>
    + Send + Sync + 'static {
    move |resp| {
        let img_dict = img_dict.clone();
        let font_dict = font_dict.clone();
        let url = resp.url.clone();
        let meta = resp.request.meta.clone();
        Box::pin(async move {
            let html_str = match resp.text() {
                Ok(s) => s,
                Err(e) => {
                    log::warn!("section_handler: 读取响应失败 (url={url}): {e}");
                    return (Vec::new(), Vec::new());
                }
            };
            let book_id = meta.get("book_id").and_then(|v| v.as_u64()).unwrap_or(0);
            let chapter_order = meta
                .get("chapter_order")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);

            // 策略 2 检测：POST form 拉取正文
            if parse::needs_section_post(&html_str) {
                let post_req = Request::post(&url, Some("j=1".to_string()))
                    .with_callback("section_post")
                    .with_meta(meta.clone())
                    .with_header("Content-Type", "application/x-www-form-urlencoded");
                return (Vec::new(), vec![post_req]);
            }

            // 策略 1/3/4 直接处理
            let content = if let Ok(c) = parse::try_section_data1(&html_str, &font_dict, &img_dict) {
                c
            } else if let Ok(c) = parse::try_section_data3(&html_str) {
                c
            } else if let Ok(c) = parse::try_section_data4(&html_str) {
                c
            } else {
                String::new()
            };

            let section_item = json!({
                "type": "section",
                "website_book_id": book_id,
                "chapter_order": chapter_order,
                "section_order": 1,
                "url": url,
                "content": content,
            });
            (vec![section_item], Vec::new())
        })
    }
}

/// `on("section_post")`：POST 响应 → format_content_html 解密 → emit Section item。
///
/// POST 响应体为 HTML，调用 `parse::format_content_html(Some(html), None, font, img)`
/// 进行字体/图片反爬还原。meta 与 `section_handler` 相同（book_id/chapter_order）。
pub fn section_post_handler(
    img_dict: Arc<HashMap<String, String>>,
    font_dict: Arc<HashMap<String, String>>,
) -> impl Fn(Response) -> Pin<Box<dyn Future<Output = (Vec<Value>, Vec<Request>)> + Send + 'static>>
    + Send + Sync + 'static {
    move |resp| {
        let img_dict = img_dict.clone();
        let font_dict = font_dict.clone();
        let url = resp.url.clone();
        let meta = resp.request.meta.clone();
        Box::pin(async move {
            let html_str = match resp.text() {
                Ok(s) => s,
                Err(e) => {
                    log::warn!("section_post_handler: 读取响应失败 (url={url}): {e}");
                    return (Vec::new(), Vec::new());
                }
            };
            let book_id = meta.get("book_id").and_then(|v| v.as_u64()).unwrap_or(0);
            let chapter_order = meta
                .get("chapter_order")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);

            let content =
                parse::format_content_html(Some(&html_str), None, &font_dict, &img_dict)
                    .unwrap_or_default();

            let section_item = json!({
                "type": "section",
                "website_book_id": book_id,
                "chapter_order": chapter_order,
                "section_order": 1,
                "url": url,
                "content": content,
            });
            (vec![section_item], Vec::new())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::time::Duration;
    use wisp::crawl::stop::{StopCondition, StopContext};
    use wisp::fetcher::Request;

    fn make_list_response(html: &str, url: &str) -> Response {
        Response::from_http(
            200,
            url.to_string(),
            HashMap::new(),
            html.as_bytes().to_vec(),
            "text/html; charset=utf-8".to_string(),
            Request::get(url),
        )
    }

    fn stop_ctx() -> StopContext {
        StopContext {
            pages: 0,
            items: 0,
            errors: 0,
            in_flight: 0,
            elapsed: Duration::from_secs(0),
            queue_size: 0,
        }
    }

    #[tokio::test]
    async fn test_list_handler_extracts_books_and_follows() {
        let html = r#"<html><body>
            <ul>
                <li class="column-2"><a class="name" href="/12/12345/">书名A</a></li>
                <li class="column-2"><a class="name" href="/13/12346/">书名B</a></li>
                <li class="column-2"><a class="name" href="/bad/">空标题</a></li>
                <li class="column-2"><a class="name" href="/x/0/">无效ID</a></li>
            </ul>
        </body></html>"#;
        let url = "https://www.bz.com/all/1.html";
        let resp = make_list_response(html, url);

        let tracker = EmptyPageTracker::new(3);
        let handler = list_handler(tracker.clone());
        let (items, follows) = handler(resp).await;

        assert!(items.is_empty(), "list_handler 不应 emit items");
        assert_eq!(follows.len(), 2, "应跳过空标题和 book_id=0 的项");
        assert_eq!(follows[0].callback, Some("book_detail".to_string()));
        assert_eq!(follows[0].url, "https://www.bz.com/12/12345/");
        assert_eq!(follows[0].meta["book_id"], 12345);
        assert_eq!(follows[0].meta["book_num"], 12);
        assert_eq!(follows[0].meta["title"], "书名A");
        assert_eq!(follows[1].url, "https://www.bz.com/13/12346/");
        assert_eq!(follows[1].meta["book_id"], 12346);
        // 非空页应重置 streak（不应触发停止）
        assert!(!tracker.should_stop(&stop_ctx()));
    }

    #[tokio::test]
    async fn test_list_handler_empty_page_records_empty() {
        let html = "<html><body><p>无列表</p></body></html>";
        let url = "https://www.bz.com/all/1.html";
        let resp = make_list_response(html, url);

        let tracker = EmptyPageTracker::new(3);
        let handler = list_handler(tracker.clone());
        let (_items, follows) = handler(resp).await;

        assert!(follows.is_empty(), "空页不应产生 follows");
        // 1 次空页 + max_streak=3，不应停止
        assert!(!tracker.should_stop(&stop_ctx()));
    }

    #[tokio::test]
    async fn test_section_handler_strategy1_emits_item() {
        let html = r#"<html><body>
            <div class="page-content"><p>正文段落。<br>第二行。</p></div>
        </body></html>"#;
        let url = "https://www.bz.com/12/12345_1/23456.html";
        let resp = Response::from_http(
            200,
            url.to_string(),
            HashMap::new(),
            html.as_bytes().to_vec(),
            "text/html; charset=utf-8".to_string(),
            Request::get(url).with_meta(json!({
                "book_id": 12345,
                "chapter_order": 1,
                "chapter_title": "第1章",
            })),
        );

        let img: Arc<HashMap<String, String>> = Arc::new(HashMap::new());
        let font: Arc<HashMap<String, String>> = Arc::new(HashMap::new());
        let handler = section_handler(img, font);
        let (items, follows) = handler(resp).await;

        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["type"], "section");
        assert_eq!(items[0]["website_book_id"], 12345);
        assert_eq!(items[0]["chapter_order"], 1);
        assert_eq!(items[0]["section_order"], 1);
        assert_eq!(items[0]["url"], url);
        assert!(follows.is_empty(), "策略1不应产生 follows");
    }

    #[tokio::test]
    async fn test_section_handler_needs_post_follows_to_section_post() {
        let html = r#"<html><body>
            <script>$.post('',{'j':'1'},function(e){})</script>
        </body></html>"#;
        let url = "https://www.bz.com/12/12345_1/23456.html";
        let resp = Response::from_http(
            200,
            url.to_string(),
            HashMap::new(),
            html.as_bytes().to_vec(),
            "text/html; charset=utf-8".to_string(),
            Request::get(url).with_meta(json!({
                "book_id": 12345,
                "chapter_order": 1,
            })),
        );

        let img: Arc<HashMap<String, String>> = Arc::new(HashMap::new());
        let font: Arc<HashMap<String, String>> = Arc::new(HashMap::new());
        let handler = section_handler(img, font);
        let (items, follows) = handler(resp).await;

        assert!(items.is_empty(), "需要 POST 时不应 emit item");
        assert_eq!(follows.len(), 1);
        assert_eq!(follows[0].callback, Some("section_post".to_string()));
        assert_eq!(follows[0].url, url);
        assert_eq!(follows[0].method, wisp::fetcher::Method::Post);
        assert_eq!(follows[0].body.as_deref(), Some("j=1"));
        assert_eq!(
            follows[0].headers.get("Content-Type"),
            Some(&"application/x-www-form-urlencoded".to_string())
        );
        assert_eq!(follows[0].meta["book_id"], 12345);
    }
}
