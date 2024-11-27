use crate::bypass::{is_bypassed, CloudflareBypass};
use crate::error::SpiderError;
use crate::error::SpiderError::{DecodingError, HtmlParseError, NotFoundChapters, OtherError, RequestError, UnknownError};
use crate::{decrpyt_aes_128_cbc, get_section_data_by_py, POST_TEXT};
use config::Config;
use encoding::all::GBK;
use encoding::{DecoderTrap, Encoding};
use lazy_static::lazy_static;
use pbr::ProgressBar;
use regex::Regex;
use reqwest::{Client, Response};
use scraper::selectable::Selectable;
use scraper::{Html, Selector};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::fmt::{Display, Formatter};
use std::hash::Hash;
use std::io::Stdout;
use std::ops::Deref;
use std::string::FromUtf8Error;
use std::sync::Arc;
use std::time::Duration;
use futures::future::join_all;
use futures::{FutureExt, TryStreamExt};
use tokio::fs::{create_dir_all, OpenOptions};
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;
use tokio::time::sleep;

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
    pub book_id: i64,
    pub root_url: String,
    pub config: Arc<Config>,
    pub img_fanpa_dict: Arc<HashMap<String, String>>,
    pub font_fanpa_dict: Arc<HashMap<String, String>>,
    pub client: Arc<Client>,
    pub cf: Arc<Mutex<CloudflareBypass>>
}


impl BanzhuDownloadTask {
    pub fn new(root_url: String, book_id: i64, config: Arc<Config>,
               img_fanpa_dict: Arc<HashMap<String, String>>,
               font_fanpa_dict: Arc<HashMap<String, String>>,
               client: Arc<Client>, cf: Arc<Mutex<CloudflareBypass>>) -> Self {
        BanzhuDownloadTask {root_url, book_id, config, img_fanpa_dict, font_fanpa_dict, client, cf}
    }



    pub async fn post_client_request(&self, url: &str, data: &Value) -> Result<Response, SpiderError> {
        let headers = self.cf.lock().await.get_headers().await;

        Ok(self.client.post(url).json(data).headers(headers).send().await?)
    }

    pub async fn get_client_request(&self, url: &str) -> Result<Response, reqwest::Error> {
        let headers = self.cf.lock().await.get_headers().await;
        Ok(self.client.get(url).headers(headers).send().await?)
    }

    async fn post_text(&self, url: &str, text: &Value) -> Result<String, SpiderError> {
        let retry = 5;
        for i in 0..retry {
            if i != 0 {
                sleep(Duration::from_millis(100)).await;
                println!("第{}次重连: {}", i, url);
            }
            let response = self.post_client_request(url, text).await?;
            if response.status().is_success() {
                let text = response.text().await?;
                return Ok(text);
            }
        }
        Err(UnknownError)
    }

    async fn bypass_cloudflare(&self) -> Result<(), SpiderError>{
        Ok(self.cf.lock().await.bypass_cloudflare().await?)
    }

    pub async fn get(&self, url: &str) -> Result<String, SpiderError> {
        let retry = 5;
        for i in 0..retry {
            if i != 0 {
                sleep(Duration::from_millis(100)).await;
                println!("第{}次重连: {}", i, url);
            }
            // 模拟随机请求行为
            // time::sleep(Duration::from_millis(rng.gen_range(89..1128))).await;
            match self.get_client_request(url).await {
                Ok(response) => {
                    if response.status().is_success() {
                        let text = response.text().await.unwrap();
                        if text.len() > 0 {
                            if !is_bypassed(&text) {
                                self.bypass_cloudflare().await?;
                            } else if text.contains(POST_TEXT) {
                                let data = json!({
                                    "action": "1",
                                    "v": "1234"
                                });
                                let resp = self.post_text(url, &data).await?;
                                if resp == "success" {
                                    println!("post 1234 success")
                                }
                            } else {
                                return Ok(text);
                            }
                        }
                    } else {
                        if response.status().as_u16() == 403 {
                            println!("url: {}, status: 403", url);
                        }
                    }
                }
                Err(e) => {
                    if e.is_timeout() || e.is_connect() {
                        self.bypass_cloudflare().await?;
                    } else if e.is_status() {
                        println!("status: {:?}", e.status().unwrap());
                    }
                }
            };
        }

        Err(UnknownError)
    }

    pub async fn download(&self) -> Result<(), SpiderError> {
        let url = format!("{}/{}/{}/", self.root_url, self.book_id % 1000, self.book_id);
        println!("crawl book {}: {url}",self.book_id);

        if let Some(captures) = URL_REGEX.captures(&url) {
            if !URL_REGEX.is_match(&url) {
                println!("Invalid URL: {}", &url);
            }
            let book_id: usize = captures["id"].parse().unwrap();
            let book_num: usize = captures["num"].parse().unwrap();

            match self.get(&url).await {
                Ok(text) => {
                    let text = text.as_str();
                    if text.len() > 0 {
                        let html = Html::parse_document(text);

                        let mut book = self.get_info(&html).await?;
                        book.id = book_id;
                        book.num = book_num;
                        let chapters = self.get_chapters_content(&book).await?;
                        save_book_local_txt(
                            &book,
                            &chapters,
                            self.config
                                .get_string("save_path")
                                .unwrap_or("book".to_string())
                                .as_str(),
                        )
                        .await?;
                    }
                }
                Err(_) => {
                    println!("{url} 请求失败");
                    // 尝试重新获取cookie
                    self.bypass_cloudflare().await?;
                }
            };
        }

        Ok(())
    }

    pub async fn get_info(&self, html: &Html) -> Result<Book, SpiderError> {
        let page_sec = Selector::parse(".pagelistbox .page").unwrap();
        let page = html.select(&page_sec).next().ok_or(HtmlParseError)?;
        let page_text = page.inner_html();
        let page: u8 = PAGE_REGEX.captures(&page_text).unwrap()["page"]
            .to_string()
            .parse()
            .unwrap();
        
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
    ) -> Result<Vec<Chapter>, SpiderError> {
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
            return Err(NotFoundChapters);
        }
        self.get_sections_url(&mut chapters).await?;

        self.get_sections_data(&mut chapters).await?;

        Ok(chapters)
    }

    pub async fn get_sections_data(
        &self,
        chapters: &mut Vec<Chapter>,
    ) -> Result<(), SpiderError> {
        println!("正在获取Section Data...");
        let mut pbr = Arc::new(Mutex::new(create_pbr(get_chapter_section_num(chapters))));
        
        let mut futures = vec![];
        // 异步获取html
        for chapter in &mut *chapters {
            if let Some(sections) = &mut chapter.sections {
                for section in sections {
                    let section_url = section.url.clone();
                    let pbr = pbr.clone();
                    let future = async move {
                        let html_str = self.get(&section_url).await.unwrap();
                        let content = self.get_section_data1(&html_str).unwrap();

                        let html = Html::parse_document(&html_str);

                        let content = content + self.get_section_data2(&section_url, &html).await.unwrap_or(String::new()).as_str();

                        let content = content + self.get_section_data3(&html).unwrap_or(String::new()).as_str();

                        let content = content + self.get_section_data4(&html).unwrap_or(String::new()).as_str();

                        if content.len() > 0 {
                            // 去除特殊字符
                            let content = content.replace(' ', " ");
                            // let content = content.replace("\n", "\n\n");
                            // 添加首行缩进
                            let content = "\t".to_owned() + content.as_str();
                            section.content = Some(content);
                        }
                        // 进度 加 1
                        pbr.lock().await.inc();
                    };
                    futures.push(future);
                }
            }
        }
        join_all(futures).await;
        
        Ok(())
    }
    /// 接口返回的整个html
    async fn get_section_data2(
        &self,
        url: &str,
        html: &Html,
    ) -> Result<String, SpiderError> {
        let html_str = html.html();
        let mut content = String::new();
        if SECTION_DATA_REGEX2.is_match(&html_str) {
            let data = json!({
                "j":"1"
            });
            let res = self.post_client_request(url, &data).await?;

            content = res.text().await?;
            content = self.format_content(content)?;
        }
        Ok(content)
    }

    /// 返回的内容
    fn get_section_data3(&self, html: &Html) -> Result<String, SpiderError> {
        let mut content = String::new();
        let html_str = html.html();
        if let Some(cap) = SECTION_DATA_REGEX3.captures(&html_str) {
            let ns = &cap["ns"];
            content = get_section_data_by_py(&html_str, ns)?;
            if content.len() > 0 {
                content = content.replace(r"<div.+?>", "");
                content = content.replace(r"</div>", "");
                content = self.format_content(format!(
                    "<div class=\"neirong\"><div>{}</div></div>",
                    content
                ))?;
            }
        }
        Ok(content)
    }

    /// 返回内容
    fn get_section_data4(&self, html: &Html) -> Result<String, SpiderError> {
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
            let content = self.format_content(format!(
                "<div class=\"neirong\"><div>{}</div></div>",
                content
            ))?;
            return Ok(content);
        }
        Ok("".to_string())
    }
    fn get_section_data1(&self, html: &str) -> Result<String, SpiderError> {
        Ok(self.format_content(html.to_string())?)
    }

    fn format_content(&self, html_str: String) -> Result<String, SpiderError> {
        let html = Html::parse_document(&html_str);
        let nodes = html
            .select(&Selector::parse(".neirong div").unwrap())
            .next()
            .unwrap()
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
        Ok(content)
    }

    pub async fn get_sections_url(
        &self,
        chapters: &mut Vec<Chapter>,
    ) -> Result<(), SpiderError> {
        println!("正在获取Section URL...");

        let mut pbr = Arc::new(Mutex::new(create_pbr(chapters.len() as u64)));
        let mut futures = vec![];
        for chapter in chapters {
            let mut sections = vec![];
            let pbr = pbr.clone();
            
            let future = async move {
                let html_str = self.get(&chapter.url).await.unwrap();
                let html = Html::parse_document(&html_str);
                let selector = Selector::parse(".chapterPages a").unwrap();
                let section_list = html.select(&selector);

                let mut section_num = 1;
                let mut sec_num_list = vec![];
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
                pbr.lock().await.inc();
            };
            futures.push(future);
        }
        
        join_all(futures).await;
        
        Ok(())
    }

    pub async fn get_chapters_url(
        &self,
        page_urls: Vec<String>,
    ) -> Result<Vec<Chapter>, SpiderError> {
        println!("正在获取Chapter URL...");

        let mut chapters = vec![];
        let mut pbr = create_pbr(page_urls.len() as u64);
        for page_url in page_urls {
            let html = Html::parse_document(&self.get(&page_url).await?);
            let selector = Selector::parse(".chapter-list .bd ul li a").unwrap();
            let chapter_list = html.select(&selector);
                
            for chapter in chapter_list {
                if let Some(href) = chapter.attr("href") {
                    if let Some(title) = chapter.text().next() {
                        let url = format!("{}{}", self.root_url, href);
                        chapters.push(Chapter::new(url, title.to_string()))
                    }
                }
            }
            pbr.inc();
        }
        // 去重
        chapters = arr_dup_rem_linked(chapters);
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

async fn save_book_local_txt(
    book: &Book,
    chapters: &Vec<Chapter>,
    dir: &str,
) -> Result<(), SpiderError> {
    let dir = format!("{}/{}", dir, book.category);
    // 先创建目录
    create_dir_all(&dir).await?;
    let filename = format!("{}/{}.txt", dir, book.title);
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(filename)
        .await?;
    // 写入小说头
    file.write(book.to_string().as_bytes()).await?;

    for chapter in chapters {
        let chapter_title = format!("\n\n{}\n\n", &chapter.title);
        // 写入章节标题
        file.write(chapter_title.as_bytes()).await?;
        if let Some(sections) = &chapter.sections {
            for section in sections {
                if let Some(content) = &section.content {
                    // 写入内容
                    file.write_all(content.as_bytes()).await?;
                }
            }
        }
    }
    file.flush().await?;
    Ok(())
}
fn split_second(s: &str, pattern: &str) -> String {
    s.split(pattern).collect::<Vec<&str>>()[1]
        .trim()
        .to_string()
}

pub fn create_pbr(count: u64) -> ProgressBar<Stdout> {
    let mut pb = ProgressBar::new(count);
    pb.format("╢▌▌░╟");
    return pb;
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
