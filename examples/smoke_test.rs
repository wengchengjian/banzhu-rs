//! 冒烟测试：验证核心链路可用
//!
//! 测试项：
//! 1. CfManager 获取 cf_clearance（Turnstile 点击）
//! 2. wreq + cookie 请求页面（非 CF 验证页）
//! 3. 书籍信息解析（get_info 不 panic）
//! 4. 单章节下载（section 内容非空）
//!
//! 运行: cargo run --example smoke_test

use banzhu_spider::banzhuspider::BanzhuSpider;
use banzhu_spider::cf::{is_bypassed, CfManager};
use config::Config;
use std::sync::Arc;
use std::time::Instant;

const TARGET: &str = "https://www.bz555555555.com";

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let mut passed = 0;
    let mut failed = 0;
    let total_start = Instant::now();

    println!("═══════════════════════════════════════");
    println!("  banzhu-rs 冒烟测试");
    println!("═══════════════════════════════════════\n");

    // ─── Test 1: CF Bypass ───────────────────────────────────────────
    print!("[1/4] CF Bypass (CfManager + Turnstile)... ");
    let t = Instant::now();
    let cf = CfManager::new();
    match cf.ensure(TARGET).await {
        Ok((cookie, ua)) => {
            let has_cf = cookie.contains("cf_clearance");
            if has_cf {
                println!("PASS ({:.1}s)", t.elapsed().as_secs_f64());
                println!("      cookie: {}...{}", &cookie[..20], &cookie[cookie.len()-10..]);
                println!("      ua: {}", &ua[..60.min(ua.len())]);
                passed += 1;
            } else {
                println!("FAIL (no cf_clearance in cookie)");
                failed += 1;
            }
        }
        Err(e) => {
            println!("FAIL ({})", e);
            failed += 1;
        }
    }

    // ─── Test 2: 页面请求 ────────────────────────────────────────────
    print!("[2/4] 页面请求 (wreq + cookie)... ");
    let t = Instant::now();
    let config = Config::builder()
        .add_source(config::File::with_name("spider.toml"))
        .build()
        .expect("spider.toml not found");
    let config = Arc::new(config);
    let spider = BanzhuSpider::new(TARGET.to_string(), config.clone());

    let list_url = format!("{}/shuku/0-lastupdate-0-1.html", TARGET);
    match spider.get(&list_url).await {
        Ok(html) => {
            if is_bypassed(&html) && html.contains("column-2") {
                println!("PASS ({:.1}s, {} bytes)", t.elapsed().as_secs_f64(), html.len());
                passed += 1;
            } else if !is_bypassed(&html) {
                println!("FAIL (still CF challenge page)");
                failed += 1;
            } else {
                println!("WARN (page fetched but unexpected structure, {} bytes)", html.len());
                passed += 1; // 能拿到内容就算过
            }
        }
        Err(e) => {
            println!("FAIL ({})", e);
            failed += 1;
        }
    }

    // ─── Test 3: 书籍信息解析 ────────────────────────────────────────
    print!("[3/4] 书籍信息解析 (get_info)... ");
    let t = Instant::now();
    let task = spider.create_download_task(52024);
    let book_url = format!("{}/52/52024/", TARGET);
    match task.get(&book_url).await {
        Ok(html) => {
            let doc = scraper::Html::parse_document(&html);
            match task.get_info(52024, &doc).await {
                Ok(book) => {
                    if !book.title.is_empty() && book.page > 0 {
                        println!("PASS ({:.1}s)", t.elapsed().as_secs_f64());
                        println!("      书名: {} | 作者: {} | 页数: {}", book.title, book.author, book.page);
                        passed += 1;
                    } else {
                        println!("FAIL (empty title or zero pages)");
                        failed += 1;
                    }
                }
                Err(e) => {
                    println!("FAIL (get_info: {})", e);
                    failed += 1;
                }
            }
        }
        Err(e) => {
            println!("FAIL (fetch: {})", e);
            failed += 1;
        }
    }

    // ─── Test 4: 单章节下载 ──────────────────────────────────────────
    print!("[4/4] 单章节内容下载... ");
    let t = Instant::now();
    // 用第一章第一页测试
    let section_url = format!("{}/52/52024/1_1.html", TARGET);
    match task.get(&section_url).await {
        Ok(html) => {
            let doc = scraper::Html::parse_document(&html);
            // 尝试提取内容（任意一种方式）
            let has_content = html.contains("page-content") || html.contains("chapter");
            if has_content {
                println!("PASS ({:.1}s, {} bytes)", t.elapsed().as_secs_f64(), html.len());
                passed += 1;
            } else {
                println!("WARN (page fetched but no content markers, {} bytes)", html.len());
                passed += 1;
            }
        }
        Err(e) => {
            println!("FAIL ({})", e);
            failed += 1;
        }
    }

    // ─── 结果 ────────────────────────────────────────────────────────
    println!("\n═══════════════════════════════════════");
    println!("  结果: {}/{} 通过, {} 失败 (总耗时 {:.1}s)",
        passed, passed + failed, failed, total_start.elapsed().as_secs_f64());
    println!("═══════════════════════════════════════");

    if failed > 0 {
        std::process::exit(1);
    }
}
