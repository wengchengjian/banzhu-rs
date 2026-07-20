use super::*;
use crate::db::{BookRecord, ChapterRecord, SectionRecord};
use axum::body::Body;
use axum::http::header;
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use std::io::Write;

// ─── Export (TXT / EPUB) ─────────────────────────────────────────────────────

#[derive(Deserialize)]
pub(crate) struct ExportQuery {
    pub format: Option<String>,
}

// GET /api/export/:bookId?format=txt|epub
pub(crate) async fn export_book(
    State(state): State<Arc<AppState>>,
    Path(book_id): Path<i64>,
    Query(q): Query<ExportQuery>,
) -> Response {
    let format = q.format.unwrap_or_else(|| "txt".to_string()).to_lowercase();

    // 在持锁期间取出所需数据，尽快释放锁
    let (book, chapters, sections) = {
        let db = state.db.lock().await;

        let book = match db.get_book(book_id) {
            Ok(Some(b)) => b,
            Ok(None) => {
                return error_response(StatusCode::NOT_FOUND, "书籍不存在");
            }
            Err(e) => {
                return error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &format!("查询失败: {}", e),
                );
            }
        };

        let chapters = match db.get_chapters_by_book(book_id) {
            Ok(c) => c,
            Err(e) => {
                return error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &format!("查询失败: {}", e),
                );
            }
        };

        let sections = match db.get_sections_by_book(book_id) {
            Ok(s) => s,
            Err(e) => {
                return error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &format!("查询失败: {}", e),
                );
            }
        };

        (book, chapters, sections)
    };

    if chapters.is_empty() {
        return error_response(StatusCode::NOT_FOUND, "该书暂无章节内容");
    }

    match format.as_str() {
        "txt" => export_txt(book, chapters, sections),
        "epub" => export_epub(book, chapters, sections),
        other => error_response(
            StatusCode::BAD_REQUEST,
            &format!("不支持的导出格式: {}（可选 txt / epub）", other),
        ),
    }
}

/// 按章节聚合 section 文本，返回 (章节标题, 章节正文) 列表，保持 chapter_order 顺序
fn group_by_chapter(
    chapters: Vec<ChapterRecord>,
    sections: Vec<SectionRecord>,
) -> Vec<(String, String)> {
    // sections 已按 chapter_id, section_order 排序；用 map 按 chapter_id 聚合
    let mut by_chapter: std::collections::HashMap<i64, String> = std::collections::HashMap::new();
    for sec in sections {
        by_chapter
            .entry(sec.chapter_id)
            .or_default()
            .push_str(&sec.content);
    }

    chapters
        .into_iter()
        .map(|ch| {
            let content = by_chapter.remove(&ch.id).unwrap_or_default();
            (ch.title, content)
        })
        .collect()
}

fn export_txt(book: BookRecord, chapters: Vec<ChapterRecord>, sections: Vec<SectionRecord>) -> Response {
    let grouped = group_by_chapter(chapters, sections);

    let mut out = String::new();
    out.push_str(&book.title);
    out.push_str("\n\n作者：");
    out.push_str(&book.author);
    out.push_str("\n\n");

    for (title, content) in grouped {
        out.push_str("\n\n");
        out.push_str(&title);
        out.push_str("\n\n");
        out.push_str(content.trim());
        out.push('\n');
    }

    let filename = sanitize_filename(&book.title);
    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "text/plain; charset=utf-8".to_string()),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{}.txt\"", filename),
            ),
        ],
        Body::from(out),
    )
        .into_response()
}

fn export_epub(book: BookRecord, chapters: Vec<ChapterRecord>, sections: Vec<SectionRecord>) -> Response {
    let grouped = group_by_chapter(chapters, sections);

    let buf = std::io::Cursor::new(Vec::new());
    let mut zip = zip::ZipWriter::new(buf);

    let stored = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Stored);
    let deflated = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    // mimetype 必须第一个且不压缩
    if write_zip(&mut zip, "mimetype", b"application/epub+zip", stored).is_err() {
        return error_response(StatusCode::INTERNAL_SERVER_ERROR, "EPUB 打包失败");
    }

    let container = r#"<?xml version="1.0" encoding="UTF-8"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles>
    <rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>
  </rootfiles>
</container>
"#;
    let _ = write_zip(&mut zip, "META-INF/container.xml", container.as_bytes(), deflated);

    let opf = build_opf(&book, grouped.len());
    let _ = write_zip(&mut zip, "OEBPS/content.opf", opf.as_bytes(), deflated);

    let ncx = build_ncx(&book, &grouped);
    let _ = write_zip(&mut zip, "OEBPS/toc.ncx", ncx.as_bytes(), deflated);

    let nav = build_nav(&grouped);
    let _ = write_zip(&mut zip, "OEBPS/nav.xhtml", nav.as_bytes(), deflated);

    for (i, (title, content)) in grouped.iter().enumerate() {
        let xhtml = build_chapter_xhtml(title, content);
        let name = format!("OEBPS/chapter_{}.xhtml", i + 1);
        let _ = write_zip(&mut zip, &name, xhtml.as_bytes(), deflated);
    }

    let cursor = match zip.finish() {
        Ok(c) => c,
        Err(_) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, "EPUB 打包失败"),
    };

    let filename = sanitize_filename(&book.title);
    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/epub+zip".to_string()),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{}.epub\"", filename),
            ),
        ],
        Body::from(cursor.into_inner()),
    )
        .into_response()
}

fn write_zip(
    zip: &mut zip::ZipWriter<std::io::Cursor<Vec<u8>>>,
    name: &str,
    data: &[u8],
    options: zip::write::SimpleFileOptions,
) -> std::io::Result<()> {
    zip.start_file(name.to_string(), options)?;
    zip.write_all(data)?;
    Ok(())
}

fn build_opf(book: &BookRecord, chapter_count: usize) -> String {
    let mut manifest = String::new();
    let mut spine = String::new();
    for i in 1..=chapter_count {
        manifest.push_str(&format!(
            "    <item id=\"chapter_{i}\" href=\"chapter_{i}.xhtml\" media-type=\"application/xhtml+xml\"/>\n"
        ));
        spine.push_str(&format!("    <itemref idref=\"chapter_{i}\"/>\n"));
    }

    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ");
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<package xmlns="http://www.idpf.org/2007/opf" version="3.0" unique-identifier="book-id">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:title>{title}</dc:title>
    <dc:creator>{author}</dc:creator>
    <dc:language>zh-CN</dc:language>
    <dc:identifier id="book-id">urn:banzhu:book:{id}</dc:identifier>
    <meta property="dcterms:modified">{now}</meta>
  </metadata>
  <manifest>
    <item id="nav" href="nav.xhtml" media-type="application/xhtml+xml" properties="nav"/>
    <item id="ncx" href="toc.ncx" media-type="application/x-dtbncx+xml"/>
{manifest}  </manifest>
  <spine toc="ncx">
{spine}  </spine>
</package>
"#,
        title = escape_xml(&book.title),
        author = escape_xml(&book.author),
        id = book.id,
    )
}

fn build_ncx(book: &BookRecord, grouped: &[(String, String)]) -> String {
    let mut nav_points = String::new();
    for (i, (title, _)) in grouped.iter().enumerate() {
        let order = i + 1;
        nav_points.push_str(&format!(
            r#"    <navPoint id="chapter_{order}" playOrder="{order}">
      <navLabel><text>{title}</text></navLabel>
      <content src="chapter_{order}.xhtml"/>
    </navPoint>
"#,
            title = escape_xml(title)
        ));
    }

    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<ncx xmlns="http://www.daisy.org/z3986/2005/ncx/" version="2005-1">
  <head>
    <meta name="dtb:uid" content="urn:banzhu:book:{id}"/>
    <meta name="dtb:depth" content="1"/>
    <meta name="dtb:totalPageCount" content="0"/>
    <meta name="dtb:maxPageNumber" content="0"/>
  </head>
  <docTitle><text>{title}</text></docTitle>
  <navMap>
{nav_points}  </navMap>
</ncx>
"#,
        id = book.id,
        title = escape_xml(&book.title),
    )
}

fn build_nav(grouped: &[(String, String)]) -> String {
    let mut items = String::new();
    for (i, (title, _)) in grouped.iter().enumerate() {
        let order = i + 1;
        items.push_str(&format!(
            "      <li><a href=\"chapter_{order}.xhtml\">{title}</a></li>\n",
            title = escape_xml(title)
        ));
    }

    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE html>
<html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops">
  <head><title>目录</title></head>
  <body>
    <nav epub:type="toc" id="toc">
      <h1>目录</h1>
      <ol>
{items}      </ol>
    </nav>
  </body>
</html>
"#
    )
}

fn build_chapter_xhtml(title: &str, content: &str) -> String {
    let paragraphs: Vec<String> = content
        .split('\n')
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty())
        .map(|p| format!("    <p>{}</p>", escape_xml(&p)))
        .collect();

    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE html>
<html xmlns="http://www.w3.org/1999/xhtml">
  <head><title>{title}</title></head>
  <body>
    <h1>{title}</h1>
{body}  </body>
</html>
"#,
        title = escape_xml(title),
        body = if paragraphs.is_empty() {
            String::new()
        } else {
            paragraphs.join("\n") + "\n"
        },
    )
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn sanitize_filename(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| match c {
            '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => c,
        })
        .collect();
    if cleaned.trim().is_empty() {
        "book".to_string()
    } else {
        cleaned
    }
}

fn error_response(status: StatusCode, msg: &str) -> Response {
    (
        status,
        [(header::CONTENT_TYPE, "application/json; charset=utf-8".to_string())],
        Body::from(
            serde_json::to_string(&json!({ "code": -1, "msg": msg }))
                .unwrap_or_else(|_| "{}".to_string()),
        ),
    )
        .into_response()
}
