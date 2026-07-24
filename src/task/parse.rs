use super::*;
use scraper::selectable::Selectable;
use scraper::Selector;

impl BanzhuDownloadTask {
    pub async fn get_info(&self, id: usize, html: &Html) -> Result<Book> {
        let page_sec = Selector::parse(".pagelistbox .page").map_err(|_| anyhow!("CSS选择器错误"))?;
        let page = html
            .select(&page_sec)
            .next()
            .ok_or(anyhow!("book:{id} 未找到分页元素"))?;
        let page_text = page.inner_html();
        let page: u8 = PAGE_REGEX
            .captures(&page_text)
            .ok_or(anyhow!("book:{id} 分页格式异常: {}", &page_text[..50.min(page_text.len())]))?
            ["page"]
            .to_string()
            .parse()?;

        let book_sec = Selector::parse("h1").map_err(|_| anyhow!("CSS选择器错误"))?;
        let book_name = html
            .select(&book_sec)
            .next()
            .ok_or(anyhow!("book:{id} 未找到书名(h1)"))?
            .text()
            .next()
            .ok_or(anyhow!("book:{id} h1无文本"))?
            .to_string();

        let mut introduce = String::new();

        let bd_sec = Selector::parse(".bd").map_err(|_| anyhow!("CSS选择器错误"))?;

        let bd = html.select(&bd_sec).next();
        if let Some(bd) = bd {
            if let Some(text) = bd.text().next() {
                if text.len() != 0 {
                    introduce.push_str(text);
                }
            }
        }
        let info_sec = Selector::parse(".info").map_err(|_| anyhow!("CSS选择器错误"))?;
        let info_el = html.select(&info_sec).next()
            .ok_or(anyhow!("book:{id} 未找到.info元素"))?;
        let mut info = info_el.text();
        let author = split_second(info.next().ok_or(anyhow!("book:{id} 缺少作者信息"))?, "：")?;
        let book_category = split_second(info.next().ok_or(anyhow!("book:{id} 缺少分类信息"))?, "：")?;
        let book_count: u32 = split_second(info.next().ok_or(anyhow!("book:{id} 缺少字数信息"))?, "：")?
            .parse().map_err(|_| anyhow!("book:{id} 字数解析失败"))?;
        let book_like: u32 = split_second(info.next().ok_or(anyhow!("book:{id} 缺少喜欢数信息"))?, "：")?
            .parse().map_err(|_| anyhow!("book:{id} 喜欢数解析失败"))?;

        let filename = clean_filename(&book_name);

        let book = Book {
            num: 0,
            id: 0,
            title: book_name,
            filename,
            page,
            author,
            category: book_category,
            introduce,
            likes: book_like,
            count: book_count,
        };

        return Ok(book);
    }

    pub async fn get_chapters_url(&self, page_urls: Vec<String>) -> Result<Vec<Chapter>> {
        info!("正在获取Chapter URL...");
        let mut chapters = vec![];
        let concurrency = 8;
        let result = stream::iter(page_urls)
            .map(|url| async move {
                let mut chapters = vec![];
                let content = self.get(&url).await?;

                let html = Html::parse_document(&content);
                let selector = Selector::parse(".chapter-list").unwrap();
                let a_selector = Selector::parse(".bd .list li a").unwrap();
                let chapter_list = html.select(&selector).nth(1);
                if let Some(chapter_list) = chapter_list {
                    for chapter in chapter_list.select(&a_selector) {
                        if let Some(href) = chapter.attr("href") {
                            if let Some(title) = chapter.text().next() {
                                let url = format!("{}{}", self.root_url, href);
                                chapters.push(Chapter::new(url, title.to_string()))
                            }
                        }
                    }
                }
                anyhow::Ok(chapters)
            })
            .buffered(concurrency)
            .collect::<Vec<_>>()
            .await;
        for ret in result {
            match ret {
                Ok(chapter) => {
                    if !chapter.is_empty() {
                        chapters.extend(chapter);
                    }
                }
                Err(e) => return Err(anyhow!("获取chapter失败:{}", e)),
            }
        }
        if !chapters.is_empty() {
            chapters = arr_dup_rem_linked(chapters);
        }
        Ok(chapters)
    }

    pub async fn get_sections_url(&self, chapters: &mut Vec<Chapter>) -> Result<()> {
        debug!("正在获取Section URL...");
        let concurrency = 8;

        let result: Vec<Result<()>> = stream::iter(chapters)
            .map(|chapter| {
                async move {
                    let html_str = self.get(&chapter.url).await?;
                    let html = Html::parse_document(&html_str);
                    let selector =
                        Selector::parse(".chapterPages a").map_err(|_e| anyhow!("html解析异常"))?;
                    let section_list = html.select(&selector);
                    let mut sections = vec![];

                    let mut section_num = 1;
                    let mut sec_num_list = vec![];
                    for section_l in section_list {
                        section_num += 1;
                        let text = section_l.text().next().unwrap_or("【0】");
                        if let Some(cap) = SECTION_NUM_REGEX.captures(text) {
                            if let Ok(num) = cap["num"].to_string().parse::<u8>() {
                                sec_num_list.push(num);
                            }
                        }
                    }
                    let mut max_sec_num = section_num;
                    if let Some(&max) = sec_num_list.iter().max() {
                        max_sec_num = max;
                    }

                    let group = SECTION_PAGE_REGEX.captures(chapter.url.as_str())
                        .ok_or(anyhow!("章节URL格式异常: {}", chapter.url))?;

                    let left = group["left"].to_string();
                    let right = group["right"].to_string();

                    for i in 1..max_sec_num + 1 {
                        sections.push(Section::new(format!("{}/{}_{}.html", left, right, i)));
                    }
                    sections = arr_dup_rem_linked(sections);

                    chapter.sections = Some(sections);
                    Ok(())
                }
            })
            .buffer_unordered(concurrency)
            .collect::<Vec<_>>()
            .await;
        for ret in result {
            if ret.is_err() {
                return Err(anyhow!("获取section_url失败: {}", ret.unwrap_err()));
            }
        }
        Ok(())
    }
}
