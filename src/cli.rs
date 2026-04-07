use crate::appconfig;
use crate::banzhuspider::BanzhuSpider;
use crate::db::{BookRecord, ChapterRecord, Database, SectionRecord};
use crate::task::BanzhuDownloadTask;
use anyhow::{anyhow, Result};
use chrono::Local;
use clap::{Parser, Subcommand};
use indicatif::MultiProgress;
use log::{error, info};
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::sync::Arc;

#[derive(Parser)]
#[command(name = "banzhu")]
#[command(version = "0.3.0")]
#[command(about = "小说爬虫命令行工具 - 支持下载、搜索、导出、预览、导入")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    #[command(about = "根据ID范围下载小说并存入数据库")]
    Download {
        #[arg(help = "小说ID范围，格式: minid-maxid 或单个ID")]
        id_range: String,
    },
    #[command(about = "从数据库导出小说")]
    Export {
        #[arg(help = "小说ID，不指定则导出全部")]
        id: Option<i64>,
        #[arg(short, long, default_value = "txt", help = "导出格式: txt 或 epub")]
        format: String,
        #[arg(short, long, default_value = "book", help = "导出目录")]
        output: String,
    },
    #[command(about = "全文搜索小说（支持标题、作者、内容）")]
    Search {
        #[arg(help = "搜索关键字")]
        keyword: String,
        #[arg(short, long, help = "精确短语匹配")]
        exact: bool,
        #[arg(
            short,
            long,
            default_value = "all",
            help = "搜索范围: all(全部), title(标题), author(作者), content(内容)"
        )]
        field: String,
        #[arg(short, long, default_value = "10", help = "每页结果数")]
        limit: i64,
        #[arg(short, long, default_value = "0", help = "结果偏移量")]
        offset: i64,
        #[arg(short, long, help = "上一页")]
        prev: bool,
        #[arg(short, long, help = "下一页")]
        next: bool,
        #[arg(long, help = "重建全文索引")]
        rebuild_index: bool,
    },
    #[command(about = "预览指定小说的特定章节内容")]
    Preview {
        #[arg(help = "小说ID")]
        id: i64,
        #[arg(help = "章节序号")]
        chapter_num: i64,
    },
    #[command(about = "批量导入本地小说文件到数据库")]
    Import {
        #[arg(help = "文件或目录路径")]
        path: String,
    },
    #[command(about = "查看或修改配置")]
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
}

#[derive(Subcommand)]
pub enum ConfigAction {
    #[command(about = "显示当前配置")]
    Show,
    #[command(about = "设置配置项")]
    Set {
        #[arg(help = "配置项名称 (save_db_path / root_url)")]
        key: String,
        #[arg(help = "配置项值")]
        value: String,
    },
}

fn open_db() -> Result<Database> {
    let db_path = appconfig::get_db_path()?;
    Database::open(&db_path)
}

pub fn run_config(action: &ConfigAction) -> Result<()> {
    match action {
        ConfigAction::Show => appconfig::show_config(),
        ConfigAction::Set { key, value } => appconfig::set_config(key, value),
    }
}

pub async fn run_download(id_range: &str) -> Result<()> {
    let root_url = appconfig::get_root_url()?;
    let db = open_db()?;

    let (start_id, end_id) = parse_id_range(id_range)?;

    info!("开始下载小说，ID范围: {} - {}", start_id, end_id);

    let config = load_legacy_config()?;
    let spider = create_spider(&config, &root_url)?;

    {
        spider.cf.write().await.bypass_cloudflare().await?;
    }

    let multi_pbr = create_multi_pbr();

    for book_id in start_id..=end_id {
        if db.book_exists(book_id as i64)? {
            println!("小说 {} 已存在于数据库中，跳过", book_id);
            continue;
        }

        println!("正在下载小说 ID: {} ...", book_id);

        let task = BanzhuDownloadTask::new(
            root_url.clone(),
            book_id,
            config.clone(),
            spider.img_fanpa_dict.clone(),
            spider.font_fanpa_dict.clone(),
            spider.client.clone(),
            spider.cf.clone(),
            multi_pbr.clone(),
            spider.spider_config.clone(),
        );

        match task.download().await {
            Ok(()) => {
                if let Err(e) = save_book_to_db(&db, &task, book_id).await {
                    error!("保存小说 {} 到数据库失败: {}", book_id, e);
                    println!("保存小说 {} 到数据库失败: {}", book_id, e);
                }
            }
            Err(e) => {
                error!("下载小说 {} 失败: {}", book_id, e);
                println!("下载小说 {} 失败: {}", book_id, e);
            }
        }
    }

    println!("下载完成");
    Ok(())
}

async fn save_book_to_db(db: &Database, task: &BanzhuDownloadTask, book_id: u32) -> Result<()> {
    let url = format!("{}/{}/{}/", task.root_url, book_id / 1000, book_id);
    let html_str = task.get(&url).await?;
    let html = scraper::Html::parse_document(&html_str);

    let book = task.get_info(book_id as usize, &html).await?;

    let book_record = BookRecord {
        id: book_id as i64,
        num: (book_id / 1000) as i64,
        title: book.title.clone(),
        filename: book.filename.clone(),
        author: book.author.clone(),
        category: book.category.clone(),
        introduce: book.introduce.clone(),
        likes: book.likes as i64,
        word_count: book.count as i64,
        page_count: book.page as i64,
        download_time: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
    };

    let chapters = task.get_chapters_content(&book).await?;

    let mut chapter_records: Vec<(ChapterRecord, Vec<SectionRecord>)> = Vec::new();

    for (idx, chapter) in chapters.iter().enumerate() {
        let chapter_record = ChapterRecord {
            id: 0,
            book_id: book_id as i64,
            title: chapter.title.clone(),
            url: chapter.url.clone(),
            chapter_order: (idx + 1) as i64,
        };

        let mut section_records = Vec::new();
        if let Some(sections) = &chapter.sections {
            for (sec_idx, section) in sections.iter().enumerate() {
                section_records.push(SectionRecord {
                    id: 0,
                    chapter_id: 0,
                    book_id: book_id as i64,
                    url: section.url.clone(),
                    content: section.content.clone().unwrap_or_default(),
                    section_order: (sec_idx + 1) as i64,
                });
            }
        }

        chapter_records.push((chapter_record, section_records));
    }

    db.save_book_with_chapters(&book_record, &chapter_records)?;
    println!("小说《{}》已保存到数据库", book.title);

    Ok(())
}

fn parse_id_range(range: &str) -> Result<(u32, u32)> {
    if range.contains('-') {
        let parts: Vec<&str> = range.split('-').collect();
        if parts.len() != 2 {
            return Err(anyhow!(
                "ID范围格式错误，正确格式: minid-maxid (如: 10-100)"
            ));
        }
        let start: u32 = parts[0]
            .trim()
            .parse()
            .map_err(|_| anyhow!("起始ID必须是数字"))?;
        let end: u32 = parts[1]
            .trim()
            .parse()
            .map_err(|_| anyhow!("结束ID必须是数字"))?;
        if start > end {
            return Err(anyhow!("起始ID不能大于结束ID"));
        }
        Ok((start, end))
    } else {
        let id: u32 = range
            .parse()
            .map_err(|_| anyhow!("ID必须是数字或范围格式 (如: 10-100)"))?;
        Ok((id, id))
    }
}

pub fn run_export(id: Option<i64>, format: &str, output_dir: &str) -> Result<()> {
    let db = open_db()?;

    let books: Vec<BookRecord> = if let Some(book_id) = id {
        vec![db
            .get_book(book_id)?
            .ok_or_else(|| anyhow!("数据库中未找到ID为 {} 的小说", book_id))?]
    } else {
        let total = db.count_books()?;
        if total == 0 {
            println!("数据库中没有小说可导出");
            return Ok(());
        }
        println!("共 {} 本小说，开始导出...", total);
        db.list_books(total, 0)?
    };

    fs::create_dir_all(output_dir)?;

    let mut success = 0;
    let mut fail = 0;

    for book in &books {
        let chapters = db.get_chapters_by_book(book.id)?;
        let sections = db.get_sections_by_book(book.id)?;

        let section_map: HashMap<i64, Vec<&SectionRecord>> = {
            let mut map: HashMap<i64, Vec<&SectionRecord>> = HashMap::new();
            for section in &sections {
                map.entry(section.chapter_id).or_default().push(section);
            }
            map
        };

        let result = match format.to_lowercase().as_str() {
            "txt" => export_txt(book, &chapters, &section_map, output_dir),
            "epub" => export_epub(book, &chapters, &section_map, output_dir),
            _ => Err(anyhow!("不支持的导出格式: {}", format)),
        };

        match result {
            Ok(()) => {
                println!("✓ 导出成功: {}", book.title);
                success += 1;
            }
            Err(e) => {
                println!("✗ 导出失败: {} - {}", book.title, e);
                fail += 1;
            }
        }
    }

    println!("\n导出完成: 成功 {} 本, 失败 {} 本", success, fail);

    if success > 0 {
        open_path_in_explorer(output_dir)?;
    }

    Ok(())
}

fn export_txt(
    book: &BookRecord,
    chapters: &[ChapterRecord],
    section_map: &HashMap<i64, Vec<&SectionRecord>>,
    output_dir: &str,
) -> Result<()> {
    let mut content = String::new();

    content.push_str(&format!("书名：{}\n", book.title));
    content.push_str(&format!("作者：{}\n", book.author));
    content.push_str(&format!("分类：{}\n", book.category));
    content.push_str(&format!("字数：{}\n", book.word_count));
    content.push_str(&format!("简介：{}\n\n", book.introduce));

    for chapter in chapters {
        content.push_str(&format!("\n{}\n\n", chapter.title));
        if let Some(sections) = section_map.get(&chapter.id) {
            for section in sections {
                if !section.content.is_empty() {
                    content.push_str(&format!("\t{}\n", section.content.trim()));
                }
            }
        }
    }

    let filename = format!("{}/{}.txt", output_dir, book.filename);
    let mut file = fs::File::create(&filename)?;
    file.write_all(content.trim().as_bytes())?;

    Ok(())
}

fn export_epub(
    book: &BookRecord,
    chapters: &[ChapterRecord],
    section_map: &HashMap<i64, Vec<&SectionRecord>>,
    output_dir: &str,
) -> Result<()> {
    use epub_builder::{EpubBuilder, EpubContent, ReferenceType, ZipLibrary};

    let zip_lib = ZipLibrary::new().map_err(|e| anyhow!("ZIP库初始化失败: {}", e))?;
    let mut builder =
        EpubBuilder::new(zip_lib).map_err(|e| anyhow!("EPUB构建器初始化失败: {}", e))?;

    builder
        .metadata("title", &book.title)
        .map_err(|e| anyhow!("设置标题失败: {}", e))?;
    builder
        .metadata("author", &book.author)
        .map_err(|e| anyhow!("设置作者失败: {}", e))?;
    builder
        .metadata("lang", "zh")
        .map_err(|e| anyhow!("设置语言失败: {}", e))?;

    let intro_html = format!(
        "<html><body><h1>{}</h1><p>作者：{}</p><p>分类：{}</p><p>简介：{}</p></body></html>",
        book.title, book.author, book.category, book.introduce
    );
    builder
        .add_content(
            EpubContent::new("intro.xhtml", intro_html.as_bytes())
                .title("简介")
                .reftype(ReferenceType::Preface),
        )
        .map_err(|e| anyhow!("添加简介失败: {}", e))?;

    for (idx, chapter) in chapters.iter().enumerate() {
        let mut body = format!("<html><body><h2>{}</h2>", chapter.title);

        if let Some(sections) = section_map.get(&chapter.id) {
            for section in sections {
                if !section.content.is_empty() {
                    let paragraphs: Vec<&str> = section.content.split('\n').collect();
                    for para in paragraphs {
                        let trimmed = para.trim();
                        if !trimmed.is_empty() {
                            body.push_str(&format!("<p>{}</p>", trimmed));
                        }
                    }
                }
            }
        }

        body.push_str("</body></html>");

        builder
            .add_content(
                EpubContent::new(format!("chapter_{}.xhtml", idx + 1), body.as_bytes())
                    .title(&chapter.title),
            )
            .map_err(|e| anyhow!("添加章节失败: {}", e))?;
    }

    let filename = format!("{}/{}.epub", output_dir, book.filename);
    let mut output = fs::File::create(&filename)?;
    builder
        .generate(&mut output)
        .map_err(|e| anyhow!("生成EPUB失败: {}", e))?;

    Ok(())
}

fn open_path_in_explorer(path: &str) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(path)
            .spawn()
            .ok();
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open").arg(path).spawn().ok();
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(path)
            .spawn()
            .ok();
    }
    Ok(())
}

pub fn run_search(
    keyword: &str,
    exact: bool,
    field: &str,
    limit: i64,
    mut offset: i64,
    prev: bool,
    next: bool,
    rebuild_index: bool,
) -> Result<()> {
    let db = open_db()?;

    if keyword.chars().count() < 2 {
        return Err(anyhow!("搜索关键词长度至少需要 2 个字符"));
    }

    if rebuild_index {
        println!("正在重建全文索引...");
        let count = db.rebuild_fts_index()?;
        println!("全文索引重建完成，共索引 {} 本小说", count);
    }

    if prev && next {
        return Err(anyhow!("不能同时使用 --prev 和 --next 参数"));
    }

    if prev {
        offset = (offset - limit).max(0);
    }

    if next {
        offset += limit;
    }

    let sf = parse_search_field(field)?;

    let total = db.search_fts_count(keyword, exact)?;

    if total == 0 {
        let results = db.search_books(keyword)?;
        if results.is_empty() {
            println!("未找到与 \"{}\" 相关的小说", keyword);
            return Ok(());
        }

        println!("搜索结果（共 {} 条）：\n", results.len());
        println!(
            "{:<8} {:<20} {:<15} {:<10} {:<10} {:<10} {}",
            "ID", "书名", "作者", "分类", "字数", "章节数", "下载时间"
        );
        println!("{}", "-".repeat(90));

        for record in &results {
            let title = truncate_str(&record.title, 18);
            let author = truncate_str(&record.author, 13);
            let category = truncate_str(&record.category, 8);

            println!(
                "{:<8} {:<20} {:<15} {:<10} {:<10} {:<10} {}",
                record.id,
                title,
                author,
                category,
                record.word_count,
                record.chapter_count,
                record.download_time,
            );
        }

        return Ok(());
    }

    let results = db.search_fts(keyword, exact, sf, limit, offset)?;

    crate::search::format_search_results(&results, keyword, sf);

    if total > limit {
        let current_page = (offset / limit) + 1;
        let total_pages = (total + limit - 1) / limit;
        println!(
            "第 {}/{} 页 (共 {} 条结果)",
            current_page, total_pages, total
        );
        if current_page > 1 {
            println!(
                "上一页: banzhu search '{}' --offset {} --limit {}",
                keyword,
                offset - limit,
                limit
            );
        }
        if current_page < total_pages {
            println!(
                "下一页: banzhu search '{}' --offset {} --limit {}",
                keyword,
                offset + limit,
                limit
            );
        }
    }

    Ok(())
}

fn parse_search_field(s: &str) -> Result<crate::search::SearchField> {
    crate::search::SearchField::from_str(s)
        .ok_or_else(|| anyhow!("不支持的搜索范围: {}，可选: all, title, author, content", s))
}

fn truncate_str(s: &str, max_len: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() > max_len {
        let mut result: String = chars[..max_len - 1].iter().collect();
        result.push('…');
        result
    } else {
        s.to_string()
    }
}

pub fn run_preview(id: i64, chapter_num: i64) -> Result<()> {
    let db = open_db()?;

    let book = db
        .get_book(id)?
        .ok_or_else(|| anyhow!("数据库中未找到ID为 {} 的小说", id))?;

    let chapter = db
        .get_chapter_by_book_and_order(id, chapter_num)?
        .ok_or_else(|| anyhow!("小说《{}》未找到第 {} 章", book.title, chapter_num))?;

    let sections = db.get_sections_by_chapter(chapter.id)?;

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("书名：{}", book.title);
    println!("章节：{}", chapter.title);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    if let Some(first_section) = sections.first() {
        if !first_section.content.is_empty() {
            println!("{}\n", first_section.content.trim());
        }
    }

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!(
        "第 {} 章 / 共 {} 节 (仅显示第1节)",
        chapter_num,
        sections.len()
    );

    Ok(())
}

fn create_multi_pbr() -> MultiProgress {
    let mp = MultiProgress::new();
    mp.set_draw_target(indicatif::ProgressDrawTarget::stdout());
    mp
}

fn load_legacy_config() -> Result<Arc<config::Config>> {
    use config::File;
    Ok(Arc::new(
        config::Config::builder()
            .add_source(File::with_name("spider.toml"))
            .build()
            .map_err(|e| anyhow!("读取spider.toml失败: {}", e))?,
    ))
}

fn create_spider(config: &Arc<config::Config>, root_url: &str) -> Result<BanzhuSpider> {
    Ok(BanzhuSpider::new(root_url.to_string(), config.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_id_range_single() {
        let (start, end) = parse_id_range("50").unwrap();
        assert_eq!(start, 50);
        assert_eq!(end, 50);
    }

    #[test]
    fn test_parse_id_range_multi() {
        let (start, end) = parse_id_range("10-100").unwrap();
        assert_eq!(start, 10);
        assert_eq!(end, 100);
    }

    #[test]
    fn test_parse_id_range_invalid() {
        assert!(parse_id_range("abc").is_err());
        assert!(parse_id_range("10-abc").is_err());
        assert!(parse_id_range("100-10").is_err());
    }

    #[test]
    fn test_truncate_str() {
        assert_eq!(truncate_str("hello", 10), "hello");
        let truncated = truncate_str("这是一个很长的小说名称用来测试截断", 10);
        assert!(truncated.chars().count() <= 11);
    }

    #[test]
    fn test_export_txt() {
        let db = Database::open_in_memory().unwrap();

        let book = BookRecord {
            id: 1,
            num: 0,
            title: "测试小说".to_string(),
            filename: "测试小说".to_string(),
            author: "测试作者".to_string(),
            category: "玄幻".to_string(),
            introduce: "这是简介".to_string(),
            likes: 100,
            word_count: 500000,
            page_count: 5,
            download_time: "2025-01-01 00:00:00".to_string(),
        };

        let chapter = ChapterRecord {
            id: 1,
            book_id: 1,
            title: "第一章 测试".to_string(),
            url: "".to_string(),
            chapter_order: 1,
        };

        let section = SectionRecord {
            id: 1,
            chapter_id: 1,
            book_id: 1,
            url: "".to_string(),
            content: "这是章节内容。".to_string(),
            section_order: 1,
        };

        db.insert_book(&book).unwrap();
        db.insert_chapter(&chapter).unwrap();
        db.insert_section(&section).unwrap();

        let mut section_map: HashMap<i64, Vec<&SectionRecord>> = HashMap::new();
        section_map.insert(1, vec![&section]);

        let output_dir = std::env::temp_dir().join("banzhu_test_export");
        let _ = fs::create_dir_all(&output_dir);

        export_txt(
            &book,
            &[chapter],
            &section_map,
            output_dir.to_str().unwrap(),
        )
        .unwrap();

        let output_file = output_dir.join("测试小说.txt");
        assert!(output_file.exists());

        let content = fs::read_to_string(&output_file).unwrap();
        assert!(content.contains("测试小说"));
        assert!(content.contains("测试作者"));
        assert!(content.contains("这是章节内容"));

        let _ = fs::remove_dir_all(&output_dir);
    }

    #[test]
    fn test_search_empty() {
        let db = Database::open_in_memory().unwrap();
        let results = db.search_books("不存在的小说").unwrap();
        assert!(results.is_empty());
    }
}
