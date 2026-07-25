//! banzhu 站点解析自由函数（从 task/parse.rs + content.rs 迁移，去 &self 依赖）。
//! 所有公开函数只接受 &str，内部用 scraper::Html 解析。

use std::fmt::{Display, Formatter};

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
