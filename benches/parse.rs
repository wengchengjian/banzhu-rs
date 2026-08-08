//! banzhu-spider 核心路径基准测试。
//!
//! 覆盖爬虫最热门的 CPU/IO 路径：
//! - HTML 解析：书籍详情页 / 章节列表 / 章节分页
//! - 正文反爬解密：策略1（字体+图片字典）、策略3（ns 索引重排）、策略4（AES-CBC）
//! - 字符串处理：文件名清理 / 换行规范化
//! - DB 批量写入：books / chapters / sections
//!
//! 运行：cargo bench --bench parse

use banzhu_spider::crypto;
use banzhu_spider::db::{BookRecord, ChapterRecord, Database, SectionRecord};
use banzhu_spider::spider::parse::{
    clean_filename, format_novel_content, parse_book_info, parse_chapter_list,
    parse_section_urls, try_section_data1, try_section_data3, try_section_data4,
};
use banzhu_spider::spider::{init_font_fanpa_dict, init_img_fanpa_dict};
use criterion::{criterion_group, criterion_main, Criterion};

// ─── 真实规模数据构造 ──────────────────────────────────────────────────────

/// 书籍详情页：模拟真实 bz 站点 ~40KB HTML（30 本相似书籍推荐 + 元数据）。
fn build_book_detail_html() -> String {
    let mut recs = String::new();
    for i in 0..30 {
        recs.push_str(&format!(
            "<li><a href=\"/{}/{}/\"><img src=\"/toimg/data/c{i}.png\"/></a>
             <p class=\"info\"><a class=\"name\" href=\"/{}/{}/\">类似书籍{}</a></p></li>\n",
            i, 1000 + i, i, 1000 + i, i
        ));
    }
    format!(
        r#"<!DOCTYPE html><html><head><title>测试书名</title></head><body>
        <div class="pagelistbox"><span class="page">(第1/5页)当前10条/页</span></div>
        <h1>测试书名</h1>
        <div class="bd">这是一本测试书的简介内容，描述主角的成长经历与冒险故事。</div>
        <div class="info">作者：测试作者<br>分类：玄幻<br>字数：100000<br>喜欢：200</div>
        <div class="chapter-list"><div class="bd"><ul class="list">{recs}</ul></div></div>
        <div class="chapter-list"><div class="bd"><ul class="list">
        <li><a href="/12/12345_1/23456.html">第1章 开始</a></li>
        </ul></div></div>
        </body></html>"#
    )
}

/// 章节列表页：模拟真实站点 300 章。
fn build_chapter_list_html() -> String {
    let mut items = String::new();
    for i in 1..=300 {
        items.push_str(&format!(
            "<li><a href=\"/12/12345_1/{}.html\">第{}章 章节标题内容{}号</a></li>\n",
            23456 + i,
            i,
            i
        ));
    }
    format!(
        r#"<!DOCTYPE html><html><body>
        <div class="chapter-list"><div class="bd"><ul class="list"><li><a href='#'>最新章节</a></li></ul></div></div>
        <div class="chapter-list"><div class="bd"><ul class="list">{} </ul></div></div>
        </body></html>"#,
        items
    )
}

/// 章节分页页：8 个分页链接。
fn build_section_page_html() -> String {
    let mut pages = String::new();
    for i in 1..=8 {
        pages.push_str(&format!(
            "<a href=\"javascript:;\">【{}】</a>\n",
            i
        ));
    }
    format!(
        r#"<!DOCTYPE html><html><body>
        <div class="chapterPages">{pages}</div>
        </body></html>"#
    )
}

/// 正文页（策略1）：`.page-content p` 内含长文本 + 图片反爬 + 字体单字。
fn build_section_data1_html() -> String {
    let mut paras = String::new();
    for _ in 0..60 {
        paras.push_str(
            "<p>这是一个段落，包含主角在冒险旅途中的心理活动与对话描写，情节紧凑引人入胜。
             <img src=\"/toimg/data/abc123.png\">段落后继续描写环境。</p>\n",
        );
    }
    format!(
        r#"<!DOCTYPE html><html><body>
        <div class="page-content">{} </div>
        </body></html>"#,
        paras
    )
}

/// 正文页（策略3）：`var ns='...'` + `[id^="chapter"]` 内的 `<br>` 分段。
fn build_section_data3_html() -> String {
    // ns = base64("1,3,2,4,5") → "MSwzLDIsNCw1"
    let mut segs = String::new();
    for i in 1..=200 {
        segs.push_str(&format!("[标记{}]段落文本内容第{}段<br>\n", i, i));
    }
    format!(
        r#"<html><body><script>var ns='MSwzLDIsNCw1';</script>
        <div id="chapter1">{segs}</div></body></html>"#
    )
}

/// 用与生产 `decrpyt_aes_128_cbc` 一致的算法（code 的 md5 派生 key/iv）生成合法密文。
fn aes_encrypt_section(plain: &str, code: &str) -> String {
    use base64::engine::general_purpose;
    use base64::Engine as _;
    use cipher::block_padding::Pkcs7;
    use cipher::{BlockEncryptMut, KeyIvInit};

    type Aes128CbcEnc = cbc::Encryptor<aes::Aes128>;

    let m = md5::compute(code.as_bytes());
    let mx = format!("{:x}", m);
    let iv = mx[..16].bytes().collect::<Vec<_>>();
    let key = mx[16..].bytes().collect::<Vec<_>>();
    let mut buf = vec![0u8; plain.len() + 16];
    let ct = Aes128CbcEnc::new_from_slices(&key, &iv)
        .unwrap()
        .encrypt_padded_b2b_mut::<Pkcs7>(plain.as_bytes(), &mut buf)
        .unwrap();
    general_purpose::STANDARD.encode(ct)
}

/// 正文页（策略4）：`var chapter = secret(...)` AES 密文（合法密文，真实解密路径）。
fn build_section_data4_html() -> String {
    // 3000 字正文，模拟真实章节页
    let plain = "这是正文内容段落，包含主角的冒险描写与人物对话。".repeat(100);
    let cipher = aes_encrypt_section(&plain, "mycode");
    format!(
        r#"<html><body>
    <script>var chapter = secret("{}", "mycode", 1, 2);</script>
    </body></html>"#,
        cipher
    )
}

// ─── DB 记录构造 ───────────────────────────────────────────────────────────

fn make_books(n: usize) -> Vec<BookRecord> {
    (0..n)
        .map(|i| BookRecord {
            id: 0,
            website_book_id: Some(1000 + i as i64),
            path_num: 0,
            title: format!("测试书籍第{}本", i),
            filename: format!("测试书籍第{}本", i),
            author: "测试作者".to_string(),
            category: "玄幻".to_string(),
            introduce: "这是一本测试书籍的简介，内容描述主角的成长故事。".to_string(),
            likes: 200,
            word_count: 100000,
            page_count: 5,
            created_at: 0,
            updated_at: 0,
        })
        .collect()
}

fn make_chapters(n: usize) -> Vec<(i64, ChapterRecord)> {
    (0..n)
        .map(|i| {
            (
                1000i64,
                ChapterRecord {
                    id: 0,
                    book_id: 0,
                    title: format!("第{}章", i),
                    url: format!("/12/12345_1/{}.html", i),
                    chapter_order: (i + 1) as i64,
                    word_count: 0,
                },
            )
        })
        .collect()
}

fn make_sections(n: usize) -> Vec<(i64, i64, SectionRecord)> {
    (0..n)
        .map(|i| {
            (
                1000i64,
                (i + 1) as i64,
                SectionRecord {
                    id: 0,
                    chapter_id: 0,
                    book_id: 0,
                    url: format!("/12/12345_1/{}_1.html", i),
                    content: "这是正文内容段落，包含主角的冒险描写与人物对话。".repeat(40),
                    section_order: 1,
                },
            )
        })
        .collect()
}

// ─── 基准函数 ──────────────────────────────────────────────────────────────

fn bench_parse_book_info(c: &mut Criterion) {
    let html = build_book_detail_html();
    c.bench_function("parse_book_info (40KB 详情页)", |b| {
        b.iter(|| {
            let _ = parse_book_info(12345, criterion::black_box(&html)).unwrap();
        })
    });
}

fn bench_parse_chapter_list(c: &mut Criterion) {
    let html = build_chapter_list_html();
    c.bench_function("parse_chapter_list (300 章)", |b| {
        b.iter(|| {
            let _ = parse_chapter_list(criterion::black_box(&html), "https://example.com").unwrap();
        })
    });
}

fn bench_parse_section_urls(c: &mut Criterion) {
    let html = build_section_page_html();
    c.bench_function("parse_section_urls (8 分页)", |b| {
        b.iter(|| {
            let _ = parse_section_urls(
                criterion::black_box("/12/12345_1/23456.html"),
                criterion::black_box(&html),
            )
            .unwrap();
        })
    });
}

fn bench_section_data1(c: &mut Criterion) {
    let html = build_section_data1_html();
    let font_dict = init_font_fanpa_dict();
    let img_dict = init_img_fanpa_dict();
    c.bench_function("try_section_data1 (字体+图片反爬)", |b| {
        b.iter(|| {
            let _ = try_section_data1(
                criterion::black_box(&html),
                &font_dict,
                &img_dict,
            )
            .unwrap();
        })
    });
}

fn bench_section_data3(c: &mut Criterion) {
    let html = build_section_data3_html();
    c.bench_function("try_section_data3 (ns 索引重排)", |b| {
        b.iter(|| {
            let _ = try_section_data3(criterion::black_box(&html)).unwrap();
        })
    });
}

fn bench_section_data4(c: &mut Criterion) {
    let html = build_section_data4_html();
    c.bench_function("try_section_data4 (AES-CBC)", |b| {
        b.iter(|| {
            let _ = try_section_data4(criterion::black_box(&html)).unwrap();
        })
    });
}

fn bench_crypto_decrypt_section(c: &mut Criterion) {
    let html = build_section_data3_html();
    let ns = "MSwzLDIsNCw1";
    c.bench_function("crypto::decrypt_section_data", |b| {
        b.iter(|| {
            let _ = crypto::decrypt_section_data(criterion::black_box(&html), ns);
        })
    });
}

fn bench_string_helpers(c: &mut Criterion) {
    let name = "测试/书籍:名称?包含*非法|字符<>.txt";
    let content = "第一行\r\n\r\n\r\n第二行\n第三行\r\n第四行".repeat(1000);
    c.bench_function("clean_filename (非法字符替换)", |b| {
        b.iter(|| {
            let _ = clean_filename(criterion::black_box(name));
        })
    });
    c.bench_function("format_novel_content (换行规范化)", |b| {
        b.iter(|| {
            let _ = format_novel_content(criterion::black_box(&content));
        })
    });
}

fn bench_db_batch_upsert(c: &mut Criterion) {
    // 预热一次：创建内存 DB，插入 books 以便 chapters/sections 的 JOIN 命中
    let books = make_books(50);
    let chapters = make_chapters(50);
    let sections = make_sections(50);

    let mut group = c.benchmark_group("db_batch_upsert");

    group.bench_function("books (50 条)", |b| {
        b.iter(|| {
            let db = Database::open_in_memory().unwrap();
            let _ = db.batch_upsert_books(criterion::black_box(&books)).unwrap();
        })
    });

    group.bench_function("chapters (50 条)", |b| {
        b.iter(|| {
            let db = Database::open_in_memory().unwrap();
            db.batch_upsert_books(&books).unwrap();
            let _ = db.batch_upsert_chapters(criterion::black_box(&chapters)).unwrap();
        })
    });

    group.bench_function("sections (50 条)", |b| {
        b.iter(|| {
            let db = Database::open_in_memory().unwrap();
            db.batch_upsert_books(&books).unwrap();
            db.batch_upsert_chapters(&chapters).unwrap();
            let _ = db.batch_upsert_sections(criterion::black_box(&sections)).unwrap();
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_parse_book_info,
    bench_parse_chapter_list,
    bench_parse_section_urls,
    bench_section_data1,
    bench_section_data3,
    bench_section_data4,
    bench_crypto_decrypt_section,
    bench_string_helpers,
    bench_db_batch_upsert,
);
criterion_main!(benches);
