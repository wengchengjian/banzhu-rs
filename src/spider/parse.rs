//! banzhu 站点解析自由函数（从 task/parse.rs + content.rs 迁移，去 &self 依赖）。
//! 所有公开函数只接受 &str，内部用 scraper::Html 解析。

use std::collections::{HashMap, HashSet};
use std::fmt::{Display, Formatter};
use std::hash::Hash;
use std::ops::Deref;

use anyhow::Result;
use encoding::all::GBK;
use encoding::{DecoderTrap, Encoding};
use lazy_static::lazy_static;
use regex::Regex;
use scraper::{Html, Selector};

#[allow(unused_imports)]
use log::{debug, warn};

lazy_static! {
    /// 书籍详情页分页正则（迁移自 task/mod.rs::PAGE_REGEX）
    pub(crate) static ref PAGE_REGEX: Regex =
        Regex::new(r"\(第(\d+?)/(?P<page>\d+?)页\)当前\d+?条/页").unwrap();
    /// 章节分页文本中的页码提取正则（迁移自 task/mod.rs::SECTION_NUM_REGEX）
    pub(crate) static ref SECTION_NUM_REGEX: Regex = Regex::new(r"【(?P<num>\d+?)】").unwrap();
    /// 章节URL拆分正则（迁移自 task/mod.rs::SECTION_PAGE_REGEX）
    pub(crate) static ref SECTION_PAGE_REGEX: Regex =
        Regex::new(r"^(?P<left>.+?)/(?P<right>\d+?)\.html").unwrap();
    /// 策略2：检测 $.post form 拉取正文模式（迁移自 task/content.rs）
    pub(crate) static ref SECTION_DATA_REGEX2: Regex =
        Regex::new(r#"\$\.post\('',\{'j':'1'\},function\(e\)"#).unwrap();
    /// 策略3：var ns='...' base64 索引数组（迁移自 task/content.rs）
    pub(crate) static ref SECTION_DATA_REGEX3: Regex = Regex::new(r#"var ns='(?P<ns>.+?)'"#).unwrap();
    /// 策略4：var chapter = secret(cipher, code, ...) AES 密文（迁移自 task/content.rs）
    pub(crate) static ref SECTION_DATA_REGEX4: Regex = Regex::new(
        r#"(?s)var chapter = secret\(\s*["'](?P<cipher>.+?)["'],\s*["'](?P<code>.+?)["'],.+?\);"#,
    )
    .unwrap();
    /// 图片反爬 URL 提取正则（迁移自 task/content.rs）
    pub(crate) static ref IMG_PANFA_REGEX: Regex = Regex::new(r"/toimg/data/(?P<url>.+?.png)").unwrap();
}

/// 书籍元数据（迁移自 task/mod.rs::Book）
#[derive(Debug, Clone)]
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

/// 章节数据（迁移自 task/mod.rs::Chapter）
#[derive(Debug, Eq, PartialEq, Clone, Hash)]
pub struct Chapter {
    pub title: String,
    pub url: String,
    pub sections: Option<Vec<Section>>,
}

/// Section 数据（迁移自 task/mod.rs::Section）
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
        Chapter { url: href, title, sections: None }
    }
}

/// 解析书籍详情页，提取 Book 元数据（迁移自 task/parse.rs::get_info）。
///
/// 注意：保留原 `get_info` 的行为 —— `Book.id` 设为 0，参数 `book_id` 仅用于错误信息。
pub fn parse_book_info(book_id: usize, html: &str) -> Result<Book> {
    let html = Html::parse_document(html);

    let page_sec = Selector::parse(".pagelistbox .page")
        .map_err(|_| anyhow::anyhow!("CSS选择器错误"))?;
    let page = html
        .select(&page_sec)
        .next()
        .ok_or_else(|| anyhow::anyhow!("book:{book_id} 未找到分页元素"))?;
    let page_text = page.inner_html();
    let page: u8 = PAGE_REGEX
        .captures(&page_text)
        .ok_or_else(|| anyhow::anyhow!("book:{book_id} 分页格式异常"))?["page"]
        .parse()?;

    let book_sec = Selector::parse("h1").map_err(|_| anyhow::anyhow!("CSS选择器错误"))?;
    let book_name = html
        .select(&book_sec)
        .next()
        .ok_or_else(|| anyhow::anyhow!("book:{book_id} 未找到书名(h1)"))?
        .text()
        .next()
        .ok_or_else(|| anyhow::anyhow!("book:{book_id} h1无文本"))?
        .to_string();

    let mut introduce = String::new();
    let bd_sec = Selector::parse(".bd").map_err(|_| anyhow::anyhow!("CSS选择器错误"))?;
    if let Some(bd) = html.select(&bd_sec).next() {
        if let Some(text) = bd.text().next() {
            if !text.is_empty() {
                introduce.push_str(text);
            }
        }
    }

    let info_sec = Selector::parse(".info").map_err(|_| anyhow::anyhow!("CSS选择器错误"))?;
    let info_el = html
        .select(&info_sec)
        .next()
        .ok_or_else(|| anyhow::anyhow!("book:{book_id} 未找到.info元素"))?;
    let mut info = info_el.text();
    let author = split_second(
        info.next()
            .ok_or_else(|| anyhow::anyhow!("book:{book_id} 缺少作者信息"))?,
        "：",
    )?;
    let category = split_second(
        info.next()
            .ok_or_else(|| anyhow::anyhow!("book:{book_id} 缺少分类信息"))?,
        "：",
    )?;
    let count: u32 = split_second(
        info.next()
            .ok_or_else(|| anyhow::anyhow!("book:{book_id} 缺少字数信息"))?,
        "：",
    )?
    .parse()
    .map_err(|_| anyhow::anyhow!("book:{book_id} 字数解析失败"))?;
    let likes: u32 = split_second(
        info.next()
            .ok_or_else(|| anyhow::anyhow!("book:{book_id} 缺少喜欢数信息"))?,
        "：",
    )?
    .parse()
    .map_err(|_| anyhow::anyhow!("book:{book_id} 喜欢数解析失败"))?;

    debug!(
        "parse_book_info: 书 {} 解析成功: 书名='{}', 分页={}, 作者='{}', 字数={}, 喜欢={}",
        book_id, book_name, page, author, count, likes
    );
    Ok(Book {
        num: 0,
        id: 0,
        title: book_name.clone(),
        filename: clean_filename(&book_name),
        page,
        author,
        category,
        introduce,
        likes,
        count,
    })
}

/// 清理文件名中的非法字符（迁移自 task/content.rs::clean_filename）
pub fn clean_filename(name: &str) -> String {
    let illegal_chars = ['\\', '/', ':', '*', '?', '"', '<', '>', '|'];
    let mut filename = name.to_string();
    for c in illegal_chars {
        filename = filename.replace(c, "_");
    }
    if filename.len() >= 200 {
        filename = filename[..200].to_string();
    }
    filename
}

/// 按 pattern 分割字符串取第二段（迁移自 task/content.rs::split_second）
pub(crate) fn split_second(s: &str, pattern: &str) -> Result<String> {
    Ok(s.split(pattern)
        .collect::<Vec<&str>>()
        .get(1)
        .ok_or_else(|| anyhow::anyhow!("解析错误"))?
        .trim()
        .to_string())
}

/// 字符转 `\u{hex}` 形式（迁移自 task/content.rs::char_to_unicode）。
/// 注意：原实现即 `format!(r"\u{:x}", c as u32)`，对 'A' 输出 `\u41`（非标准 Unicode 转义），保留原行为。
pub fn char_to_unicode(c: char) -> String {
    let unicode_value: u32 = c as u32;
    format!(r"\u{:x}", unicode_value)
}

/// Vec 去重并保留原始顺序（迁移自 task/content.rs::arr_dup_rem_linked）
pub fn arr_dup_rem_linked<T: Eq + Clone + Hash>(arr: Vec<T>) -> Vec<T> {
    let mut set = HashSet::new();
    let mut uniq_arr = Vec::new();
    for ele in arr {
        let elec = ele.clone();
        if set.insert(elec) {
            uniq_arr.push(ele);
        }
    }
    uniq_arr
}

/// 将连续多个换行符规范化为两个换行（迁移自 task/content.rs::format_novel_content）
pub fn format_novel_content(content: &str) -> String {
    static RE: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r"[\r\n]+").unwrap());
    re.replace_all(content, "\n\n").to_string()
}

/// 解析章节列表页，提取所有 Chapter（迁移自 task/parse.rs::get_chapters_url 内部逻辑）。
/// 取第二个 `.chapter-list` 元素下的 `.bd .list li a` 链接，构造 Chapter 列表并去重保序。
pub fn parse_chapter_list(html: &str, root_url: &str) -> Result<Vec<Chapter>> {
    let html = Html::parse_document(html);
    let selector = Selector::parse(".chapter-list").map_err(|_| anyhow::anyhow!("CSS选择器错误"))?;
    let a_selector =
        Selector::parse(".bd .list li a").map_err(|_| anyhow::anyhow!("CSS选择器错误"))?;
    let chapter_list = html.select(&selector).nth(1);

    let mut chapters = vec![];
    if let Some(chapter_list) = chapter_list {
        for chapter in chapter_list.select(&a_selector) {
            if let Some(href) = chapter.attr("href") {
                if let Some(title) = chapter.text().next() {
                    let url = format!("{}{}", root_url, href);
                    chapters.push(Chapter::new(url, title.to_string()));
                }
            }
        }
    }
    if !chapters.is_empty() {
        chapters = arr_dup_rem_linked(chapters);
    }
    debug!("parse_chapter_list: 解析出 {} 个章节", chapters.len());
    Ok(chapters)
}

/// 解析章节页，提取所有 Section URL（迁移自 task/parse.rs::get_sections_url 内部逻辑）。
/// 从 `.chapterPages a` 文本中提取 `【num】` 取最大值作为分页数；无匹配时退化为 `<a>` 元素计数。
/// 章节URL通过 SECTION_PAGE_REGEX 拆分为 `left/right`，生成 `left/right_1.html` … `left/right_N.html`。
pub fn parse_section_urls(chapter_url: &str, html: &str) -> Result<Vec<Section>> {
    let html = Html::parse_document(html);
    let selector =
        Selector::parse(".chapterPages a").map_err(|_| anyhow::anyhow!("html解析异常"))?;
    let mut section_num = 1u8;
    let max_sec_num: u8;
    let mut sec_num_list = vec![];

    for section_l in html.select(&selector) {
        section_num += 1;
        let text = section_l.text().next().unwrap_or("【0】");
        if let Some(cap) = SECTION_NUM_REGEX.captures(text) {
            if let Ok(num) = cap["num"].to_string().parse::<u8>() {
                sec_num_list.push(num);
            }
        }
    }
    if let Some(&max) = sec_num_list.iter().max() {
        max_sec_num = max;
    } else {
        max_sec_num = section_num;
    }

    let group = SECTION_PAGE_REGEX
        .captures(chapter_url)
        .ok_or_else(|| anyhow::anyhow!("章节URL格式异常: {}", chapter_url))?;
    let left = group["left"].to_string();
    let right = group["right"].to_string();

    let mut sections: Vec<Section> = (1..=max_sec_num)
        .map(|i| Section::new(format!("{}/{}_{}.html", left, right, i)))
        .collect();
    sections = arr_dup_rem_linked(sections);
    Ok(sections)
}

/// 策略 1：直接从 `.page-content p` 提取并字典反爬（迁移自 task/content.rs）。
pub fn try_section_data1(
    html_str: &str,
    font_dict: &HashMap<String, String>,
    img_dict: &HashMap<String, String>,
) -> Result<String> {
    format_content_html(None, Some(html_str), font_dict, img_dict)
}

/// 策略 2 信号：检测页面是否需要 POST form 拉取正文（迁移自 task/content.rs）。
/// 返回 true 表示需要 follow 到 section_post 处理（具体 Request 由 callback 构造）。
pub fn needs_section_post(html_str: &str) -> bool {
    SECTION_DATA_REGEX2.is_match(html_str)
}

/// 策略 3：`var ns='...'` 索引重排解密（迁移自 task/content.rs）。
/// 解密失败或无匹配时返回空字符串。
pub fn try_section_data3(html_str: &str) -> Result<String> {
    if let Some(cap) = SECTION_DATA_REGEX3.captures(html_str) {
        let ns = &cap["ns"];
        if let Some(content) = crate::crypto::decrypt_section_data(html_str, ns) {
            if !content.is_empty() {
                return Ok(content);
            }
        }
    }
    Ok(String::new())
}

/// 策略 4：`var chapter = secret(cipher, code, ...)` AES 解密（迁移自 task/content.rs）。
/// 解密后的字节流优先按 UTF-8 解码，失败则回退到 GBK，再失败用 lossy 转换。
pub fn try_section_data4(html_str: &str) -> Result<String> {
    if let Some(cap) = SECTION_DATA_REGEX4.captures(html_str) {
        let cipher_text = &cap["cipher"];
        let code = &cap["code"];
        let content = crate::decrpyt_aes_128_cbc(cipher_text.as_bytes(), code.as_bytes())?;
        let content = String::from_utf8(content).unwrap_or_else(|e| {
            let arr = e.into_bytes();
            GBK.decode(&arr, DecoderTrap::Replace)
                .unwrap_or_else(|_| String::from_utf8_lossy(&arr).to_string())
        });
        return Ok(content);
    }
    Ok(String::new())
}

/// 字体/图片反爬 + 格式化（迁移自 task/content.rs::format_content）。
/// `html_str` 与 `html_text` 任一为 Some 即可解析；两者都为 None 返回错误。
pub fn format_content_html(
    html_str: Option<&str>,
    html_text: Option<&str>,
    font_dict: &HashMap<String, String>,
    img_dict: &HashMap<String, String>,
) -> Result<String> {
    let parsed = match (html_str, html_text) {
        (Some(s), _) => Html::parse_document(s),
        (_, Some(s)) => Html::parse_document(s),
        _ => return Err(anyhow::anyhow!("参数错误")),
    };

    let nodes = parsed
        .select(&Selector::parse(".page-content p").map_err(|_| anyhow::anyhow!("html解析失败"))?)
        .next()
        .ok_or_else(|| anyhow::anyhow!("没有page-content节点"))?
        .descendants();

    let mut content = String::new();
    for node in nodes {
        if node.value().is_text() {
            if let Some(text) = node.value().as_text() {
                let word = text.deref();
                if word.len() == 3 {
                    let uni_word = char_to_unicode(word.chars().next().unwrap());
                    if let Some(w) = font_dict.get(&uni_word) {
                        content.push_str(w);
                    } else {
                        content.push_str(word);
                    }
                } else {
                    content.push_str(word);
                }
            }
        } else if node.value().is_element() {
            if let Some(element) = node.value().as_element() {
                match element.name() {
                    "br" => content.push('\n'),
                    "img" => {
                        if let Some(src) = element.attr("src") {
                            if let Some(cap) = IMG_PANFA_REGEX.captures(src) {
                                let url = &cap["url"];
                                if let Some(w) = img_dict.get(url) {
                                    content.push_str(w);
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    content = format_novel_content(&content);
    Ok(content)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_second_extracts_after_colon() {
        let result = split_second("作者：张三", "：").unwrap();
        assert_eq!(result, "张三");
    }

    #[test]
    fn test_split_second_trims_whitespace() {
        let result = split_second("作者：  张三  ", "：").unwrap();
        assert_eq!(result, "张三");
    }

    #[test]
    fn test_split_second_missing_pattern_returns_error() {
        let result = split_second("作者张三", "：");
        assert!(result.is_err());
    }

    #[test]
    fn test_char_to_unicode_ascii() {
        assert_eq!(char_to_unicode('A'), r"\u41");
    }

    #[test]
    fn test_char_to_unicode_chinese() {
        // '书' = U+4E66
        assert_eq!(char_to_unicode('书'), r"\u4e66");
    }

    #[test]
    fn test_arr_dup_rem_linked_preserves_order() {
        let input = vec![1, 2, 1, 3, 2, 4];
        assert_eq!(arr_dup_rem_linked(input), vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_arr_dup_rem_linked_strings() {
        let input = vec!["a".to_string(), "b".to_string(), "a".to_string()];
        assert_eq!(arr_dup_rem_linked(input), vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn test_format_novel_content_collapses_multiple_newlines() {
        let input = "line1\r\n\r\n\r\nline2\nline3";
        assert_eq!(format_novel_content(input), "line1\n\nline2\n\nline3");
    }

    #[test]
    fn test_page_regex_matches_fixture() {
        let caps = PAGE_REGEX.captures("(第1/5页)当前10条/页");
        assert!(caps.is_some());
        let page: &str = &caps.unwrap()["page"];
        assert_eq!(page, "5");
    }
}
