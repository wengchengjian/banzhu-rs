//! 混合方案：CfManager(stealth Chrome) 获取 cf_clearance → wreq(Chrome137) 下载

use banzhu_spider::appconfig;
use banzhu_spider::banzhuspider::BanzhuSpider;
use banzhu_spider::db::{BookRecord, ChapterRecord, SectionRecord};
use config::Config;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let config = Config::builder()
        .add_source(config::File::with_name("spider.toml"))
        .build()?;
    let config = Arc::new(config);
    let root_url = config.get_string("root_url")?;

    // CfManager 已集成到 BanzhuSpider 中，首次请求时自动获取 cf_clearance
    let spider = BanzhuSpider::new(root_url.clone(), config.clone());
    let book_id: u32 = 52024;
    let task = spider.create_download_task(book_id);
    println!("Downloading book {} ...", book_id);

    match task.download().await {
        Ok((book, chapters)) => {
            println!("\n===== 下载成功 =====");
            println!("书名: {}", book.title);
            println!("作者: {}", book.author);
            println!("字数: {} | 章节: {}", book.count, chapters.len());

            for (i, ch) in chapters.iter().take(2).enumerate() {
                if let Some(sections) = &ch.sections {
                    let preview: String = sections
                        .iter()
                        .filter_map(|s| s.content.as_ref())
                        .flat_map(|c| c.chars().take(150))
                        .collect();
                    println!("\n[{}] {}: {}", i + 1, ch.title, preview.trim());
                }
            }

            let db = appconfig::open_db()?;
            let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
            let br = BookRecord {
                id: 0,
                website_book_id: Some(book_id as i64),
                num: book.num as i64,
                title: book.title.clone(),
                filename: book.filename.clone(),
                author: book.author.clone(),
                category: book.category.clone(),
                introduce: book.introduce.clone(),
                likes: book.likes as i64,
                word_count: book.count as i64,
                page_count: book.page as i64,
                download_time: now,
            };
            let cr: Vec<_> = chapters
                .iter()
                .enumerate()
                .map(|(i, ch)| {
                    (
                        ChapterRecord {
                            id: 0,
                            book_id: 0,
                            title: ch.title.clone(),
                            url: ch.url.clone(),
                            chapter_order: (i + 1) as i64,
                        },
                        ch.sections
                            .as_ref()
                            .map(|s| {
                                s.iter()
                                    .enumerate()
                                    .map(|(j, sec)| SectionRecord {
                                        id: 0,
                                        chapter_id: 0,
                                        book_id: 0,
                                        url: sec.url.clone(),
                                        content: sec.content.clone().unwrap_or_default(),
                                        section_order: (j + 1) as i64,
                                    })
                                    .collect()
                            })
                            .unwrap_or_default(),
                    )
                })
                .collect();
            db.save_book_with_chapters(&br, &cr)
                .map(|id| println!("\nDB id={}, {}章", id, chapters.len()))
                .unwrap_or_else(|e| println!("保存失败: {}", e));
        }
        Err(e) => eprintln!("下载失败: {}", e),
    }

    Ok(())
}
