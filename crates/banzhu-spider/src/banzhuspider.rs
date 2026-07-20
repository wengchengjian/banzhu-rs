use crate::cf::{is_bypassed, CfManager};
use crate::task::BanzhuDownloadTask;
use anyhow::{anyhow, Result};
use config::Config;
use wreq::Client;
use wreq_util::Emulation;
use log::{debug, warn};
use scraper::{Html, Selector};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

/// Constants for anti-crawling dictionaries
const IMAGE_FANPA_FILE: &str = include_str!("../asset/txt/变形字体库v2.txt");
const FONT_FANPA_FILE: &str = include_str!("../asset/txt/字体反爬库.txt");

/// Spider configuration
#[derive(Debug)]
pub struct SpiderConfig {
    pub max_concurrent_tasks: usize,
    pub retry_attempts: u32,
    pub retry_delay: Duration,
    pub request_timeout: Duration,
}

impl Default for SpiderConfig {
    fn default() -> Self {
        Self {
            max_concurrent_tasks: 16,
            retry_attempts: 3,
            retry_delay: Duration::from_millis(100),
            request_timeout: Duration::from_secs(5),
        }
    }
}

/// Summary of a book from the latest-update listing page
#[derive(Debug, Clone)]
pub struct BookSummary {
    pub book_id: u32,
    pub book_num: u32,
    pub title: String,
    pub author: String,
    pub category: String,
    pub word_count: u32,
    pub latest_chapter_url: String,
}

/// Main spider for web scraping
pub struct BanzhuSpider {
    url: String,
    config: Arc<Config>,
    pub spider_config: Arc<SpiderConfig>,
    /// wreq Client 本身线程安全（Clone + Send + Sync），无需 Mutex
    pub client: Client,
    /// CF cookie 生命周期管理器
    pub cf_manager: Arc<CfManager>,
    pub img_fanpa_dict: Arc<HashMap<String, String>>,
    pub font_fanpa_dict: Arc<HashMap<String, String>>,
}

/// Initialize image anti-crawling dictionary
pub fn init_img_fanpa_dict() -> HashMap<String, String> {
    let mut img_fanpa_dict = HashMap::new();
    for line in IMAGE_FANPA_FILE.split('\n') {
        if let Some((word, url)) = line.split_once(' ') {
            img_fanpa_dict.insert(url.trim().to_string(), word.trim().to_string());
        }
    }
    img_fanpa_dict
}

/// Initialize font anti-crawling dictionary
pub fn init_font_fanpa_dict() -> HashMap<String, String> {
    let mut dict = HashMap::new();
    for line in FONT_FANPA_FILE.split('\n') {
        if let Some((key, val)) = line.split_once('\t') {
            dict.insert(key.trim().to_string(), val.trim().to_string());
        }
    }
    dict
}

impl BanzhuSpider {
    pub fn new(url: String, config: Arc<Config>) -> Self {
        // 从配置读取参数
        let timeout_secs = config.get_int("spider.request_timeout_secs").unwrap_or(15) as u64;
        let retry_attempts = config.get_int("spider.retry_attempts").unwrap_or(3) as u32;
        let retry_delay_ms = config.get_int("spider.retry_delay_ms").unwrap_or(100) as u64;
        let max_concurrent = config.get_int("spider.max_concurrent_tasks").unwrap_or(16) as usize;
        let proxy_enabled = config.get_bool("spider.proxy.enabled").unwrap_or(false);
        let proxy_url = config.get_string("spider.proxy.url").unwrap_or_default();

        // wreq + Chrome137 TLS/JA4 指纹模拟
        let mut builder = Client::builder()
            .emulation(Emulation::Chrome137)
            .cookie_store(true)
            .zstd(true)
            .timeout(Duration::from_secs(timeout_secs));

        // 代理配置
        if proxy_enabled && !proxy_url.is_empty() {
            if let Ok(proxy) = wreq::Proxy::all(&proxy_url) {
                builder = builder.proxy(proxy);
                log::info!("代理已启用: {}", proxy_url);
            } else {
                log::warn!("代理 URL 无效: {}", proxy_url);
            }
        }

        let client = builder
            .build()
            .expect("Failed to create wreq client — check wreq/wreq-util versions");

        // CF 绕过配置
        let cf_ttl = Duration::from_secs(
            config.get_int("cf_bypass.cookie_ttl_secs").unwrap_or(1200) as u64,
        );
        let cf_headless = config.get_bool("cf_bypass.headless").unwrap_or(false);

        let img_fanpa_dict = init_img_fanpa_dict();
        let font_fanpa_dict = init_font_fanpa_dict();

        BanzhuSpider {
            url: url.clone(),
            config,
            client,
            cf_manager: Arc::new(CfManager::with_config(cf_ttl, cf_headless)),
            img_fanpa_dict: Arc::new(img_fanpa_dict),
            font_fanpa_dict: Arc::new(font_fanpa_dict),
            spider_config: Arc::new(SpiderConfig {
                max_concurrent_tasks: max_concurrent,
                retry_attempts,
                retry_delay: Duration::from_millis(retry_delay_ms),
                request_timeout: Duration::from_secs(timeout_secs),
            }),
        }
    }

    pub fn with_config(mut self, config: SpiderConfig) -> Self {
        self.spider_config = Arc::new(config);
        self
    }

    pub fn create_download_task(&self, book_id: u32) -> BanzhuDownloadTask {
        BanzhuDownloadTask::new(
            self.url.clone(),
            book_id,
            self.config.clone(),
            self.img_fanpa_dict.clone(),
            self.font_fanpa_dict.clone(),
            self.client.clone(),
            self.spider_config.clone(),
            self.cf_manager.clone(),
        )
    }

    /// wreq GET 请求，自动注入 CF cookie，返回 HTML 字符串
    pub async fn get(&self, url: &str) -> Result<String> {
        let mut backoff = self.spider_config.retry_delay;

        for attempt in 0..self.spider_config.retry_attempts {
            if attempt > 0 {
                debug!("Retry attempt {} for {}", attempt, url);
                sleep(backoff).await;
                backoff *= 2;
            }

            let (cookie, ua) = self
                .cf_manager
                .ensure(&self.url)
                .await
                .map_err(|e| anyhow!("CF bypass failed: {}", e))?;

            let result = self
                .client
                .get(url)
                .header("Cookie", cookie.as_str())
                .header("User-Agent", ua.as_str())
                .send()
                .await;

            match result {
                Ok(response) => {
                    if response.status().is_success() {
                        let text = response.text().await?;
                        if !text.is_empty() && is_bypassed(&text) {
                            return Ok(text);
                        }
                        // CF 验证页 → 强制刷新 cookie，下次重试使用新 cookie
                        if crate::cf::is_cf_challenge(&text) {
                            warn!("CF challenge detected for {}, refreshing cookie...", url);
                            let _ = self.cf_manager.refresh(&self.url).await;
                        }
                    }
                }
                Err(e) => {
                    debug!("请求重试 {} (attempt {}): {}", url, attempt + 1, e);
                    continue;
                }
            }
        }

        Err(anyhow!("Max retry attempts reached for {}", url))
    }

    /// 从最新列表页解析书籍摘要
    pub async fn fetch_latest_list(&self, page: u32) -> Result<Vec<BookSummary>> {
        let url = format!("{}/shuku/0-lastupdate-0-{}.html", self.url, page);
        let html_str = self.get(&url).await?;
        let html = Html::parse_document(&html_str);

        let li_sel = Selector::parse("li.column-2").map_err(|_| anyhow!("html解析失败"))?;
        let name_sel = Selector::parse("a.name").map_err(|_| anyhow!("html解析失败"))?;
        let update_sel = Selector::parse(".update a").map_err(|_| anyhow!("html解析失败"))?;
        let author_sel = Selector::parse(".info .author").map_err(|_| anyhow!("html解析失败"))?;
        let words_sel = Selector::parse(".info .words").map_err(|_| anyhow!("html解析失败"))?;

        let mut books = Vec::new();

        for li in html.select(&li_sel) {
            let (book_num, book_id) = if let Some(name_el) = li.select(&name_sel).next() {
                if let Some(href) = name_el.value().attr("href") {
                    let parts: Vec<&str> = href.trim_matches('/').split('/').collect();
                    if parts.len() >= 2 {
                        let num: u32 = parts[0].parse().unwrap_or(0);
                        let id: u32 = parts[1].parse().unwrap_or(0);
                        (num, id)
                    } else {
                        continue;
                    }
                } else {
                    continue;
                }
            } else {
                continue;
            };

            let title = li
                .select(&name_sel)
                .next()
                .and_then(|el| el.text().next())
                .unwrap_or("")
                .to_string();

            let latest_chapter_url = li
                .select(&update_sel)
                .next()
                .and_then(|el| el.value().attr("href"))
                .unwrap_or("")
                .to_string();

            let author = li
                .select(&author_sel)
                .next()
                .and_then(|el| el.text().next())
                .unwrap_or("")
                .trim()
                .to_string();

            let category = li
                .select(&author_sel)
                .next()
                .and_then(|el| el.text().next())
                .unwrap_or("")
                .trim()
                .to_string();

            let word_count: u32 = li
                .select(&words_sel)
                .next()
                .and_then(|el| el.text().next())
                .unwrap_or("0")
                .parse()
                .unwrap_or(0);

            books.push(BookSummary {
                book_id,
                book_num,
                title,
                author,
                category,
                word_count,
                latest_chapter_url,
            });
        }

        Ok(books)
    }
}

#[cfg(test)]
mod tests {
    use config::File;

    use super::*;

    #[tokio::test]
    async fn test_spider_config() {
        let spider = BanzhuSpider::new(
            "https://example.com".to_string(),
            Arc::new(
                Config::builder()
                    .add_source(File::with_name("spider.toml"))
                    .build()
                    .expect("Failed to build spider config"),
            ),
        );

        assert_eq!(spider.spider_config.max_concurrent_tasks, 16);
        assert_eq!(spider.spider_config.retry_attempts, 3);
    }
}
