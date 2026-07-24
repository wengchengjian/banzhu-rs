use super::*;
use crate::decrpyt_aes_128_cbc;
use encoding::all::GBK;
use encoding::{DecoderTrap, Encoding};
use scraper::Selector;
use std::collections::HashSet;
use std::hash::Hash;
use std::ops::Deref;

impl BanzhuDownloadTask {
    pub(super) async fn process_section(&self, section_url: &str) -> Result<String> {
        let html_str = self.get(section_url).await?;
        let mut content = String::new();
        let html = Html::parse_document(&html_str);
        let mut need2format = false;

        if let Ok(initial_content) = self.get_section_data1(&html).await {
            content = initial_content;
        } else if let Ok(content2) = self.get_section_data2(section_url, &html).await {
            need2format = true;
            content = content2;
        } else if let Ok(content3) = self.get_section_data3(&html).await {
            need2format = true;
            content = content3;
        } else if let Ok(content4) = self.get_section_data4(&html).await {
            need2format = true;
            content = content4;
        }

        if need2format {
            content = format!("<div class=\"page-content\"><div>{}</div></div>", content);
            return self.format_content(Some(&content), None);
        }
        Ok(content)
    }

    async fn get_section_data1(&self, html: &Html) -> Result<String> {
        Ok(self.format_content(None, Some(html))?)
    }

    async fn get_section_data2(&self, url: &str, html: &Html) -> Result<String> {
        let html_str = html.html();
        let mut content = String::new();
        if SECTION_DATA_REGEX2.is_match(&html_str) {
            content = self.post_form(url, vec![("j", "1")]).await?;
        }
        Ok(content)
    }

    async fn get_section_data3(&self, html: &Html) -> Result<String> {
        let html_str = html.html();
        if let Some(cap) = SECTION_DATA_REGEX3.captures(&html_str) {
            let ns = &cap["ns"];
            if let Some(content) = crate::crypto::decrypt_section_data(&html_str, ns) {
                if !content.is_empty() {
                    return Ok(content);
                }
            }
        }
        Ok(String::new())
    }

    async fn get_section_data4(&self, html: &Html) -> Result<String> {
        let html_str = html.html();
        if let Some(cap) = SECTION_DATA_REGEX4.captures(&html_str) {
            let cipher_text = &cap["cipher"];
            let code = &cap["code"];
            let content = decrpyt_aes_128_cbc(cipher_text.as_bytes(), code.as_bytes())?;
            let content = String::from_utf8(content).unwrap_or_else(|e| {
                let arr = e.into_bytes();
                GBK.decode(&arr, DecoderTrap::Replace)
                    .unwrap_or_else(|_| String::from_utf8_lossy(&arr).to_string())
            });
            return Ok(content);
        }
        Ok("".to_string())
    }

    fn format_content(&self, html_str: Option<&str>, html: Option<&Html>) -> Result<String> {
        let mut html2 = None;
        if let Some(html_str) = html_str {
            html2 = Some(Html::parse_document(html_str))
        }

        let html = {
            if let Some(html) = html {
                Some(html)
            } else if let Some(_html_str) = html_str {
                Some(html2.as_ref().unwrap())
            } else {
                None
            }
        };

        if let Some(html) = html {
            let nodes = html
                .select(&Selector::parse(".page-content p").map_err(|_e| anyhow!("html解析失败"))?)
                .next()
                .ok_or(anyhow!("没有page-content节点"))?
                .descendants();

            let mut content = String::new();
            for node in nodes {
                if node.value().is_text() {
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
                        match element.name() {
                            "br" => {
                                content.push('\n');
                            }
                            "img" => {
                                if let Some(src) = element.attr("src") {
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
            content = format_novel_content(&content);
            return Ok(content);
        };

        Err(anyhow!("参数错误"))
    }
}

pub fn char_to_unicode(c: char) -> String {
    let unicode_value: u32 = c as u32;
    format!(r"\u{:x}", unicode_value)
}

/// 检查页面是否通过 Cloudflare（二次确认）
pub(crate) fn is_bypassed_after_fetch(html: &str) -> bool {
    crate::cf::is_cf_challenge(html)
}

pub(crate) fn split_second(s: &str, pattern: &str) -> Result<String> {
    Ok(s.split(pattern)
        .collect::<Vec<&str>>()
        .get(1)
        .ok_or(anyhow!("解析错误"))?
        .trim()
        .to_string())
}

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

pub fn format_novel_content(content: &str) -> String {
    static RE: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r"[\r\n]+").unwrap());
    re.replace_all(content, "\n\n").to_string()
}
