use crate::db::Database;
use crate::event::EventBus;
use anyhow::Result;
use config::Config;
use log::info;
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

    /// 执行一次增量爬取：基于 wisp 框架(legacy 路径已在 Task 1 删除)。
    pub async fn crawl_once(&self) -> Result<()> {
        use crate::spider;

        let cron_enabled = self.config.get_bool("cron.enabled").unwrap_or(true);
        if !cron_enabled {
            info!("Cron is disabled, skipping crawl");
            return Ok(());
        }

        let root_url = self
            .config
            .get_string("root_url")
            .map_err(|_| anyhow::anyhow!("spider.toml 未配置 root_url"))?;
        let pages_limit: u32 = self.config.get_int("cron.pages_limit").unwrap_or(50) as u32;
        let concurrency: usize = self.config.get_int("cron.book_concurrency").unwrap_or(4) as usize;
        let proxy_url = self
            .config
            .get_string("spider.proxy.url")
            .ok()
            .filter(|s| !s.is_empty())
            .filter(|_| self.config.get_bool("spider.proxy.enabled").unwrap_or(false));

        let img_dict = Arc::new(spider::init_img_fanpa_dict());
        let font_dict = Arc::new(spider::init_font_fanpa_dict());

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
            s.message = "wisp 后端：开始爬取".to_string();
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

        let mut engine_builder = wisp::crawl::Engine::infra()
            .max_concurrent(concurrency)
            // max_pages 是引擎级总页数上限（列表页+详情页+章节页）
            // pages_limit 只控制列表页数量，由 spider start_urls 决定
            // 设置为 pages_limit * 100 以允许详情页和章节页被处理
            .max_pages(pages_limit as usize * 100)
            .download_delay(std::time::Duration::from_millis(500))
            .obey_robots(false)
            .fetch_mode(wisp::fetcher::FetchMode::Auto);

        if let Some(proxy) = proxy_url.as_deref() {
            engine_builder = engine_builder.proxy(proxy);
        }

        // 配置 headless 模式（false = 可见浏览器，用于绕过 CF 检测）
        let headless = self.config.get_bool("spider.headless").unwrap_or(true);
        let mut fetch_config = wisp::fetcher::FetchClientConfig::default();
        fetch_config.headless = headless;
        fetch_config.challenge_timeout = std::time::Duration::from_secs(60);
        if let Some(proxy) = proxy_url.as_deref() {
            fetch_config.proxy = Some(proxy.to_string());
        }
        engine_builder = engine_builder.fetch_client_config(fetch_config);

        let engine = engine_builder.build()?;

        // 保存控制句柄，供外部 shutdown 使用
        *self.engine_control.lock().await = Some(engine.control().clone());

        let (stats, _items) = engine.run(spider).await?;

        // 清理控制句柄
        *self.engine_control.lock().await = None;

        // 事后批量标记：所有 running 任务标 success
        {
            let db = self.db.lock().await;
            let _ = db.mark_all_running_tasks_success();
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
        let _ = website_book_id;
        Err(anyhow::anyhow!("单书爬取暂未实现,请通过 crawl_once 触发全量爬取"))
    }

    /// 重新爬取指定书籍（trigger=retry）。暂未实现,请通过 crawl_once 触发全量爬取。
    pub async fn retry_book(&self, website_book_id: u32) -> Result<()> {
        let _ = website_book_id;
        Err(anyhow::anyhow!("单书爬取暂未实现,请通过 crawl_once 触发全量爬取"))
    }
}
