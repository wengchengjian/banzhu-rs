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
use wisp::crawl::Page;
use wisp::fetcher::CrawlRequest;

/// 小说正文（章节页）的调度优先级。
///
/// wisp 默认 follow 优先级随深度递减（BFS：浅层列表页优先）。小说场景相反：
/// 正文内容是最关键数据，须优先于新列表页/详情页抓取，故设置高优先级。
const SECTION_PRIORITY: i32 = 100;
/// 章节分页（`_N.html`）的优先级：比正文首页更高，确保多页内容被完整抓取。
const SECTION_PAGE_PRIORITY: i32 = 200;

/// 内容链优先级：让新书持续流入，避免「爆发→空转→爆发」锯齿。
///
/// 原默认 follow 优先级随深度递减（BFS），导致内容链的入口环节（详情页 -1、
/// 章节分页 -2）反而比列表页(0) 低：正文(100/200) 跑完后队列空转，等低优先级
/// 的章节分页出队才产生新正文 → 吞吐锯齿。统一给内容链入口设置显式高优先级：
///   BOOK_DETAIL(50) < CHAPTER_PAGE(150) < SECTION(100) < SECTION_PAGE(200)
/// 详情页/章节分页均高于列表页(0)，保证整条内容链持续填满并发。
const BOOK_DETAIL_PRIORITY: i32 = 50;
/// 章节分页（`{num}/{id}_{pg}/`）的优先级：高于正文首页，确保章节 URL 尽快产出。
const CHAPTER_PAGE_PRIORITY: i32 = 150;

/// `on_page("default")`：列表页 → 解析书籍 → follow 到 `book_detail`。
///
/// 解析 `li.column-2 > a.name`，从 href 中拆出 `book_num/book_id`，
/// 携带 meta（book_id/book_num/title/author）follow 到 `book_detail`。
/// 空页/非空页通过 `EmptyPageTracker` 共享原子计数器上报（用于 `until` 终止条件）。
///
/// 采用 wisp 异步 `on_page!` handler：接收 `Page`、返回 `Page`，内部可 `.await`。
pub fn list_handler(
    tracker: EmptyPageTracker,
) -> impl Fn(Page) -> Pin<Box<dyn Future<Output = Page> + Send + 'static>> + Send + Sync + 'static
{
    move |mut page| {
        let tracker = tracker.clone();
        Box::pin(async move {
            let url_for_log = page.url().to_string();
            let mut found_any = false;
            let mut follows_count = 0usize;
            for li in page.css("li.column-2").iter() {
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
                follows_count += 1;
                page.follow_meta_with_priority(
                    &href,
                    "book_detail",
                    json!({
                        "book_id": book_id,
                        "book_num": book_num,
                        "title": title,
                        "author": "",
                    }),
                    BOOK_DETAIL_PRIORITY,
                );
            }
            if found_any {
                log::debug!("list_handler: 列表页 {} 解析出 {} 本书，follow 到 book_detail", url_for_log, follows_count);
                tracker.record_non_empty();
            } else {
                // 诊断：打印页面 title 和 body 前 400 字符，定位解析失败原因
                let title = page
                    .select_one("title")
                    .map(|t| t.text())
                    .unwrap_or_default();
                let body_preview = page
                    .select_one("body")
                    .map(|b| {
                        let t = b.text();
                        t.chars().take(400).collect::<String>()
                    })
                    .unwrap_or_default();
                log::warn!(
                    "list_handler 解析为空: url={}, title='{}', body前400字符={:?}",
                    url_for_log,
                    title.trim(),
                    body_preview
                );
                tracker.record_empty();
            }
            page
        })
    }
}

/// `on_page("book_detail")`：书籍详情页 → emit Book item + follow 章节分页。
///
/// 调用 `parse::parse_book_info` 提取元数据；用 meta 中的 `book_num/book_id`
/// 覆盖 `Book.num/Book.id`（`parse_book_info` 返回 0，仅用 `book_id` 作错误信息）。
/// 查询 DB 已有章节数计算起始页 `start_page`（增量爬取），follow `start_page..=book.page`
/// 到 `chapter` 回调。
///
/// 采用 wisp 异步 `on_page!` handler：handler 内可直接 `.await` DB。
pub fn book_detail_handler(
    root_url: String,
    db: Arc<Mutex<Database>>,
) -> impl Fn(Page) -> Pin<Box<dyn Future<Output = Page> + Send + 'static>> + Send + Sync + 'static
{
    move |mut page| {
        let root_url = root_url.clone();
        let db = db.clone();
        Box::pin(async move {
            let book_id = page.meta_u64("book_id") as usize;
            let book_num = page.meta_u64("book_num") as usize;

            let html_str = match page.resp().text() {
                Ok(s) => s,
                Err(e) => {
                    log::warn!("book_detail_handler: 读取响应失败 (book_id={book_id}): {e}");
                    return page;
                }
            };

            let mut book = match parse::parse_book_info(book_id, &html_str) {
                Ok(b) => b,
                Err(e) => {
                    log::warn!("book_detail_handler: parse_book_info 失败 (book_id={book_id}): {e}");
                    return page;
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
            log::debug!(
                "book_detail_handler: 书 {} '{}' 已有 {} 章, 起始分页 {}..={} (共 {} 页), 增量爬取",
                book_id, book.title, existing_chapters, start_page, book.page,
                book.page.saturating_sub(start_page).saturating_add(1)
            );

            // follow 章节分页 URL: {root_url}/{num}/{id}_{page}/
            for pg in start_page..=book.page {
                let page_url = format!("{}/{}/{}_{}/", root_url, book.num, book.id, pg);
                page.follow_meta_with_priority(
                    &page_url,
                    "chapter",
                    json!({
                        "book_id": book_id,
                        "book_num": book.num,
                        "title": book.title,
                        "author": book.author,
                    }),
                    CHAPTER_PAGE_PRIORITY,
                );
            }

            page.item(json!({
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
            }));
            page
        })
    }
}

/// `on_page("chapter")`：章节分页 → emit Chapter items + follow 各章节 URL 到 `section`。
///
/// 调用 `parse::parse_chapter_list(html, root_url)` 取第二个 `.chapter-list` 下的
/// `.bd .list li a`，去重保序。每个章节 emit 一个 chapter_item（chapter_order 从 1 起），
/// 并 follow `ch.url` 到 `section` 回调，meta 携带 `book_id/chapter_order/chapter_title`。
///
/// 采用 wisp 异步 `on_page!` handler：接收 `Page`、返回 `Page`。
pub fn chapter_handler(
    root_url: String,
) -> impl Fn(Page) -> Pin<Box<dyn Future<Output = Page> + Send + 'static>> + Send + Sync + 'static
{
    move |mut page| {
        let root_url = root_url.clone();
        Box::pin(async move {
            let book_id = page.meta_u64("book_id");

            let html_str = match page.resp().text() {
                Ok(s) => s,
                Err(e) => {
                    log::warn!("chapter_handler: 读取响应失败 (book_id={book_id}): {e}");
                    return page;
                }
            };

            let chapters = match parse::parse_chapter_list(&html_str, &root_url) {
                Ok(c) => c,
                Err(e) => {
                    log::warn!("chapter_handler: parse_chapter_list 失败 (book_id={book_id}): {e}");
                    return page;
                }
            };

            log::debug!(
                "chapter_handler: 书 {} 解析出 {} 个章节，emit+follow section",
                book_id,
                chapters.len()
            );
            for (idx, ch) in chapters.iter().enumerate() {
                let chapter_order = (idx + 1) as i64;
                page.item(json!({
                    "type": "chapter",
                    "website_book_id": book_id,
                    "title": ch.title,
                    "url": ch.url,
                    "chapter_order": chapter_order,
                }));
                // 小说正文最高优先级：优先于新列表页/详情页/章节分页，确保内容先抓完
                page.follow_meta_with_priority(
                    &ch.url,
                    "section",
                    json!({
                        "book_id": book_id,
                        "chapter_order": chapter_order,
                        "chapter_title": ch.title,
                    }),
                    SECTION_PRIORITY,
                );
            }
            page
        })
    }
}

/// `on_page("section")`：章节页 → 4 策略尝试 → emit Section item 或 follow `section_post`。
///
/// 若 `needs_section_post(html)` 为真，POST `j=1` 到 `section_post` 回调
/// （Content-Type: application/x-www-form-urlencoded）；
/// 否则按 1/3/4 顺序尝试解密策略，emit 单个 Section item（section_order=1）。
///
/// 采用 wisp 异步 `on_page!` handler：接收 `Page`、返回 `Page`。
pub fn section_handler(
    img_dict: Arc<HashMap<String, String>>,
    font_dict: Arc<HashMap<String, String>>,
) -> impl Fn(Page) -> Pin<Box<dyn Future<Output = Page> + Send + 'static>> + Send + Sync + 'static
{
    move |mut page| {
        let img_dict = img_dict.clone();
        let font_dict = font_dict.clone();
        Box::pin(async move {
            let url = page.url().to_string();
            let html_str = match page.resp().text() {
                Ok(s) => s,
                Err(e) => {
                    log::warn!("section_handler: 读取响应失败 (url={url}): {e}");
                    return page;
                }
            };
            let book_id = page.meta_u64("book_id");
            let chapter_order = page.meta().get("chapter_order").and_then(Value::as_i64).unwrap_or(0);
            // 当前页号：分页 follow 时由 meta 带入，首次为 1
            let section_order = page.meta().get("section_order").and_then(Value::as_i64).unwrap_or(1);

            // 首次进入（干净章节 URL）时检测多页：若 .chapterPages 分页数 >1，
            // emit 当前页（section_order=1）并 follow 其余分页（_2.html ~ _N.html）。
            // 分页 URL 再次进入本回调时从 meta 读 section_order，只解密 emit 当前页。
            if section_order == 1 && !url.ends_with("_1.html") {
                if let Ok(sections) = parse::parse_section_urls(&url, &html_str) {
                    if sections.len() > 1 {
                        log::debug!(
                            "section_handler: 书 {} 第 {} 章检测到 {} 个分页，follow 其余 {} 个",
                            book_id, chapter_order, sections.len(), sections.len() - 1
                        );
                        let meta_base = page.meta().clone();
                        for (idx, sec) in sections.iter().enumerate() {
                            let page_no = (idx + 1) as i64;
                            if page_no == 1 {
                                continue; // 第 1 页即当前页，本函数直接处理
                            }
                            let mut meta = meta_base.clone();
                            meta["section_order"] = json!(page_no);
                            log::debug!(
                                "section_handler: 书 {} 第 {} 章 follow 分页 {} (page {})",
                                book_id, chapter_order, sec.url, page_no
                            );
                            // 分页比正文首页优先级更高，确保多页内容优先被完整抓取
                            page.follow_meta_with_priority(
                                &sec.url,
                                "section",
                                meta,
                                SECTION_PAGE_PRIORITY,
                            );
                        }
                    }
                }
            }

            // 策略 2 检测：POST form 拉取正文
            if parse::needs_section_post(&html_str) {
                log::debug!("section_handler: 书 {} 第 {} 章(页{})命中策略2(POST)，follow section_post: {}", book_id, chapter_order, section_order, url);
                let meta = page.meta().clone();
                let post_req = CrawlRequest::post(&url, Some("j=1".to_string()))
                    .with_callback("section_post")
                    .with_meta(meta)
                    .with_header("Content-Type", "application/x-www-form-urlencoded");
                page.follow_request(post_req);
                return page;
            }

            // 策略 1/3/4 直接处理（带命中策略日志，便于排查反爬）
            let content = if let Ok(c) = parse::try_section_data1(&html_str, &font_dict, &img_dict) {
                log::debug!("section_handler: 书 {} 第 {} 章(页{})策略1(字体/图片)解密成功: {}", book_id, chapter_order, section_order, url);
                c
            } else if let Ok(c) = parse::try_section_data3(&html_str) {
                log::debug!("section_handler: 书 {} 第 {} 章(页{})策略3(RC4)解密成功: {}", book_id, chapter_order, section_order, url);
                c
            } else if let Ok(c) = parse::try_section_data4(&html_str) {
                log::debug!("section_handler: 书 {} 第 {} 章(页{})策略4(AES)解密成功: {}", book_id, chapter_order, section_order, url);
                c
            } else {
                log::warn!("section_handler: 书 {} 第 {} 章(页{})全部策略解密失败: {}", book_id, chapter_order, section_order, url);
                String::new()
            };

            page.item(json!({
                "type": "section",
                "website_book_id": book_id,
                "chapter_order": chapter_order,
                "section_order": section_order,
                "url": url,
                "content": content,
            }));
            page
        })
    }
}

/// `on_page("section_post")`：POST 响应 → format_content_html 解密 → emit Section item。
///
/// POST 响应体为 HTML，调用 `parse::format_content_html(Some(html), None, font, img)`
/// 进行字体/图片反爬还原。meta 与 `section_handler` 相同（book_id/chapter_order）。
///
/// 采用 wisp 异步 `on_page!` handler：接收 `Page`、返回 `Page`。
pub fn section_post_handler(
    img_dict: Arc<HashMap<String, String>>,
    font_dict: Arc<HashMap<String, String>>,
) -> impl Fn(Page) -> Pin<Box<dyn Future<Output = Page> + Send + 'static>> + Send + Sync + 'static
{
    move |mut page| {
        let img_dict = img_dict.clone();
        let font_dict = font_dict.clone();
        Box::pin(async move {
            let url = page.url().to_string();
            let html_str = match page.resp().text() {
                Ok(s) => s,
                Err(e) => {
                    log::warn!("section_post_handler: 读取响应失败 (url={url}): {e}");
                    return page;
                }
            };
            let book_id = page.meta_u64("book_id");
            let chapter_order = page.meta().get("chapter_order").and_then(Value::as_i64).unwrap_or(0);
            // 分页 follow 时 section_order 由 meta 带入，POST 页可能是第 2+ 页
            let section_order = page.meta().get("section_order").and_then(Value::as_i64).unwrap_or(1);

            let content =
                parse::format_content_html(Some(&html_str), None, &font_dict, &img_dict)
                    .unwrap_or_default();
            log::debug!(
                "section_post_handler: 书 {} 第 {} 章(页{}) POST 正文解密完成, content_len={}",
                book_id, chapter_order, section_order, content.len()
            );

            page.item(json!({
                "type": "section",
                "website_book_id": book_id,
                "chapter_order": chapter_order,
                "section_order": section_order,
                "url": url,
                "content": content,
            }));
            page
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::time::Duration;
    use wisp::crawl::stop::{StopCondition, StopContext};
    use wisp::fetcher::{Method, Request, Response};

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

    /// 包装：用 `Page::new` 包 Response，调用异步 Page handler 并取回 (items, follows)。
    async fn run_page<F, Fut>(handler: F, resp: Response) -> (Vec<Value>, Vec<CrawlRequest>)
    where
        F: Fn(Page) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Page> + Send + 'static,
    {
        handler(Page::new(resp)).await.finish()
    }

    fn stop_ctx() -> StopContext {
        StopContext {
            pages: 0,
            items: 0,
            errors: 0,
            in_flight: 0,
            elapsed: Duration::from_secs(0),
            queue_size: 0,
            callback_pages: HashMap::new(),
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
        let (items, follows) = run_page(handler, resp).await;

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
        let (_items, follows) = run_page(handler, resp).await;

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
        let mut resp = Response::from_http(
            200,
            url.to_string(),
            HashMap::new(),
            html.as_bytes().to_vec(),
            "text/html; charset=utf-8".to_string(),
            Request::get(url),
        );
        resp.request = CrawlRequest::get(url).with_meta(json!({
            "book_id": 12345,
            "chapter_order": 1,
            "chapter_title": "第1章",
        }));

        let img: Arc<HashMap<String, String>> = Arc::new(HashMap::new());
        let font: Arc<HashMap<String, String>> = Arc::new(HashMap::new());
        let handler = section_handler(img, font);
        let (items, follows) = run_page(handler, resp).await;

        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["type"], "section");
        assert_eq!(items[0]["website_book_id"], 12345);
        assert_eq!(items[0]["chapter_order"], 1);
        assert_eq!(items[0]["section_order"], 1);
        assert_eq!(items[0]["url"], url);
        assert!(follows.is_empty(), "策略1不应产生 follows");
    }

    #[tokio::test]
    async fn test_section_handler_multipage_follows_other_pages() {
        // 章节页含 3 个分页链接（【1】【2】【3】），首 URL 为干净格式 9041.html
        let html = r#"<html><body>
            <div class="page-content"><p>第一页正文。</p></div>
            <center class="chapterPages">
                <a href="9041_1.html" class="curr">【1】</a>
                <a href="9041_2.html">【2】</a>
                <a href="9041_3.html">【3】</a>
            </center>
        </body></html>"#;
        let url = "https://www.bz.com/53/53510/9041.html";
        let mut resp = Response::from_http(
            200,
            url.to_string(),
            HashMap::new(),
            html.as_bytes().to_vec(),
            "text/html; charset=utf-8".to_string(),
            Request::get(url),
        );
        resp.request = CrawlRequest::get(url).with_meta(json!({
            "book_id": 53510,
            "chapter_order": 3,
        }));

        let img: Arc<HashMap<String, String>> = Arc::new(HashMap::new());
        let font: Arc<HashMap<String, String>> = Arc::new(HashMap::new());
        let handler = section_handler(img, font);
        let (items, follows) = run_page(handler, resp).await;

        // 当前页 emit section_order=1
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["section_order"], 1);
        // follow 第 2、3 页
        assert_eq!(follows.len(), 2, "应 follow 其余 2 个分页");
        let follow_urls: Vec<&str> = follows.iter().map(|r| r.request.url.as_str()).collect();
        assert!(follow_urls.contains(&"https://www.bz.com/53/53510/9041_2.html"));
        assert!(follow_urls.contains(&"https://www.bz.com/53/53510/9041_3.html"));
        // follow 的 meta 带 section_order
        for r in &follows {
            let so = r.meta.get("section_order").and_then(Value::as_i64).unwrap_or(0);
            assert!(so >= 2, "分页 follow 应携带 section_order>=2");
        }
    }

    #[tokio::test]
    async fn test_section_handler_single_page_no_follows() {
        // 单页章节（无 .chapterPages 分页）不应 follow
        let html = r#"<html><body>
            <div class="page-content"><p>唯一正文。</p></div>
        </body></html>"#;
        let url = "https://www.bz.com/12/12345/100.html";
        let mut resp = Response::from_http(
            200,
            url.to_string(),
            HashMap::new(),
            html.as_bytes().to_vec(),
            "text/html; charset=utf-8".to_string(),
            Request::get(url),
        );
        resp.request = CrawlRequest::get(url).with_meta(json!({
            "book_id": 12345,
            "chapter_order": 1,
        }));

        let img: Arc<HashMap<String, String>> = Arc::new(HashMap::new());
        let font: Arc<HashMap<String, String>> = Arc::new(HashMap::new());
        let handler = section_handler(img, font);
        let (items, follows) = run_page(handler, resp).await;

        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["section_order"], 1);
        assert!(follows.is_empty(), "单页章节不应 follow");
    }

    #[tokio::test]
    async fn test_section_handler_multipage_follow_emits_correct_order() {
        // 分页 URL（_2.html）再次进入 section_handler 时，应从 meta 读 section_order=2
        let html = r#"<html><body>
            <div class="page-content"><p>第二页正文。</p></div>
            <center class="chapterPages">
                <a href="9041_1.html">【1】</a>
                <a href="9041_2.html" class="curr">【2】</a>
                <a href="9041_3.html">【3】</a>
            </center>
        </body></html>"#;
        let url = "https://www.bz.com/53/53510/9041_2.html";
        let mut resp = Response::from_http(
            200,
            url.to_string(),
            HashMap::new(),
            html.as_bytes().to_vec(),
            "text/html; charset=utf-8".to_string(),
            Request::get(url),
        );
        resp.request = CrawlRequest::get(url).with_meta(json!({
            "book_id": 53510,
            "chapter_order": 3,
            "section_order": 2,
        }));

        let img: Arc<HashMap<String, String>> = Arc::new(HashMap::new());
        let font: Arc<HashMap<String, String>> = Arc::new(HashMap::new());
        let handler = section_handler(img, font);
        let (items, follows) = run_page(handler, resp).await;

        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["section_order"], 2, "分页页应从 meta 读 section_order");
        assert_eq!(items[0]["url"], url);
        assert!(follows.is_empty(), "分页页不应再次 follow");
    }

    #[tokio::test]
    async fn test_section_handler_needs_post_follows_to_section_post() {
        let html = r#"<html><body>
            <script>$.post('',{'j':'1'},function(e){})</script>
        </body></html>"#;
        let url = "https://www.bz.com/12/12345_1/23456.html";
        let mut resp = Response::from_http(
            200,
            url.to_string(),
            HashMap::new(),
            html.as_bytes().to_vec(),
            "text/html; charset=utf-8".to_string(),
            Request::get(url),
        );
        resp.request = CrawlRequest::get(url).with_meta(json!({
            "book_id": 12345,
            "chapter_order": 1,
        }));

        let img: Arc<HashMap<String, String>> = Arc::new(HashMap::new());
        let font: Arc<HashMap<String, String>> = Arc::new(HashMap::new());
        let handler = section_handler(img, font);
        let (items, follows) = run_page(handler, resp).await;

        assert!(items.is_empty(), "需要 POST 时不应 emit item");
        assert_eq!(follows.len(), 1);
        assert_eq!(follows[0].callback, Some("section_post".to_string()));
        assert_eq!(follows[0].url, url);
        assert_eq!(follows[0].method, Method::Post);
        assert_eq!(follows[0].body.as_deref(), Some("j=1"));
        assert_eq!(
            follows[0].headers.get("Content-Type"),
            Some(&"application/x-www-form-urlencoded".to_string())
        );
        assert_eq!(follows[0].meta["book_id"], 12345);
    }
}
