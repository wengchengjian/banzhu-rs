use crate::appconfig::get_db_path;
use crate::db::{BookRecord, ChapterRecord, Database, SectionRecord};
use anyhow::{anyhow, Result};
use chrono::Local;
use lazy_static::lazy_static;
use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};

lazy_static! {
    static ref CHAPTER_PATTERNS: Vec<Regex> = vec![
        Regex::new(r"^第[零一二三四五六七八九十百千万\d]+[章节回卷集部篇]").unwrap(),
        Regex::new(r"^[Cc]hapter\s*\d+").unwrap(),
        Regex::new(r"^分卷阅读\s*\d+").unwrap(),
        Regex::new(r"^\(\d+\)\s*.+").unwrap(),
        Regex::new(r"^\d+[、.．]\s*.+").unwrap(),
        Regex::new(r"^卷[零一二三四五六七八九十百千万\d]+").unwrap(),
    ];
    static ref BOOK_TITLE_PATTERN: Regex = Regex::new(r"^书名[：:]").unwrap();
    static ref BOOK_AUTHOR_PATTERN: Regex = Regex::new(r"^作者[：:]").unwrap();
    static ref BOOK_INTRO_PATTERN: Regex = Regex::new(r"^简介[：:]").unwrap();
}

struct ImportResult {
    success_count: usize,
    fail_count: usize,
    failed_files: Vec<(String, String)>,
}

struct ParsedBook {
    title: String,
    author: String,
    introduce: String,
    category: String,
    chapters: Vec<ParsedChapter>,
}

struct ParsedChapter {
    title: String,
    content: String,
}

const SECTION_CHARS_WITH_CHAPTER: usize = 1000;
const SECTION_CHARS_NO_CHAPTER: usize = 2000;

pub fn run_import(path: &str) -> Result<()> {
    let db_path = get_db_path()?;
    let db = Database::open(&db_path)?;

    let path_buf = PathBuf::from(path);
    if !path_buf.exists() {
        return Err(anyhow!("路径不存在: {}", path));
    }

    let mut files: Vec<PathBuf> = Vec::new();

    if path_buf.is_file() {
        if is_supported_format(&path_buf) {
            files.push(path_buf);
        } else {
            return Err(anyhow!("不支持的文件格式，仅支持 txt 和 epub 文件"));
        }
    } else if path_buf.is_dir() {
        collect_files(&path_buf, &mut files)?;
    } else {
        return Err(anyhow!("无效路径: {}", path));
    }

    if files.is_empty() {
        println!("未找到可导入的文件");
        return Ok(());
    }

    println!("找到 {} 个文件，开始导入...\n", files.len());

    let mut result = ImportResult {
        success_count: 0,
        fail_count: 0,
        failed_files: Vec::new(),
    };

    for file_path in &files {
        match import_single_file(&db, file_path) {
            Ok(title) => {
                println!("✓ 导入成功: {}", title);
                result.success_count += 1;
            }
            Err(e) => {
                let file_name = file_path
                    .file_name()
                    .unwrap_or_default()
                    .to_str()
                    .unwrap_or("unknown");
                println!("✗ 导入失败: {} - {}", file_name, e);
                result.fail_count += 1;
                result
                    .failed_files
                    .push((file_name.to_string(), e.to_string()));
            }
        }
    }

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("导入报告:");
    println!("  成功: {} 个", result.success_count);
    println!("  失败: {} 个", result.fail_count);

    if !result.failed_files.is_empty() {
        println!("\n失败文件列表:");
        for (name, reason) in &result.failed_files {
            println!("  - {}: {}", name, reason);
        }
    }
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    Ok(())
}

fn is_supported_format(path: &Path) -> bool {
    match path.extension().and_then(|e| e.to_str()) {
        Some(ext) => matches!(ext.to_lowercase().as_str(), "txt" | "epub"),
        None => false,
    }
}

fn collect_files(dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_files(&path, files)?;
        } else if is_supported_format(&path) {
            files.push(path);
        }
    }
    Ok(())
}

fn import_single_file(db: &Database, file_path: &Path) -> Result<String> {
    let ext = file_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let parsed = match ext.as_str() {
        "txt" => parse_txt_file(file_path)?,
        "epub" => parse_epub_file(file_path)?,
        _ => return Err(anyhow!("不支持的格式: {}", ext)),
    };

    if parsed.title.is_empty() {
        let title = file_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("未知")
            .to_string();
        return Err(anyhow!("无法解析书名: {}", title));
    }

    if db.book_exists_by_title(&parsed.title)? {
        return Err(anyhow!("《{}》已存在于数据库中", parsed.title));
    }

    let filename = crate::task::clean_filename(&parsed.title);
    let word_count: i64 = parsed
        .chapters
        .iter()
        .map(|c| c.content.chars().count() as i64)
        .sum();

    let book_record = BookRecord {
        id: 0,
        num: 0,
        title: parsed.title.clone(),
        filename,
        author: parsed.author,
        category: parsed.category,
        introduce: parsed.introduce,
        likes: 0,
        word_count,
        page_count: 0,
        download_time: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
    };

    let mut chapter_records: Vec<(ChapterRecord, Vec<SectionRecord>)> = Vec::new();

    for (idx, chapter) in parsed.chapters.iter().enumerate() {
        let chapter_record = ChapterRecord {
            id: 0,
            book_id: 0,
            title: chapter.title.clone(),
            url: String::new(),
            chapter_order: (idx + 1) as i64,
        };

        let section_size = SECTION_CHARS_WITH_CHAPTER;
        let sections = split_content_into_sections(&chapter.content, 0, section_size);

        chapter_records.push((chapter_record, sections));
    }

    db.save_book_with_chapters(&book_record, &chapter_records)?;
    Ok(parsed.title)
}

fn parse_txt_file(file_path: &Path) -> Result<ParsedBook> {
    let content = fs::read_to_string(file_path).map_err(|e| anyhow!("读取文件失败: {}", e))?;

    let mut title = String::new();
    let mut author = String::new();
    let mut introduce = String::new();
    let mut category = String::new();

    let parent_dir = file_path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("其他类别")
        .to_string();
    category = parent_dir;

    let lines: Vec<&str> = content.lines().collect();
    let mut content_start = 0;

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if BOOK_TITLE_PATTERN.is_match(trimmed) {
            title = trimmed
                .splitn(2, |c| c == '：' || c == ':')
                .nth(1)
                .unwrap_or("")
                .trim()
                .to_string();
            content_start = i + 1;
        } else if BOOK_AUTHOR_PATTERN.is_match(trimmed) {
            author = trimmed
                .splitn(2, |c| c == '：' || c == ':')
                .nth(1)
                .unwrap_or("")
                .trim()
                .to_string();
            content_start = i + 1;
        } else if BOOK_INTRO_PATTERN.is_match(trimmed) {
            introduce = trimmed
                .splitn(2, |c| c == '：' || c == ':')
                .nth(1)
                .unwrap_or("")
                .trim()
                .to_string();
            content_start = i + 1;
        } else if !title.is_empty() {
            break;
        }
    }

    if title.is_empty() {
        title = file_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("未知")
            .to_string();
    }

    let body_content: String = lines[content_start..].join("\n");
    let chapters = parse_chapters(&body_content, &title);

    Ok(ParsedBook {
        title,
        author,
        introduce,
        category,
        chapters,
    })
}

fn parse_chapters(content: &str, book_title: &str) -> Vec<ParsedChapter> {
    let lines: Vec<&str> = content.lines().collect();
    let mut chapters: Vec<ParsedChapter> = Vec::new();
    let mut current_title = String::new();
    let mut current_lines: Vec<String> = Vec::new();

    let book_title_pattern = Regex::new(&format!(
        r"^{}[零一二三四五六七八九十百千万\d]+",
        regex::escape(&book_title)
    ))
    .unwrap();

    for line in &lines {
        let trimmed = line.trim();

        if is_chapter_title(trimmed, &book_title_pattern) {
            if !current_title.is_empty() || !current_lines.is_empty() {
                let chapter_content = current_lines.join("\n");
                if !chapter_content.trim().is_empty() {
                    chapters.push(ParsedChapter {
                        title: current_title.clone(),
                        content: chapter_content,
                    });
                }
            }
            current_title = trimmed.to_string();
            current_lines.clear();
        } else {
            current_lines.push(line.to_string());
        }
    }

    if !current_title.is_empty() || !current_lines.is_empty() {
        let chapter_content = current_lines.join("\n");
        if !chapter_content.trim().is_empty() {
            chapters.push(ParsedChapter {
                title: if current_title.is_empty() {
                    "正文".to_string()
                } else {
                    current_title
                },
                content: chapter_content,
            });
        }
    }

    if chapters.is_empty() {
        let section_size = SECTION_CHARS_NO_CHAPTER;
        let chars: Vec<char> = content.chars().collect();
        let total = chars.len();
        let mut pos = 0;
        let mut idx = 1;

        while pos < total {
            let end = std::cmp::min(pos + section_size, total);
            let chunk: String = chars[pos..end].iter().collect();
            chapters.push(ParsedChapter {
                title: format!("第{}章", idx),
                content: chunk,
            });
            pos = end;
            idx += 1;
        }
    }

    chapters
}

fn is_chapter_title(line: &str, book_title_pattern: &Regex) -> bool {
    if line.is_empty() || line.len() > 100 {
        return false;
    }

    for pattern in CHAPTER_PATTERNS.iter() {
        if pattern.is_match(line) {
            return true;
        }
    }

    if book_title_pattern.is_match(line) {
        return true;
    }

    false
}

fn split_content_into_sections(
    content: &str,
    book_id: i64,
    section_size: usize,
) -> Vec<SectionRecord> {
    let chars: Vec<char> = content.chars().collect();
    let total = chars.len();

    if total <= section_size {
        return vec![SectionRecord {
            id: 0,
            chapter_id: 0,
            book_id,
            url: String::new(),
            content: content.to_string(),
            section_order: 1,
        }];
    }

    let mut sections = Vec::new();
    let mut pos = 0;
    let mut order = 1;

    while pos < total {
        let end = std::cmp::min(pos + section_size, total);
        let chunk: String = chars[pos..end].iter().collect();

        sections.push(SectionRecord {
            id: 0,
            chapter_id: 0,
            book_id,
            url: String::new(),
            content: chunk,
            section_order: order,
        });

        pos = end;
        order += 1;
    }

    sections
}

fn parse_epub_file(file_path: &Path) -> Result<ParsedBook> {
    let mut title = String::new();
    let mut author = String::new();

    {
        use epub::doc::EpubDoc;

        let doc = EpubDoc::new(file_path.to_str().unwrap_or(""))
            .map_err(|e| anyhow!("EPUB文件解析失败: {}", e))?;

        title = doc
            .metadata
            .get("title")
            .and_then(|v| v.first().cloned())
            .unwrap_or_default();

        author = doc
            .metadata
            .get("creator")
            .and_then(|v| v.first().cloned())
            .unwrap_or_default();
    }

    if title.is_empty() {
        title = file_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("未知")
            .to_string();
    }

    let chapters = vec![ParsedChapter {
        title: "正文".to_string(),
        content: String::new(),
    }];

    Ok(ParsedBook {
        title,
        author,
        introduce: String::new(),
        category: String::new(),
        chapters,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_supported_format() {
        assert!(is_supported_format(Path::new("test.txt")));
        assert!(is_supported_format(Path::new("test.epub")));
        assert!(!is_supported_format(Path::new("test.pdf")));
        assert!(!is_supported_format(Path::new("test.doc")));
    }

    #[test]
    fn test_parse_chapters_with_chapter_format() {
        let content = "第一章 开始\n\n这是第一章的内容。\n\n第二章 发展\n\n这是第二章的内容。";
        let chapters = parse_chapters(content, "测试小说");
        assert!(chapters.len() >= 2);
    }

    #[test]
    fn test_parse_chapters_no_chapters() {
        let content = "这是一段没有章节标记的纯文本内容。它应该被自动分片。";
        let chapters = parse_chapters(content, "测试");
        assert!(!chapters.is_empty());
    }

    #[test]
    fn test_parse_chapters_numbered_format() {
        let content = "1、开端\n\n第一段内容\n\n2、发展\n\n第二段内容";
        let chapters = parse_chapters(content, "测试");
        assert!(chapters.len() >= 2);
    }

    #[test]
    fn test_split_content_into_sections() {
        let content = "a".repeat(2500);
        let sections = split_content_into_sections(&content, 1, 1000);
        assert_eq!(sections.len(), 3);
        assert_eq!(sections[0].section_order, 1);
        assert_eq!(sections[2].section_order, 3);
    }

    #[test]
    fn test_is_chapter_title() {
        let book_pattern = Regex::new(r"^走出吴庄[零一二三四五六七八九十百千万\d]+").unwrap();
        assert!(is_chapter_title("第一章 开始", &book_pattern));
        assert!(is_chapter_title("第十章 大战", &book_pattern));
        assert!(is_chapter_title("第二十三章", &book_pattern));
        assert!(!is_chapter_title("普通文本行", &book_pattern));
    }

    #[test]
    fn test_parse_txt_format() {
        let content = "书名：测试小说\n作者：测试作者\n简介：这是简介\n\n第一章 开始\n\n这是内容。";
        let temp_dir = std::env::temp_dir().join("banzhu_import_test");
        let _ = fs::create_dir_all(&temp_dir);
        let file_path = temp_dir.join("测试小说.txt");
        fs::write(&file_path, content).unwrap();

        let result = parse_txt_file(&file_path).unwrap();
        assert_eq!(result.title, "测试小说");
        assert_eq!(result.author, "测试作者");

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
