//! wisp 驱动的 banzhu 爬虫模块。
//!
//! 取代旧的 banzhuspider + task + cf 三模块。

pub mod callbacks;
pub mod parse;
pub mod pipeline;
pub mod stop;

use std::collections::HashMap;
use std::sync::Arc;

use config::Config;
use tokio::sync::Mutex;
use wisp::crawl::middleware::{HeadersMiddleware, UaRotationMiddleware};
use wisp::crawl::{ClosureSpider, SpiderBuilder};

use crate::db::Database;
use crate::event::EventBus;
use crate::scheduler::CrawlStatus;

const IMAGE_FANPA_FILE: &str = include_str!("../../asset/txt/变形字体库v2.txt");
const FONT_FANPA_FILE: &str = include_str!("../../asset/txt/字体反爬库.txt");

/// 初始化图片反爬字典（迁移自 banzhuspider.rs::init_img_fanpa_dict）
pub fn init_img_fanpa_dict() -> HashMap<String, String> {
    let mut dict = HashMap::new();
    for line in IMAGE_FANPA_FILE.split('\n') {
        if let Some((word, url)) = line.split_once(' ') {
            dict.insert(url.trim().to_string(), word.trim().to_string());
        }
    }
    dict
}

/// 初始化字体反爬字典（迁移自 banzhuspider.rs::init_font_fanpa_dict）
pub fn init_font_fanpa_dict() -> HashMap<String, String> {
    let mut dict = HashMap::new();
    for line in FONT_FANPA_FILE.split('\n') {
        if let Some((key, val)) = line.split_once('\t') {
            dict.insert(key.trim().to_string(), val.trim().to_string());
        }
    }
    dict
}

/// 构造 banzhu ClosureSpider：组装 5 个 callback + UA/Headers 中间件 + 写 DB 管道 + 空页终止条件。
#[expect(clippy::too_many_arguments, reason = "brief 指定的组装函数签名")]
pub fn build_spider(
    root_url: String,
    pages_limit: u32,
    db: Arc<Mutex<Database>>,
    _config: Arc<Config>,
    event_bus: EventBus,
    status: Arc<Mutex<CrawlStatus>>,
    img_dict: Arc<HashMap<String, String>>,
    font_dict: Arc<HashMap<String, String>>,
) -> ClosureSpider {
    let start_urls: Vec<String> = (1..=pages_limit)
        .map(|p| format!("{}/shuku/0-lastupdate-0-{}.html", root_url, p))
        .collect();

    let tracker = stop::EmptyPageTracker::new(3);

    SpiderBuilder::new("banzhu")
        .start_urls(start_urls)
        .middleware(UaRotationMiddleware::desktop())
        .middleware(HeadersMiddleware::new(vec![
            ("Accept".into(), "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8".into()),
            ("Accept-Language".into(), "zh-CN,zh;q=0.9,en;q=0.8".into()),
            ("Referer".into(), root_url.clone()),
            ("Connection".into(), "keep-alive".into()),
            ("Upgrade-Insecure-Requests".into(), "1".into()),
        ]))
        .pipeline(pipeline::build_banzhu_pipeline(db.clone(), event_bus, status))
        .on("default", callbacks::list_handler(tracker.clone()))
        .on("book_detail", callbacks::book_detail_handler(root_url.clone(), db))
        .on("chapter", callbacks::chapter_handler(root_url.clone()))
        .on("section", callbacks::section_handler(img_dict.clone(), font_dict.clone()))
        .on("section_post", callbacks::section_post_handler(img_dict.clone(), font_dict.clone()))
        .until(tracker)
        .build()
}
