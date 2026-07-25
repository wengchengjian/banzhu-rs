//! wisp 驱动的 banzhu 爬虫模块。
//!
//! 取代旧的 banzhuspider + task + cf 三模块。

pub mod parse;
pub mod pipeline;
pub mod stop;
// 后续 task 添加：
// pub mod callbacks;

use std::collections::HashMap;

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
