//! 测试 headless_chrome CDP 浏览器过 Cloudflare
//!
//! 原理：真实 Chrome + CDP 控制 → 等待 Turnstile/JS Challenge 通过 → 提取 cookie + UA
//! 优点：100% 真实浏览器指纹，CF 无法区分
//! 缺点：需要安装 Chrome，内存占用 ~200MB，启动慢 (~3s)

use headless_chrome::{Browser, LaunchOptions};
use std::time::Duration;

const TARGET: &str = "https://www.bz555555555.com/52/52024/";

fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    println!("=== headless_chrome 测试 ===");
    println!("目标: {}", TARGET);
    println!("策略: 真实 Chrome (headed) → 手动/自动等 CF 通过 → 提取 Cookie\n");

    // 启动浏览器 (headed 模式，可以看到 CF 验证过程)
    let browser = Browser::new(
        LaunchOptions::default_builder()
            .headless(false)
            .window_size(Some((1024, 768)))
            .sandbox(false)
            .build()?,
    )?;

    let tab = browser.new_tab()?;
    println!("[1] 导航到目标网站...");
    tab.navigate_to(TARGET)?;
    tab.wait_until_navigated()?;

    println!("[2] 等待 Cloudflare 验证 (最多 90 秒)...");
    let mut passed = false;
    for i in 0..45 {
        std::thread::sleep(Duration::from_secs(2));
        match tab.get_content() {
            Ok(html) => {
                let is_cf = html.contains("Just a moment")
                    || html.contains("请稍候")
                    || html.contains("cf-browser-verify")
                    || html.contains("challenges.cloudflare.com");
                let len = html.len();
                if !is_cf && len > 1000 {
                    println!("[✓] CF 验证通过! ({}s, HTML {} 字节)", (i + 1) * 2, len);
                    passed = true;

                    // 检查是否是目标页面内容
                    if html.contains("天汉风云") || html.contains("52024") {
                        println!("[✓] 成功获取小说页面内容!");
                    }
                    break;
                }
                if i % 5 == 0 {
                    println!(
                        "    等待中 ({:.0}s)... HTML {} 字节, CF: {}",
                        (i + 1) * 2,
                        len,
                        is_cf
                    );
                }
            }
            Err(_) => println!("    获取内容失败..."),
        }
    }

    if passed {
        // 提取 Cookie
        let cookies = tab.get_cookies()?;
        let cf_cookie: Vec<_> = cookies
            .iter()
            .filter(|c| c.name == "cf_clearance")
            .collect();
        let all_cookies: Vec<_> = cookies
            .iter()
            .map(|c| format!("{}={}", c.name, c.value))
            .collect();

        println!("\n=== 结果 ===");
        println!("cf_clearance: {}", if cf_cookie.is_empty() { "❌ 未找到" } else { "✅ 已获取" });
        println!("Cookie 总数: {}", cookies.len());
        println!(
            "Cookie: {}",
            all_cookies.join("; ").chars().take(500).collect::<String>()
        );
    } else {
        println!("\n❌ 超时: CF 验证未通过");
    }

    tab.close(true)?;
    println!("\n完成");
    Ok(())
}
