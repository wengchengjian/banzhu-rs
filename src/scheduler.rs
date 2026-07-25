use crate::db::Database;
use crate::event::{CrawlEvent, EventBus};
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
        }
    }

    /// 执行一次增量爬取（legacy 实现，待迁移到 wisp）
    pub async fn crawl_once(&self) -> Result<()> {
        let cron_enabled = self
            .config
            .get_bool("cron.enabled")
            .unwrap_or(true);
        if !cron_enabled {
            info!("Cron is disabled, skipping crawl");
            return Ok(());
        }
        // 旧 wreq5 爬虫已删除，等待 wisp 迁移
        unimplemented!("migrating to wisp")
    }

    /// 手动下载单本书（按网站 book_id），并写入爬取日志
    pub async fn crawl_book(&self, website_book_id: u32) -> Result<()> {
        let _ = website_book_id;
        unimplemented!("migrating to wisp")
    }

    /// 重新爬取指定书籍（trigger=retry）
    pub async fn retry_book(&self, website_book_id: u32) -> Result<()> {
        let _ = website_book_id;
        unimplemented!("migrating to wisp")
    }
}
