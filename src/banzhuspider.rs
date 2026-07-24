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
    /// 同步构造（不探测代理可用性，直接按配置启用或禁用代理）
    pub fn new(url: String, config: Arc<Config>) -> Self {
        let proxy_enabled = config.get_bool("spider.proxy.enabled").unwrap_or(false);
        let proxy_url = config.get_string("spider.proxy.url").unwrap_or_default();
        let proxy = if proxy_enabled && !proxy_url.is_empty() {
            Some(proxy_url)
        } else {
            None
        };
        Self::build_inner(url, config, proxy)
    }

    /// 启动时探测代理可用性，不通则降级到直连（fallback）
    ///
    /// 探测策略：通过代理 HTTP GET 目标站点根路径，5 秒超时。
    /// 即使返回 403/503 也认为代理可用（说明能转发请求，CF 拦截是另一回事）。
    /// 探测失败（连接错误/超时）则降级到直连，避免代理挂了导致整个爬虫不可用。
    pub async fn new_with_probe(url: String, config: Arc<Config>) -> Self {
        let proxy_enabled = config.get_bool("spider.proxy.enabled").unwrap_or(false);
        let proxy_url = config.get_string("spider.proxy.url").unwrap_or_default();

        let effective_proxy: Option<String> = if proxy_enabled && !proxy_url.is_empty() {
            match Self::probe_proxy_http(&proxy_url, &url).await {
                true => {
                    log::info!("代理可用: {}", proxy_url);
                    Some(proxy_url)
                }
                false => {
                    log::warn!(
                        "代理 {} 探测失败，降级到直连模式（CF 绕过与 wreq 都不走代理）",
                        proxy_url
                    );
                    None
                }
            }
        } else {
            None
        };

        Self::build_inner(url, config, effective_proxy)
    }

    /// HTTP 探测代理可用性：通过代理访问目标站点，5 秒超时
    ///
    /// 返回 true 表示代理能正常转发（即使返回 403/503 也算可用），
    /// 返回 false 表示连接失败/超时（代理服务未启动或网络不通）。
    async fn probe_proxy_http(proxy_url: &str, target_root: &str) -> bool {
        let proxy = match wreq::Proxy::all(proxy_url) {
            Ok(p) => p,
            Err(e) => {
                log::debug!("代理 URL 无效 {}: {}", proxy_url, e);
                return false;
            }
        };

        let client = match Client::builder()
            .emulation(Emulation::Chrome137)
            .proxy(proxy)
            .timeout(Duration::from_secs(5))
            .build()
        {
            Ok(c) => c,
            Err(e) => {
                log::debug!("探测用 wreq client 构建失败: {}", e);
                return false;
            }
        };

        // GET 目标站点根路径。CF 可能返回 403/503，但只要能拿到响应就说明代理可用。
        match client.get(target_root).send().await {
            Ok(_) => true,
            Err(e) => {
                log::debug!("代理 HTTP 探测失败 {}: {}", proxy_url, e);
                false
            }
        }
    }

    /// 内部构造：用指定的代理（Some）或直连（None）构造 BanzhuSpider
    ///
    /// wreq client 和 CfManager 共享同一个 proxy 决策，
    /// 保证 cf_clearance 与后续请求出口 IP 一致。
    fn build_inner(url: String, config: Arc<Config>, proxy: Option<String>) -> Self {
        // 从配置读取参数
        let timeout_secs = config.get_int("spider.request_timeout_secs").unwrap_or(15) as u64;
        let retry_attempts = config.get_int("spider.retry_attempts").unwrap_or(3) as u32;
        let retry_delay_ms = config.get_int("spider.retry_delay_ms").unwrap_or(100) as u64;
        let max_concurrent = config.get_int("spider.max_concurrent_tasks").unwrap_or(16) as usize;

        // wreq + Chrome137 TLS/JA4 指纹模拟
        let mut builder = Client::builder()
            .emulation(Emulation::Chrome137)
            .cookie_store(true)
            .zstd(true)
            .timeout(Duration::from_secs(timeout_secs));

        // 代理配置（wreq）
        if let Some(ref proxy_url) = proxy {
            match wreq::Proxy::all(proxy_url) {
                Ok(p) => {
                    builder = builder.proxy(p);
                    log::info!("wreq 代理已启用: {}", proxy_url);
                }
                Err(e) => {
                    log::warn!("代理 URL 无效 {}: {}", proxy_url, e);
                }
            }
        }

        let client = builder
            .build()
            .expect("Failed to create wreq client — check wreq/wreq-util versions");

        // CF 绕过配置：与 wreq 共享 proxy，保证出口 IP 一致
        let cf_ttl = Duration::from_secs(
            config.get_int("cf_bypass.cookie_ttl_secs").unwrap_or(1200) as u64,
        );
        let cf_headless = config.get_bool("cf_bypass.headless").unwrap_or(false);
        let cf_proxy = proxy.clone();

        if cf_proxy.is_some() {
            log::info!("CF 绕过将使用代理: {}", cf_proxy.as_ref().unwrap());
        } else {
            log::info!("CF 绕过将走直连（不使用代理）");
        }

        let img_fanpa_dict = init_img_fanpa_dict();
        let font_fanpa_dict = init_font_fanpa_dict();

        BanzhuSpider {
            url: url.clone(),
            config,
            client,
            cf_manager: Arc::new(CfManager::with_config(cf_ttl, cf_headless, cf_proxy)),
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
                    let status = response.status();
                    let text = response.text().await.unwrap_or_default();

                    // 成功路径：2xx + 非 CF 挑战页
                    if status.is_success() && !text.is_empty() && is_bypassed(&text) {
                        return Ok(text);
                    }

                    // CF 验证页 → 强制刷新 cookie，下次重试使用新 cookie
                    if crate::cf::is_cf_challenge(&text) {
                        warn!(
                            "CF challenge detected for {} (status={}, body={} bytes), refreshing cookie...",
                            url, status, text.len()
                        );
                        let _ = self.cf_manager.refresh(&self.url).await;
                    } else if !status.is_success() {
                        // 非 2xx（通常是 403/503）— 大概率是 CF 拦截或 cookie 失效
                        // 打印前 200 字符便于诊断（可能是 CF 挑战页变体或错误页）
                        let preview: String = text.chars().take(200).collect();
                        warn!(
                            "Request {} returned status={} body={} bytes, refreshing cookie. preview: {:?}",
                            url, status, text.len(), preview
                        );
                        let _ = self.cf_manager.refresh(&self.url).await;
                    } else {
                        // 2xx 但 body 异常（空或无法识别）
                        warn!(
                            "Request {} returned 2xx but body invalid ({} bytes), retrying...",
                            url, text.len()
                        );
                    }
                }
                Err(e) => {
                    warn!("Request {} failed (attempt {}): {}", url, attempt + 1, e);
                    // 网络错误也可能是代理问题或连接被 CF 重置，首次失败刷新 cookie
                    if attempt == 0 {
                        let _ = self.cf_manager.refresh(&self.url).await;
                    }
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
        // 分类可能有多种选择器（不同站点结构略有差异），按顺序尝试
        // 注意：Selector 未实现 Copy，循环里需借用，故收集为 &Selector 数组
        let cat_sel_a = Selector::parse(".info .cat").map_err(|_| anyhow!("html解析失败"))?;
        let cat_sel_b = Selector::parse(".info .category").map_err(|_| anyhow!("html解析失败"))?;
        let cat_sel_c = Selector::parse(".info .tags").map_err(|_| anyhow!("html解析失败"))?;
        let cat_selectors: [&Selector; 3] = [&cat_sel_a, &cat_sel_b, &cat_sel_c];

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

            // 分类：尝试多个候选选择器，避免与 author 重复
            let category = cat_selectors
                .iter()
                .find_map(|sel| {
                    li.select(sel)
                        .next()
                        .and_then(|el| el.text().next())
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                })
                .unwrap_or_default();

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
