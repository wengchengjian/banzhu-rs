use crate::banzhuspider::SpiderConfig;
use crate::cf::CfManager;
use anyhow::{anyhow, Result};
use config::Config;
use futures::stream::{self, StreamExt};
use lazy_static::lazy_static;
use log::{debug, error, info, warn};
use regex::Regex;
use scraper::Html;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::sync::Arc;
use tokio::time::sleep;
use wreq::Client;

mod content;
mod parse;

pub use content::{arr_dup_rem_linked, char_to_unicode, clean_filename, format_novel_content};
pub(crate) use content::split_second;

lazy_static! {
    static ref PAGE_REGEX: Regex =
        Regex::new(r"\(第(\d+?)/(?P<page>\d+?)页\)当前\d+?条/页").unwrap();
    static ref CONTENT_FORMAT_REGEX1: Regex =
        Regex::new(r"&[a-zA-Z0-9]+;|&#[0-9]+;|&apos;|&quot;").unwrap();
    static ref SECTION_DATA_REGEX2: Regex =
        Regex::new(r#"\$\.post\('',\{'j':'1'\},function\(e\)"#).unwrap();
    static ref SECTION_DATA_REGEX3: Regex = Regex::new(r#"var ns='(?P<ns>.+?)'"#).unwrap();
    static ref SECTION_DATA_REGEX4: Regex = Regex::new(
        r#"(?s)var chapter = secret\(\s*["'](?P<cipher>.+?)["'],\s*["'](?P<code>.+?)["'],.+?\);"#
    )
    .unwrap();
    static ref IMG_PANFA_REGEX: Regex = Regex::new(r"/toimg/data/(?P<url>.+?.png)").unwrap();
    static ref FONT_FANPA_REGEX: Regex = Regex::new(r"\\u[a-fA-f0-9]{4}").unwrap();
    static ref URL_REGEX: Regex = Regex::new(r"^https://.+?/(?P<num>\d+)/(?P<id>\d+)/$").unwrap();
    static ref SECTION_NUM_REGEX: Regex = Regex::new(r"【(?P<num>\d+?)】").unwrap();
    static ref SECTION_PAGE_REGEX: Regex =
        Regex::new(r"^(?P<left>.+?)/(?P<right>\d+?)\.html").unwrap();
}

#[derive(Debug)]
pub struct Book {
    pub num: usize,
    pub id: usize,
    pub title: String,
    pub filename: String,
    pub page: u8,
    pub author: String,
    pub category: String,
    pub introduce: String,
    pub likes: u32,
    pub count: u32,
}

impl Display for Book {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "书名：{}\n\n作者: {}\n\n分类: {}\n\n喜欢: {}\n\n字数: {}\n\n简介: {}\n\n",
            self.title, self.author, self.category, self.likes, self.count, self.introduce
        )
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Hash)]
pub struct Chapter {
    pub title: String,
    pub url: String,
    pub sections: Option<Vec<Section>>,
}

#[derive(Debug, Eq, PartialEq, Clone, Hash)]
pub struct Section {
    pub url: String,
    pub content: Option<String>,
}

impl Section {
    pub fn new(url: String) -> Self {
        Self { url, content: None }
    }
}

impl Chapter {
    pub fn new(href: String, title: String) -> Chapter {
        Chapter {
            url: href,
            title,
            sections: None,
        }
    }
}

pub struct BanzhuDownloadTask {
    pub book_id: u32,
    pub root_url: String,
    pub spider_config: Arc<SpiderConfig>,
    pub config: Arc<Config>,
    pub img_fanpa_dict: Arc<HashMap<String, String>>,
    pub font_fanpa_dict: Arc<HashMap<String, String>>,
    /// wreq Client 线程安全，无需 Mutex
    pub client: Client,
    /// CF cookie 生命周期管理器（与 BanzhuSpider 共享）
    pub cf_manager: Arc<CfManager>,
}

impl BanzhuDownloadTask {
    pub fn new(
        root_url: String,
        book_id: u32,
        config: Arc<Config>,
        img_fanpa_dict: Arc<HashMap<String, String>>,
        font_fanpa_dict: Arc<HashMap<String, String>>,
        client: Client,
        spider_config: Arc<SpiderConfig>,
        cf_manager: Arc<CfManager>,
    ) -> Self {
        BanzhuDownloadTask {
            root_url,
            book_id,
            config,
            img_fanpa_dict,
            font_fanpa_dict,
            client,
            spider_config,
            cf_manager,
        }
    }

    /// HTTP GET — wreq Chrome137 TLS 指纹 + CfManager 自动注入 cf_clearance
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
                .ensure(&self.root_url)
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
                        if !text.is_empty() && crate::cf::is_bypassed(&text) {
                            return Ok(text);
                        }
                        if crate::cf::is_cf_challenge(&text) {
                            warn!("CF challenge detected for {}, refreshing cookie...", url);
                            let _ = self.cf_manager.refresh(&self.root_url).await;
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

    /// HTTP POST (用于 type 2 section)，注入 CF cookie + 重试
    async fn post_form(&self, url: &str, form: Vec<(&str, &str)>) -> Result<String> {
        let mut backoff = self.spider_config.retry_delay;

        for attempt in 0..self.spider_config.retry_attempts {
            if attempt > 0 {
                sleep(backoff).await;
                backoff *= 2;
            }

            let (cookie, ua) = self
                .cf_manager
                .ensure(&self.root_url)
                .await
                .map_err(|e| anyhow!("CF bypass failed: {}", e))?;

            let result = self
                .client
                .post(url)
                .header("Cookie", cookie.as_str())
                .header("User-Agent", ua.as_str())
                .form(&form)
                .send()
                .await;

            match result {
                Ok(response) => {
                    if response.status().is_success() {
                        return Ok(response.text().await?);
                    }
                }
                Err(e) => {
                    debug!("POST 重试 {} (attempt {}): {}", url, attempt + 1, e);
                    continue;
                }
            }
        }

        Err(anyhow!("post_form max retries reached for {}", url))
    }

    /// 下载整本书，返回 Book + Chapters 数据
    pub async fn download(&self) -> Result<(Book, Vec<Chapter>)> {
        let url = format!(
            "{}/{}/{}/",
            self.root_url,
            self.book_id / 1000,
            self.book_id
        );
        debug!("crawl book {}: {url}", self.book_id);

        if let Some(captures) = URL_REGEX.captures(&url) {
            if !URL_REGEX.is_match(&url) {
                error!("Invalid URL: {}", &url);
            }
            let book_id: usize = captures["id"].parse()?;
            let book_num: usize = captures["num"].parse()?;

            let text = self.get(&url).await?;
            if !text.is_empty() {
                let html = Html::parse_document(&text);
                let mut book = self.get_info(book_id, &html).await?;
                book.id = book_id;
                book.num = book_num;
                let chapters = self.get_chapters_content(&book).await?;
                return Ok((book, chapters));
            }
        }

        Err(anyhow!("下载失败: book_id={}", self.book_id))
    }

    /// 增量下载：只下载新增的章节页
    pub async fn download_incremental(&self, existing_chapter_count: usize) -> Result<(Book, Vec<Chapter>)> {
        let url = format!(
            "{}/{}/{}/",
            self.root_url,
            self.book_id / 1000,
            self.book_id
        );
        debug!("incremental crawl book {}: {url}", self.book_id);

        if let Some(captures) = URL_REGEX.captures(&url) {
            let book_id: usize = captures["id"].parse()?;
            let book_num: usize = captures["num"].parse()?;

            let text = self.get(&url).await?;
            if !text.is_empty() {
                let html = Html::parse_document(&text);
                let mut book = self.get_info(book_id, &html).await?;
                book.id = book_id;
                book.num = book_num;

                let _start_page = if book.page > 0 {
                    ((existing_chapter_count / 10) as u8).min(book.page.saturating_sub(1)) + 1
                } else {
                    1
                };

                let chapters = self.get_chapters_content_from_page(&book, _start_page).await?;
                return Ok((book, chapters));
            }
        }

        Err(anyhow!("增量下载失败: book_id={}", self.book_id))
    }

    pub async fn get_chapters_content(&self, book: &Book) -> Result<Vec<Chapter>> {
        self.get_chapters_content_from_page(book, 1).await
    }

    pub async fn get_chapters_content_from_page(&self, book: &Book, start_page: u8) -> Result<Vec<Chapter>> {
        let mut page_urls = vec![];
        let root_url = self.config
            .get_string("root_url")
            .map_err(|_| anyhow!("spider.toml 中未配置 root_url"))?;
        for page in start_page..book.page + 1 {
            let page_url = format!(
                "{}/{}/{}_{}/",
                root_url,
                book.num,
                book.id,
                page
            );
            page_urls.push(page_url);
        }
        let mut chapters = self.get_chapters_url(page_urls).await?;

        if chapters.len() == 0 {
            return Err(anyhow!("未发现chapter"));
        }
        self.get_sections_url(&mut chapters).await?;

        self.get_sections_data(&mut chapters).await?;

        Ok(chapters)
    }

    pub async fn get_sections_data(&self, chapters: &mut Vec<Chapter>) -> Result<()> {
        debug!("正在获取Section Data...");
        let concurrency = 8;

        let mut all_sections = Vec::new();
        for chapter in chapters.iter_mut() {
            if let Some(sections) = &mut chapter.sections {
                for section in sections.iter_mut() {
                    all_sections.push((section, chapter.title.clone()));
                }
            }
        }

        let results = stream::iter(all_sections)
            .map(|(section, chapter_title)| {
                let section_url = section.url.clone();
                async move {
                    let result = self.process_section(&section_url).await;
                    (section, result, chapter_title)
                }
            })
            .buffer_unordered(concurrency)
            .collect::<Vec<_>>()
            .await;

        for (section, result, chapter_title) in results {
            match result {
                Ok(content) => {
                    if !content.is_empty() {
                        section.content = Some(format!("\t{}", content.trim()));
                    }
                }
                Err(e) => {
                    return Err(anyhow!(
                        "Failed to process section in chapter {}: {}",
                        chapter_title,
                        e
                    ));
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    #[test]
    fn test_format_novel_content() {
        let content = "段落1\n\n\n\n段落2\n\n\n\n段落3\n\n\n段落4\n段落5".to_string();
        let formatted = super::format_novel_content(&content);
        assert_eq!(formatted, "段落1\n\n段落2\n\n段落3\n\n段落4\n\n段落5");
    }

    #[test]
    #[ignore] // 依赖外部书籍文件，CI 环境不可用
    fn test_format_file_content() {
        let content = fs::read_to_string("book/其他类别/[同人]俗人回档h.txt").unwrap();
        let formatted = super::format_novel_content(&content);
        fs::write("test.txt", formatted).unwrap();
    }
}
