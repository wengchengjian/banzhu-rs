mod turnstile;

pub(crate) use turnstile::try_click_turnstile;

use headless_chrome::{Browser, LaunchOptions};
use log::{debug, info, warn};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, RwLock};

/// 检测页面是否为 Cloudflare 验证页
pub fn is_cf_challenge(html: &str) -> bool {
    html.contains("Just a moment")
        || html.contains("请稍候")
        || html.contains("cf-browser-verify")
        || html.contains("challenges.cloudflare.com")
}

/// 检查页面是否已通过 CF 验证（非验证页 = 已绕过）
pub fn is_bypassed(html: &str) -> bool {
    !is_cf_challenge(html)
}

struct CachedCookies {
    cookie: String,
    user_agent: String,
    obtained_at: Instant,
}

/// Cloudflare cookie 生命周期管理器（singleflight 模式）
///
/// - 缓存命中：RwLock 读锁，零阻塞
/// - 缓存过期：只有一个任务启动 Chrome 刷新，其他任务等待结果
/// - 支持 JS Challenge（被动等待）和 Turnstile（CDP 点击）
pub struct CfManager {
    /// 缓存（RwLock 允许并发读）
    cached: RwLock<Option<CachedCookies>>,
    /// 刷新锁（保证同一时刻只有一个 Chrome 实例）
    refresh_lock: Mutex<()>,
    ttl: Duration,
    headless: bool,
    /// 代理 URL（与 wreq 共享，保证出口 IP 一致，避免 cf_clearance 失效）
    proxy_url: Option<String>,
}

impl CfManager {
    /// 创建 CfManager，默认 TTL 20 分钟，有头模式
    pub fn new() -> Self {
        Self {
            cached: RwLock::new(None),
            refresh_lock: Mutex::new(()),
            ttl: Duration::from_secs(20 * 60),
            headless: false,
            proxy_url: None,
        }
    }

    /// 从配置创建 CfManager
    ///
    /// proxy_url: 与 wreq 共享的代理地址（如 "http://127.0.0.1:7897"）。
    /// 传入后 Chrome 启动时也会走该代理，确保 cf_clearance 与 wreq 请求出口 IP 一致。
    pub fn with_config(ttl: Duration, headless: bool, proxy_url: Option<String>) -> Self {
        Self {
            cached: RwLock::new(None),
            refresh_lock: Mutex::new(()),
            ttl,
            headless,
            proxy_url,
        }
    }

    /// 创建 CfManager，自定义 TTL
    pub fn with_ttl(ttl: Duration) -> Self {
        Self {
            cached: RwLock::new(None),
            refresh_lock: Mutex::new(()),
            ttl,
            headless: false,
            proxy_url: None,
        }
    }

    /// 获取有效的 CF cookies（singleflight）
    ///
    /// 快速路径：缓存未过期 → RwLock 读锁直接返回
    /// 慢速路径：缓存过期 → 获取 refresh_lock → 双重检查 → 启动 Chrome
    pub async fn ensure(&self, domain: &str) -> anyhow::Result<(String, String)> {
        // 快速路径：读缓存
        {
            let cached = self.cached.read().await;
            if let Some(ref c) = *cached {
                if c.obtained_at.elapsed() < self.ttl {
                    debug!(
                        "Using cached cf_clearance (age: {:.0}s)",
                        c.obtained_at.elapsed().as_secs_f64()
                    );
                    return Ok((c.cookie.clone(), c.user_agent.clone()));
                }
            }
        }

        // 慢速路径：获取刷新锁（只有一个任务能进入）
        let _guard = self.refresh_lock.lock().await;

        // 双重检查：等锁期间可能已被其他任务刷新
        {
            let cached = self.cached.read().await;
            if let Some(ref c) = *cached {
                if c.obtained_at.elapsed() < self.ttl {
                    debug!("cf_clearance refreshed by another task while waiting");
                    return Ok((c.cookie.clone(), c.user_agent.clone()));
                }
            }
        }

        // 实际刷新
        info!("cf_clearance expired, launching Chrome to refresh...");
        let (cookie, ua) = Self::acquire(domain, self.headless, self.proxy_url.clone()).await?;
        {
            let mut cached = self.cached.write().await;
            *cached = Some(CachedCookies {
                cookie: cookie.clone(),
                user_agent: ua.clone(),
                obtained_at: Instant::now(),
            });
        }
        Ok((cookie, ua))
    }

    /// 强制刷新 cookie（跳过 TTL 检查，但仍走 singleflight）
    pub async fn refresh(&self, domain: &str) -> anyhow::Result<(String, String)> {
        let _guard = self.refresh_lock.lock().await;
        info!("Force refreshing cf_clearance...");
        let (cookie, ua) = Self::acquire(domain, self.headless, self.proxy_url.clone()).await?;
        {
            let mut cached = self.cached.write().await;
            *cached = Some(CachedCookies {
                cookie: cookie.clone(),
                user_agent: ua.clone(),
                obtained_at: Instant::now(),
            });
        }
        Ok((cookie, ua))
    }

    /// 启动 stealth Chrome 获取 cf_clearance（异步包装）
    async fn acquire(domain: &str, headless: bool, proxy_url: Option<String>) -> anyhow::Result<(String, String)> {
        info!("Launching stealth Chrome for CF bypass...");
        if let Some(ref p) = proxy_url {
            info!("Chrome will use proxy: {} (与 wreq 共享出口 IP)", p);
        }
        let domain = domain.to_string();
        tokio::task::spawn_blocking(move || Self::acquire_blocking(&domain, headless, proxy_url)).await?
    }

    /// 同步版本：stealth Chrome 获取 cf_clearance
    fn acquire_blocking(domain: &str, headless: bool, proxy_url: Option<String>) -> anyhow::Result<(String, String)> {
        // 构造启动参数：基础反检测 + 代理（如有）
        let mut args: Vec<String> = vec![
            "--disable-blink-features=AutomationControlled".into(),
            "--disable-infobars".into(),
            "--disable-gpu".into(),
            "--no-first-run".into(),
            "--no-default-browser-check".into(),
        ];
        if let Some(ref p) = proxy_url {
            args.push(format!("--proxy-server={}", p));
        }

        let browser = Browser::new(
            LaunchOptions::default_builder()
                .headless(headless)
                .window_size(Some((1280, 900)))
                .sandbox(false)
                .idle_browser_timeout(Duration::from_secs(180))
                .args(args.iter().map(|s| s.as_ref()).collect::<Vec<_>>())
                .build()?,
        )
        .map_err(|e| {
            anyhow::anyhow!(
                "Failed to launch Chrome: {}. \
                 Make sure Chrome/Chromium is installed and in PATH.",
                e
            )
        })?;

        let tab = Arc::new(browser.new_tab()?);

        // 注入反检测脚本（在任何页面 JS 之前执行）
        let stealth_js = r#"
(function() {
    const o = (obj, prop, value) => Object.defineProperty(obj, prop, {
        get: () => value, enumerable: true, configurable: true
    });
    o(navigator, 'webdriver', false);
    o(navigator, 'plugins', [1,2,3,4,5]);
    o(navigator, 'languages', ['zh-CN','zh','en']);
    o(navigator, 'hardwareConcurrency', 8);
    o(navigator, 'deviceMemory', 8);
    o(navigator, 'platform', 'Win32');
    if (!window.chrome) { window.chrome = { runtime: {} }; }
    if (!navigator.connection) {
        o(navigator, 'connection', {
            downlink: 10, effectiveType: '4g', rtt: 50, saveData: false
        });
    }
    // 隐藏自动化相关属性
    delete navigator.__proto__.webdriver;
})();
"#;
        tab.call_method(
            headless_chrome::protocol::cdp::Page::AddScriptToEvaluateOnNewDocument {
                source: stealth_js.to_string(),
                world_name: None,
                include_command_line_api: Some(true),
                run_immediately: None,
            },
        )?;

        // 导航到目标（CF 挑战页会重定向，不用 wait_until_navigated）
        tab.navigate_to(domain)?;
        std::thread::sleep(Duration::from_secs(3));

        // 等待 CF 验证通过（最多 120 秒），期间主动点击 Turnstile
        let start = Instant::now();
        let mut bypassed = false;
        let mut click_count = 0u32;
        let passive_wait = Duration::from_secs(2); // 前 2 秒被动等待 JS Challenge
        let click_interval = Duration::from_secs(2); // 之后每 2 秒尝试点击
        let mut last_click = Instant::now();

        loop {
            let elapsed = start.elapsed();
            if elapsed > Duration::from_secs(120) {
                warn!("CF bypass timed out (120s)");
                break;
            }

            // 检查是否已通过
            if let Ok(html) = tab.get_content() {
                if is_bypassed(&html) && html.len() > 1000 {
                    info!("CF challenge passed after {:.0}s", elapsed.as_secs_f64());
                    bypassed = true;
                    break;
                }
            }

            // 检查 cf_clearance cookie（有时页面还没刷新但 cookie 已拿到）
            if let Ok(cookies) = tab.get_cookies() {
                if cookies.iter().any(|c| c.name == "cf_clearance") {
                    // 多等 2 秒让页面完成跳转
                    std::thread::sleep(Duration::from_secs(2));
                    info!("cf_clearance cookie detected after {:.0}s", elapsed.as_secs_f64());
                    bypassed = true;
                    break;
                }
            }

            // 被动等待期过后，主动点击 Turnstile
            if elapsed > passive_wait && last_click.elapsed() >= click_interval {
                click_count += 1;
                if try_click_turnstile(&tab, click_count) {
                    debug!("[click #{}] Turnstile click dispatched", click_count);
                } else if click_count <= 3 || click_count % 5 == 0 {
                    debug!("[click #{}] Turnstile widget not found yet", click_count);
                }
                last_click = Instant::now();
            }

            std::thread::sleep(Duration::from_millis(500));
        }

        if !bypassed {
            warn!("CF bypass incomplete, extracting cookies anyway...");
        }

        // 提取 cookies
        let cookies = tab.get_cookies()?;
        let cf_cookie: String = cookies
            .iter()
            .filter(|c| c.name == "cf_clearance")
            .map(|c| format!("{}={}", c.name, c.value))
            .collect::<Vec<_>>()
            .join("; ");

        let all_cookies: String = cookies
            .iter()
            .map(|c| format!("{}={}", c.name, c.value))
            .collect::<Vec<_>>()
            .join("; ");

        // 获取浏览器真实 UA（用于后续请求保持一致）
        let ua = tab
            .evaluate("navigator.userAgent", false)
            .ok()
            .and_then(|r| r.value)
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_else(|| {
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
                 (KHTML, like Gecko) Chrome/137.0.0.0 Safari/537.36"
                    .to_string()
            });

        let cookie_str = if cf_cookie.is_empty() {
            all_cookies
        } else {
            cf_cookie
        };

        info!(
            "Cookies acquired ({} bytes), cf_clearance: {}, clicks: {}",
            cookie_str.len(),
            cookie_str.contains("cf_clearance"),
            click_count
        );

        tab.close(true)?;
        Ok((cookie_str, ua))
    }
}

impl Default for CfManager {
    fn default() -> Self {
        Self::new()
    }
}
