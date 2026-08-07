use crate::db::Database;
use crate::event::EventBus;
use anyhow::Result;
use config::Config;
use log::{debug, info, warn};
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::Mutex;

/// 爬虫运行状态
#[derive(Debug, Clone, Serialize)]
pub struct CrawlStatus {
    pub running: bool,
    pub current_page: u32,
    pub pages_limit: u32,
    pub books_found: u32,
    pub books_downloaded: u32,
    pub books_failed: u32,
    pub books_skipped: u32,
    pub last_run: String,
    pub message: String,
}

impl Default for CrawlStatus {
    fn default() -> Self {
        Self {
            running: false,
            current_page: 0,
            pages_limit: 0,
            books_found: 0,
            books_downloaded: 0,
            books_failed: 0,
            books_skipped: 0,
            last_run: String::new(),
            message: String::new(),
        }
    }
}

/// 调度器：持有数据库 / 配置 / 事件总线，封装 crawl 相关入口。
///
/// 注意：spider 字段已随 wreq5 旧爬虫模块一并移除，crawl_* 方法暂时 stub，
/// 待迁移到 wisp 框架后重新实现。
pub struct Scheduler {
    pub status: Arc<Mutex<CrawlStatus>>,
    db: Arc<Mutex<Database>>,
    pub config: Arc<Config>,
    pub event_bus: EventBus,
    /// 引擎控制句柄（用于外部 shutdown）
    engine_control: Arc<Mutex<Option<Arc<wisp::crawl::runtime::EngineControl>>>>,
}

impl Scheduler {
    pub fn new(
        db: Arc<Mutex<Database>>,
        config: Arc<Config>,
        event_bus: EventBus,
    ) -> Self {
        Self {
            status: Arc::new(Mutex::new(CrawlStatus::default())),
            db,
            config,
            event_bus,
            engine_control: Arc::new(Mutex::new(None)),
        }
    }

    /// 通知爬虫停止（优雅关闭）。
    pub async fn shutdown(&self) {
        if let Some(control) = self.engine_control.lock().await.take() {
            info!("通知爬虫停止...");
            control.shutdown();
        }
    }

    /// 执行一次增量爬取：由定时任务 / 启动时调用，遵守 cron.enabled 开关。
    ///
    /// 与全量爬取 `crawl_full` 通过 `status.running` 互斥：
    /// 若任一爬取正在运行，本次触发直接跳过。
    pub async fn crawl_once(&self) -> Result<()> {
        let cron_enabled = self.config.get_bool("cron.enabled").unwrap_or(true);
        if !cron_enabled {
            info!("Cron is disabled, skipping crawl");
            return Ok(());
        }

        // 防重入：若已有爬取实例在运行，直接跳过（定时触发/手动触发/启动爬取并发时避免重复）
        if self.status.lock().await.running {
            info!("已有爬取任务在运行，跳过本次触发");
            return Ok(());
        }

        let pages_limit: u32 = self.config.get_int("cron.pages_limit").unwrap_or(50) as u32;
        self.run_crawl(pages_limit, "增量").await
    }

    /// 执行一次全量爬取：一次性爬完所有列表页（`crawl.full_pages_limit`，默认 5000）。
    ///
    /// 受 `crawl.full_enabled` 配置开关控制（默认 true）。不检查 cron.enabled
    /// （手动全量不受定时开关限制），但与 `crawl_once` 通过 `status.running` 互斥：
    /// 全量爬取进行中定时任务触发会自动跳过，反之亦然。
    pub async fn crawl_full(&self) -> Result<()> {
        // 配置开关：关闭时拒绝（默认开启）
        let enabled = self.config.get_bool("crawl.full_enabled").unwrap_or(true);
        if !enabled {
            info!("crawl.full_enabled=false，全量爬取已禁用");
            return Err(anyhow::anyhow!("全量爬取未启用（crawl.full_enabled=false）"));
        }

        // 防重入：与增量共用 running 锁，保证互斥
        if self.status.lock().await.running {
            info!("已有爬取任务在运行，跳过全量爬取触发");
            return Ok(());
        }

        let pages_limit: u32 = self.config.get_int("crawl.full_pages_limit").unwrap_or(5000) as u32;
        info!("全量爬取触发: pages_limit={}", pages_limit);
        self.run_crawl(pages_limit, "全量").await
    }

    /// 公共爬取执行体：构造 spider + engine 并运行。
    ///
    /// `mode` 仅用于日志与状态提示（"增量"/"全量"）。停止由「URL 队列空 + until 空页终止」
    /// 自然决定；pages_limit 决定起始列表页数量。
    async fn run_crawl(&self, pages_limit: u32, mode: &str) -> Result<()> {
        use crate::spider;

        let root_url = self
            .config
            .get_string("root_url")
            .map_err(|_| anyhow::anyhow!("spider.toml 未配置 root_url"))?;
        let concurrency: usize = self.config.get_int("cron.book_concurrency").unwrap_or(4) as usize;
        let proxy_url = self
            .config
            .get_string("wisp.proxy.url")
            .ok()
            .filter(|s| !s.is_empty())
            .filter(|_| self.config.get_bool("wisp.proxy.enabled").unwrap_or(false));
        let fetch_mode = self.config.get_string("wisp.fetch_mode").unwrap_or_else(|_| "auto".into());
        let fetch_mode = match fetch_mode.as_str() {
            "http" => wisp::fetcher::FetchMode::Http,
            "dynamic" => wisp::fetcher::FetchMode::Dynamic,
            "stealth" => wisp::fetcher::FetchMode::Stealth,
            _ => wisp::fetcher::FetchMode::Auto,
        };
        let max_pages = self.config.get_int("wisp.max_pages").ok();

        info!(
            "crawl_{} 开始: root_url={}, pages_limit={}, concurrency={}, fetch_mode={:?}, proxy={:?}, max_pages={:?}",
            if mode == "全量" { "full" } else { "once" },
            root_url, pages_limit, concurrency, fetch_mode,
            proxy_url.as_deref(), max_pages
        );

        let img_dict = Arc::new(spider::init_img_fanpa_dict());
        let font_dict = Arc::new(spider::init_font_fanpa_dict());
        debug!("反爬字典已加载: img_dict={} 条, font_dict={} 条", img_dict.len(), font_dict.len());

        // 初始化 status
        {
            let mut s = self.status.lock().await;
            s.running = true;
            s.current_page = 0;
            s.pages_limit = pages_limit;
            s.books_found = 0;
            s.books_downloaded = 0;
            s.books_failed = 0;
            s.books_skipped = 0;
            s.message = format!("{}爬取中：开始抓取列表页（前 {} 页）", mode, pages_limit);
        }

        let spider = spider::build_spider(
            root_url.clone(),
            pages_limit,
            self.db.clone(),
            self.config.clone(),
            self.event_bus.clone(),
            self.status.clone(),
            img_dict,
            font_dict,
        );
        debug!("spider 构建完成 (label=banzhu, 5 个 on_page handler + until 空页终止)");

        let mut engine_builder = wisp::crawl::Engine::infra()
            // 断点续爬：默认启用 checkpoint（./wisp-data），页数间隔 100；
            // 支持时间间隔兜底（wisp.checkpoint_interval_secs，慢速爬取场景）
            .checkpoint_with_time(
                std::sync::Arc::new(wisp::storage::FileStore::default()),
                100,
                self.config
                    .get_int("wisp.checkpoint_interval_secs")
                    .ok()
                    .filter(|v| *v > 0)
                    .map(|v| v as u64),
            )
            .max_concurrent(concurrency)
            // 注意：不在此处设置 max_pages。wisp.max_pages 改为可选（Option），
            // 仅在配置显式给出时设置，否则默认无上限。
            // 停止由「URL 队列空 + until 空页终止」自然决定，避免 5000 硬上限截断。
            .download_delay(std::time::Duration::from_millis(
                self.config.get_int("wisp.download_delay_ms").unwrap_or(500) as u64,
            ))
            .obey_robots(self.config.get_bool("wisp.obey_robots").unwrap_or(false))
            .fetch_mode(fetch_mode)
            // UA 轮换 + 固定请求头（wisp 内置 `.headers()` 会自动构造 HeadersMiddleware）
            .ua_rotation(wisp::crawl::middleware::UaRotationMiddleware::desktop())
            .headers(vec![
                ("Accept".into(), "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8".into()),
                ("Accept-Language".into(), "zh-CN,zh;q=0.9,en;q=0.8".into()),
                ("Referer".into(), root_url.clone()),
                ("Connection".into(), "keep-alive".into()),
                ("Upgrade-Insecure-Requests".into(), "1".into()),
            ])
            // 写 DB 批量管道（wisp 新 API：pipeline 也改由 EngineBuilder 挂载）
            .pipeline(Arc::new(spider::pipeline::build_banzhu_pipeline(
                self.db.clone(),
                self.event_bus.clone(),
                self.status.clone(),
            )));

        // wisp.max_pages 可选：配置显式设置才限页，否则无上限，
        // 由「URL 队列空 + until 空页终止」自然结束。
        if let Ok(max_pages) = self.config.get_int("wisp.max_pages") {
            engine_builder = engine_builder.max_pages(max_pages as usize);
        }

        if let Some(proxy) = proxy_url.as_deref() {
            engine_builder = engine_builder.proxy(proxy);
            debug!("engine 使用代理: {}", proxy);
        }

        // 配置 stealth/浏览器参数
        let headless = self.config.get_bool("wisp.stealth.headless").unwrap_or(false);
        let challenge_timeout = self.config.get_int("wisp.stealth.challenge_timeout_secs").unwrap_or(60) as u64;
        let human_mode = self.config.get_bool("wisp.stealth.human_mode").unwrap_or(true);
        let cf_ttl = self.config.get_int("wisp.stealth.cf_cookie_ttl_secs").unwrap_or(1800) as u64;

        let mut fetch_config = wisp::fetcher::FetchClientConfig::default();
        fetch_config.headless = headless;
        fetch_config.challenge_timeout = std::time::Duration::from_secs(challenge_timeout);
        fetch_config.human_mode = human_mode;
        fetch_config.cf_cookie_ttl = std::time::Duration::from_secs(cf_ttl);
        if let Some(proxy) = proxy_url.as_deref() {
            fetch_config.proxy = Some(proxy.to_string());
        }
        engine_builder = engine_builder.fetch_client_config(fetch_config);
        debug!(
            "fetch_client_config: headless={}, challenge_timeout={}s, human_mode={}, cf_cookie_ttl={}s",
            headless, challenge_timeout, human_mode, cf_ttl
        );

        let engine = engine_builder.build()?;
        info!("wisp engine 构建完成，开始爬取...");

        // 保存控制句柄，供外部 shutdown 使用
        *self.engine_control.lock().await = Some(engine.control().clone());

        let (stats, _items) = engine.run(spider).await?;
        info!("{}爬取结束: {}", mode, stats.summary());

        // 清理控制句柄
        *self.engine_control.lock().await = None;

        // 事后批量标记：所有 running 任务标 success
        {
            let db = self.db.lock().await;
            match db.mark_all_running_tasks_success() {
                Ok(_) => debug!("收尾: 所有 running 任务已标记为 success"),
                Err(e) => warn!("收尾: mark_all_running_tasks_success 失败: {e}"),
            }
        }

        // 状态更新与事件发射（先 clone 再 emit，避免持有锁）
        let final_status = {
            let mut s = self.status.lock().await;
            s.running = false;
            s.last_run = chrono::Local::now()
                .format("%Y-%m-%d %H:%M:%S")
                .to_string();
            s.message = format!("wisp 爬取完成: {}", stats.summary());
            s.clone()
        };

        self.event_bus.emit(crate::event::CrawlEvent::Status {
            running: final_status.running,
            current_page: final_status.current_page as i64,
            pages_limit: final_status.pages_limit as i64,
            books_found: final_status.books_found as i64,
            books_downloaded: final_status.books_downloaded as i64,
            books_failed: final_status.books_failed as i64,
            books_skipped: final_status.books_skipped as i64,
            message: final_status.message,
        });

        Ok(())
    }

    /// 手动下载单本书（按网站 book_id）。暂未实现 wisp 单书爬取,请通过 crawl_once 触发全量爬取。
    pub async fn crawl_book(&self, website_book_id: u32) -> Result<()> {
        warn!("crawl_book 被调用 (website_book_id={})，但单书爬取暂未实现", website_book_id);
        Err(anyhow::anyhow!("单书爬取暂未实现,请通过 crawl_once 触发全量爬取"))
    }

    /// 重新爬取指定书籍（trigger=retry）。暂未实现,请通过 crawl_once 触发全量爬取。
    pub async fn retry_book(&self, website_book_id: u32) -> Result<()> {
        warn!("retry_book 被调用 (website_book_id={})，但单书爬取暂未实现", website_book_id);
        Err(anyhow::anyhow!("单书爬取暂未实现,请通过 crawl_once 触发全量爬取"))
    }
}
