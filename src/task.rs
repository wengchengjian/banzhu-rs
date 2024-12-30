use crate::bypass::{is_bypassed, CloudflareBypass};
use crate::error::SpiderError;
use crate::error::SpiderError::{DecodingError, HtmlParseError, NotFoundChapters,RequestError};
use crate::{create_pbr, decrpyt_aes_128_cbc, get_section_data_by_py, POST_TEXT};
use config::Config;
use encoding::all::GBK;
use encoding::{DecoderTrap, Encoding};
use lazy_static::lazy_static;
use regex::Regex;
use reqwest::{Client, Response};
use scraper::selectable::Selectable;
use scraper::{Html, Selector};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::fmt::{Display, Formatter};
use std::hash::Hash;
use std::io::{BufWriter, Stdout};
use std::ops::Deref;
use std::path::Path;
use std::string::FromUtf8Error;
use std::sync::Arc;
use std::time::Duration;
use tokio::fs::{create_dir_all, OpenOptions};
use tokio::io::{AsyncWriteExt, BufWriter as AsyncBufWriter};
use tokio::sync::Mutex;
use tokio::time::sleep;
use anyhow::{anyhow, Result};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use log::{debug, error, info, warn};
use crate::banzhuspider::{add_download_book_id, SpiderConfig};
use tokio::sync::RwLock;
use futures::stream::{self, StreamExt};

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
    fn new(url: String) -> Self {
        Self { url, content: None }
    }
}

impl Chapter {
    fn new(href: String, title: String) -> Chapter {
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
    pub client: Arc<Client>,
    pub cf: Arc<Mutex<CloudflareBypass>>,
    pub multi_pbr: MultiProgress,
}

struct ProgressGuard<'a> {
    multi_pbr: &'a MultiProgress,
    pbr: ProgressBar,
}

impl<'a> Drop for ProgressGuard<'a> {
    fn drop(&mut self) {
        self.pbr.finish_and_clear();
        self.multi_pbr.remove(&self.pbr);
    }
}

impl BanzhuDownloadTask {
    pub fn new(root_url: String, book_id: u32, config: Arc<Config>,
               img_fanpa_dict: Arc<HashMap<String, String>>,
               font_fanpa_dict: Arc<HashMap<String, String>>,
               client: Arc<Client>, cf: Arc<Mutex<CloudflareBypass>>, multi_pbr: MultiProgress, spider_config: Arc<SpiderConfig>) -> Self {
        BanzhuDownloadTask {root_url, book_id, config, img_fanpa_dict, font_fanpa_dict, client, cf, multi_pbr, spider_config }
    }

    pub async fn post_client_request(&self, url: &str, data: &Value) -> Result<Response, SpiderError> {
        let headers = self.cf.lock().await.get_headers().await;

        Ok(self.client.post(url).json(data).headers(headers).send().await?)
    }

    pub async fn get_client_request(&self, url: &str) -> Result<Response, reqwest::Error> {
        let headers = self.cf.lock().await.get_headers().await;
        Ok(self.client.get(url).timeout(self.spider_config.request_timeout).headers(headers).send().await?)
    }

    async fn post_text(&self, url: &str, text: &Value) -> Result<String> {
        let retry = 5;
        for i in 0..retry {
            if i != 0 {
                sleep(Duration::from_millis(100)).await;
                debug!("第{}次重连: {}", i, url);
            }
            let response = self.post_client_request(url, text).await?;
            if response.status().is_success() {
                let text = response.text().await?;
                return Ok(text);
            }
        }
        Err(anyhow!("未知异常"))
    }

    async fn bypass_cloudflare(&self) -> Result<()>{
        Ok(self.cf.lock().await.bypass_cloudflare().await?)
    }

    pub async fn get(&self, url: &str) -> Result<String> {
        let mut last_error = None;
        let mut backoff = self.spider_config.retry_delay;

        for attempt in 0..self.spider_config.retry_attempts {
            if attempt > 0 {
                debug!("Retry attempt {} for {}", attempt, url);
                sleep(backoff).await;
                // 指数退避策略
                backoff *= 2;
            }

            match self.get_client_request(url).await {
                Ok(response) => {
                    if response.status().is_success() {
                        let text = response.text().await?;
                        if !text.is_empty() {
                            if !is_bypassed(&text) {
                                self.bypass_cloudflare().await?;
                                continue;
                            }
                            return Ok(text);
                        }
                    } else if response.status().as_u16() == 429 {
                        // 如果遇到限流，使用更长的退避时间
                        backoff *= 2;
                        continue;
                    }
                }
                Err(e) => {
                    last_error = Some(anyhow!("request error: {}", e));
                    continue;
                }
            }
        }

        Err(last_error.unwrap_or(anyhow!("Max retry attempts reached")))
    }

    pub async fn download(&self) -> Result<()> {
        let url = format!("{}/{}/{}/", self.root_url, self.book_id % 1000, self.book_id);
        debug!("crawl book {}: {url}",self.book_id);

        if let Some(captures) = URL_REGEX.captures(&url) {
            if !URL_REGEX.is_match(&url) {
                warn!("Invalid URL: {}", &url);
            }
            let book_id: usize = captures["id"].parse()?;
            let book_num: usize = captures["num"].parse()?;

            match self.get(&url).await {
                Ok(text) => {
                    let text = text.as_str();
                    if text.len() > 0 {
                        let dir = self.config
                        .get_string("save_path")
                        .unwrap_or("book".to_string());

                        let html = Html::parse_document(text);

                        let mut book = self.get_info(&html).await?;
                        book.id = book_id;
                        book.num = book_num;

                        if check_local_book_exist(&book, &dir)? {
                            debug!("{}-{}已存在", book.category, book.title);
                            add_download_book_id(book.id as u32).await;
                            return Ok(());
                        }



                        let chapters = self.get_chapters_content(&book).await?;
                        save_book_local_txt(
                            &book,
                            &chapters,
                            &dir,
                        )
                        .await?;
                        add_download_book_id(book.id as u32).await;
                    }
                }
                Err(_) => {
                    debug!("{url} 请求失败");
                    // 尝试重新获取cookie
                    self.bypass_cloudflare().await?;
                }
            };
        }

        Ok(())
    }

    pub async fn get_info(&self, html: &Html) -> Result<Book> {
        let page_sec = Selector::parse(".pagelistbox .page").unwrap();
        let page = html.select(&page_sec).next().ok_or(anyhow!("html解析异常"))?;
        let page_text = page.inner_html();
        let page: u8 = PAGE_REGEX.captures(&page_text).unwrap()["page"]
            .to_string()
            .parse()?;
        
        let book_sec = Selector::parse("h1").unwrap();
        let book_name = html.select(&book_sec)
            .next()
            .unwrap()
            .text()
            .next()
            .unwrap()
            .to_string();
        
        let mut introduce = String::new();
        
        let bd_sec = Selector::parse(".bd").unwrap();
        
        let bd = html.select(&bd_sec).next();
        if let Some(bd) = bd {
            if let Some(text) = bd.text().next() {
                if text.len() != 0 {
                    introduce.push_str(text);
                }
            }
        }
        let info_sec = Selector::parse(".info").unwrap();
        let mut info = html
            .select(&info_sec)
            .next().unwrap()
            .text();
        let author = split_second(info.next().unwrap(), "：");
        let book_category = split_second(info.next().unwrap(), "：");
        let book_count: u32 = split_second(info.next().unwrap(), "：").parse().unwrap();
        let book_like: u32 = split_second(info.next().unwrap(), "：").parse().unwrap();

        let book = Book {
            num: 0,
            id: 0,
            title: book_name,
            page,
            author,
            category: book_category,
            introduce,
            likes: book_like,
            count: book_count,
        };

        return Ok(book);
    }

    pub async fn get_chapters_content(
        &self,
        book: &Book,
    ) -> Result<Vec<Chapter>> {
        let mut page_urls = vec![];
        for page in 1..book.page + 1 {
            let page_url = format!(
                "{}/{}/{}_{}/",
                self.config.get_string("root_url").expect("not found root_url"),
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

        self.get_sections_data(&mut chapters, book).await?;

        Ok(chapters)
    }

    pub async fn get_sections_data(
        &self,
        chapters: &mut Vec<Chapter>,
        book: &Book,
    ) -> Result<()> {
        debug!("正在获取Section Data...");
        let pbr = self.multi_pbr.add(create_pbr(get_chapter_section_num(chapters)));
        pbr.set_message(format!("{}-{}", book.title, book.id));
        
        // 使用 ProgressGuard 来确保进度条被清理
        let guard = ProgressGuard {
            multi_pbr: &self.multi_pbr,
            pbr: pbr.clone(),
        };
        
        let ret = self.get_sections_data_pbr(chapters, &guard.pbr).await;
        
        // guard 会在函数结束时自动调用 drop，确保进度条被移除
        ret
    }

    /// 接口返回的整个html
    async fn get_section_data2(
        &self,
        url: &str,
        html: &Html,
    ) -> Result<String> {
        let html_str = html.html();
        let mut content = String::new();
        if SECTION_DATA_REGEX2.is_match(&html_str) {
            let data = json!({
                "j":"1"
            });
            let res = self.post_client_request(url, &data).await?;

            content = res.text().await?;
        }
        Ok(content)
    }

    /// 返回的内容
    async fn get_section_data3(&self, html: &Html) -> Result<String> {
        let mut content = String::new();
        let html_str = html.html();
        if let Some(cap) = SECTION_DATA_REGEX3.captures(&html_str) {
            let ns = &cap["ns"];
            content = get_section_data_by_py(&html_str, ns)?;
            if content.len() > 0 {
                content = content.replace(r"<div.+?>", "");
                content = content.replace(r"</div>", "");
            }
        }
        Ok(content)
    }

    /// 返回内容
    async fn get_section_data4(&self, html: &Html) -> Result<String> {
        let html_str = html.html();
        if let Some(cap) = SECTION_DATA_REGEX4.captures(&html_str) {
            let cipher_text = &cap["cipher"];
            let code = &cap["code"];
            let content = decrpyt_aes_128_cbc(cipher_text.as_bytes(), code.as_bytes())?;
            // let length = content.len();
            // let i = length - content[length - 1] as usize;
            // let content = content[..i + 1].to_vec();
            let content = String::from_utf8(content).unwrap_or_else(|e| {
                let arr = e.into_bytes();
                // utf8失败了就用gbk试试
                GBK.decode(&arr, DecoderTrap::Strict).expect(format!("编码错误:{html_str}").as_str())
            });
            return Ok(content);
        }
        Ok("".to_string())
    }
    async fn get_section_data1(&self, html: &Html) -> Result<String> {
        Ok(self.format_content(None, Some(html))?)
    }

    fn format_content(&self, html_str: Option<&str>, html: Option<&Html>) -> Result<String> {
        let mut html2 = None;
        if let Some(html_str) = html_str{
            html2 = Some(Html::parse_document(html_str))
        }

        let html = {
            if let Some(html) = html {
                Some(html)
            } else if let Some(html_str) = html_str{
                Some(html2.as_ref().unwrap())
            } else {
                None
            }
        };

        if let Some(html) = html {
            let nodes = html
                .select(&Selector::parse(".neirong div").map_err(|e| anyhow!("html解析失败"))?)
                .next()
                .ok_or(anyhow!("没有neirong节点"))?
                .descendants();


            let mut content = String::new();
            for node in nodes {
                if node.value().is_text() {
                    // 如果是文本节点
                    if let Some(text) = node.value().as_text() {
                        let word = text.deref();
                        if word.len() == 3 {
                            let uni_word = char_to_unicode(word.chars().next().unwrap());
                            if let Some(word) = self.font_fanpa_dict.get(&uni_word) {
                                content.push_str(word);
                            } else {
                                content.push_str(word);
                            }
                        } else {
                            content.push_str(word);
                        }
                    }
                } else if node.value().is_element() {
                    if let Some(element) = node.value().as_element() {
                        // 如果是元素节点
                        match element.name() {
                            "br" => {
                                content.push('\n');
                            }
                            "img" => {
                                if let Some(src) = element.attr("src") {
                                    // 转换图片字体
                                    if let Some(cap) = IMG_PANFA_REGEX.captures(src) {
                                        let url = &cap["url"];
                                        if let Some(word) = self.img_fanpa_dict.get(url) {
                                            content.push_str(word);
                                        }
                                    }
                                }
                            }
                            "i" => {}
                            _ => {}
                        }
                    }
                }
            }
            return Ok(content);
        };

        Err(anyhow!("参数错误"))

    }

    pub async fn get_sections_data_pbr(&self,
                                       chapters: &mut Vec<Chapter>,
                                       pbr: &ProgressBar) -> Result<()> {
        // 使用固定大小的缓冲区来控制并发
        let concurrency = 8;
        
        // 收集所有需要处理的章节
        let mut all_sections = Vec::new();
        for chapter in chapters.iter_mut() {
            if let Some(sections) = &mut chapter.sections {
                for section in sections.iter_mut() {
                    all_sections.push((section, chapter.title.clone()));
                }
            }
        }

        // 使用stream进行并发处理
        let results = stream::iter(all_sections)
            .map(|(section, chapter_title)| {
                let section_url = section.url.clone();
                async move {
                    let result = self.process_section(&section_url).await;
                    pbr.inc(1);
                    (section, result, chapter_title)
                }
            })
            .buffer_unordered(concurrency)
            .collect::<Vec<_>>()
            .await;

        // 处理结果
        for (section, result, chapter_title) in results {
            match result {
                Ok(content) => {
                    if !content.is_empty() {
                        section.content = Some(format!("\t{}", content.trim()));
                    }
                }
                Err(e) => {
                    return Err(anyhow!("Failed to process section in chapter {}: {}", chapter_title, e));
                }
            }
        }
        
        Ok(())
    }

    async fn process_section(&self, section_url: &str) -> Result<String> {
        let html_str = self.get(section_url).await?;
        
        // 预分配一个合理的容量来存储内容
        let mut content = String::new();
        
        // 只解析一次HTML
        let html = Html::parse_document(&html_str);
        
        let mut is_content_1 = false;
        if let Ok(initial_content) = self.get_section_data1(&html).await {
            content = initial_content;
            is_content_1 = true;
        }

        // 处理其他内容获取方法
        if let Ok(content2) = self.get_section_data2(section_url, &html).await {
            content = content2;
        }
        if let Ok(content3) = self.get_section_data3(&html).await {
            content = content3;
        }
        if let Ok(content4) = self.get_section_data4(&html).await {
            content = content4;
        }
        let format_str = format!(
            "<div class=\"neirong\"><div>{}</div></div>",
            content
        );
        if is_content_1 {
            return Ok(content);
        } else {
            Ok(self.format_content(Some(&format_str), None)?)
        }

    }

    pub async fn get_sections_url(
        &self,
        chapters: &mut Vec<Chapter>,
    ) -> Result<()> {
        debug!("正在获取Section URL...");
        let concurrency = 4;

        stream::iter(chapters)
            .map(|chapter| {
                let mut sections = vec![];
                async move {
                    let html_str = self.get(&chapter.url).await.unwrap();
                    let html = Html::parse_document(&html_str);
                    let selector = Selector::parse(".chapterPages a").unwrap();
                    let section_list = html.select(&selector);

                    let mut section_num = 1;
                    let mut sec_num_list = vec! [];
                    for section_l in section_list {
                        section_num += 1;
                        let num: u8 = SECTION_NUM_REGEX
                            .captures(section_l.text().next().unwrap())
                            .unwrap()["num"]
                            .to_string()
                            .parse()
                            .unwrap();
                        sec_num_list.push(num);
                    }
                    let mut max_sec_num = section_num;
                    if sec_num_list.len() != 0 {
                        max_sec_num = *sec_num_list.iter().max().unwrap();
                    }

                    let group = SECTION_PAGE_REGEX.captures(chapter.url.as_str()).unwrap();

                    let left = group["left"].to_string();
                    let right = group["right"].to_string();

                    for i in 1..max_sec_num + 1 {
                        sections.push(Section::new(format!("{}/{}_{}.html", left, right, i)));
                    }
                    // 去重
                    sections = arr_dup_rem_linked(sections);

                    chapter.sections = Some(sections);
                }
            })
            .buffer_unordered(concurrency)
            .collect::<Vec<_>>().await;
        Ok(())
    }

    pub async fn get_chapters_url(
        &self,
        page_urls: Vec<String>,
    ) -> Result<Vec<Chapter>> {
        debug!("正在获取Chapter URL...");
        let mut chapters = vec![];
        for url in page_urls {
            let content = self.get(&url).await?;

            let html = Html::parse_document(&content);
            let selector = Selector::parse(".chapter-list").unwrap();
            let a_selector = Selector::parse(".bd .list li a").unwrap();
            let chapter_list = html.select(&selector).nth(1).unwrap().select(&a_selector);

            for chapter in chapter_list {
                if let Some(href) = chapter.attr("href") {
                    if let Some(title) = chapter.text().next() {
                        let url = format!("{}{}", self.root_url, href);
                        chapters.push(Chapter::new(url, title.to_string()))
                    }
                }
            }
        }
        // 去重
        let chapters = arr_dup_rem_linked(chapters);
        Ok(chapters)
    }
}
pub fn char_to_unicode(c: char) -> String {
    let unicode_value: u32 = c as u32;
    format!(r"\u{:x}", unicode_value)
}


fn get_chapter_section_num(chapters: &Vec<Chapter>) -> u64 {
    let mut num = 0;
    for chapter in chapters {
        if let Some(sections) = &chapter.sections {
            num += sections.len();
        }
    }
    return num as u64;
}

fn check_local_book_exist(book: &Book, dir: &str) -> Result<bool> {
    let dir = format!("{}/{}", dir, book.category);

    let filename = format!("{}/{}.txt", dir, book.title);
    Ok(Path::new(&filename).exists())
}

async fn save_book_local_txt(
    book: &Book,
    chapters: &Vec<Chapter>,
    dir: &str
) -> Result<()> {
    // 创建目录
    let mut category = book.category.clone();
    if category.is_empty() {
        category = "其他分类".to_string();
    }
    let dir = format!("{}/{}", dir, category);
    create_dir_all(&dir).await?;

    // 预分配缓冲区
    let estimated_size = chapters.iter()
        .filter_map(|c| c.sections.as_ref())
        .flat_map(|s| s.iter())
        .filter_map(|s| s.content.as_ref())
        .map(|c| c.len())
        .sum::<usize>();

    let mut content = String::with_capacity(estimated_size + 1024 * 1024);

    // 写入书籍信息
    content.push_str(&format!("书名：{}\n", book.title));
    content.push_str(&format!("作者：{}\n", book.author));
    content.push_str(&format!("简介：{}\n\n", book.introduce));

    // 批量写入章节内容
    for chapter in chapters {
        content.push_str(&format!("\n{}\n\n", chapter.title));
        if let Some(sections) = &chapter.sections {
            for section in sections {
                if let Some(section_content) = &section.content {
                    content.push_str(section_content);
                    content.push('\n');
                }
            }
        }
    }

    // 使用缓冲写入
    let filename = format!("{}/{}.txt", dir, book.title);
    let file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&filename)
        .await?;

    // 清除多余空格
    let _ = content.trim();
    let mut writer = AsyncBufWriter::with_capacity(128 * 1024, file);
    writer.write_all(content.as_bytes()).await?;
    writer.flush().await?;

    Ok(())
}
fn split_second(s: &str, pattern: &str) -> String {
    s.split(pattern).collect::<Vec<&str>>()[1]
        .trim()
        .to_string()
}

pub fn arr_dup_rem_linked<T: Eq + Clone + Hash>(arr: Vec<T>) -> Vec<T> {
    let mut set = HashSet::new();
    let mut uniq_arr = Vec::new();
    for ele in arr {
        let elec = ele.clone();
        if set.insert(elec) {
            uniq_arr.push(ele);
        }
    }
    return uniq_arr;
}
