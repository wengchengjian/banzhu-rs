use crate::banzhuspider::BanzhuSpider;
use crate::db::Database;
use crate::event::{CrawlEvent, EventBus};
use anyhow::Result;
use config::Config;
use futures::stream::StreamExt;
use log::{error, info};
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

pub struct Scheduler {
    pub status: Arc<Mutex<CrawlStatus>>,
    spider: Arc<BanzhuSpider>,
    db: Arc<Mutex<Database>>,
    pub config: Arc<Config>,
    pub event_bus: EventBus,
}

impl Scheduler {
    pub fn new(
        spider: Arc<BanzhuSpider>,
        db: Arc<Mutex<Database>>,
        config: Arc<Config>,
        event_bus: EventBus,
    ) -> Self {
        Self {
            status: Arc::new(Mutex::new(CrawlStatus::default())),
            spider,
            db,
            config,
            event_bus,
        }
    }

    /// 执行一次增量爬取
    pub async fn crawl_once(&self) -> Result<()> {
        let cron_enabled = self
            .config
            .get_bool("cron.enabled")
            .unwrap_or(true);
        if !cron_enabled {
            info!("Cron is disabled, skipping crawl");
            return Ok(());
        }

        let pages_limit: u32 = self.config.get_int("cron.pages_limit").unwrap_or(50) as u32;

        {
            let mut status = self.status.lock().await;
            status.running = true;
            status.current_page = 0;
            status.pages_limit = pages_limit;
            status.books_found = 0;
            status.books_downloaded = 0;
            status.books_failed = 0;
            status.books_skipped = 0;
            status.message = "开始爬取".to_string();
        }

        self.event_bus.emit(CrawlEvent::Status {
            running: true,
            current_page: 0,
            pages_limit: pages_limit as i64,
            books_found: 0,
            books_downloaded: 0,
            books_failed: 0,
            books_skipped: 0,
            message: "开始爬取".to_string(),
        });

        info!("Starting incremental crawl (max {} pages)...", pages_limit);

        let mut skipped_streak = 0;
        let mut total_found = 0u32;
        let mut total_downloaded = 0u32;
        let mut total_failed = 0u32;
        let total_skipped = 0u32;

        for page in 1..=pages_limit {
            {
                let mut status = self.status.lock().await;
                status.current_page = page;
                status.message = format!("正在爬取第 {} 页", page);
            }

            self.event_bus.emit(CrawlEvent::Status {
                running: true,
                current_page: page as i64,
                pages_limit: pages_limit as i64,
                books_found: total_found as i64,
                books_downloaded: total_downloaded as i64,
                books_failed: total_failed as i64,
                books_skipped: total_skipped as i64,
                message: format!("正在爬取第 {} 页", page),
            });

            match self.spider.fetch_latest_list(page).await {
                Ok(books) => {
                    if books.is_empty() {
                        skipped_streak += 1;
                        if skipped_streak >= 3 {
                            info!("连续 {} 页无更新，停止爬取", skipped_streak);
                            break;
                        }
                        continue;
                    }
                    skipped_streak = 0;
                    total_found += books.len() as u32;

                    // 并发下载（从配置读取，默认 4）
                    let concurrency = self.config.get_int("cron.book_concurrency").unwrap_or(4) as usize;
                    let results: Vec<_> = futures::stream::iter(books)
                        .map(|summary| async move {
                            let db = self.db.lock().await;
                            let exists = db.book_exists_by_website_id(summary.book_id as i64).unwrap_or(false);
                            let existing_chapters = if exists {
                                db.get_chapters_count_by_website_id(summary.book_id as i64).unwrap_or(0)
                            } else {
                                0
                            };
                            // 写入 pending 状态任务
                            let _ = db.upsert_crawl_task_pending(
                                summary.book_id as i64,
                                &summary.title,
                                "cron",
                            );
                            drop(db);

                            let task = self.spider.create_download_task(summary.book_id);

                            let result = if exists && existing_chapters > 0 {
                                info!("增量更新: {} (id={}, 已有{}章)", summary.title, summary.book_id, existing_chapters);
                                // 标记为 running
                                {
                                    let db = self.db.lock().await;
                                    let _ = db.mark_crawl_task_running(summary.book_id as i64);
                                }
                                task.download_incremental(existing_chapters).await
                            } else {
                                info!("新书下载: {} (id={})", summary.title, summary.book_id);
                                {
                                    let db = self.db.lock().await;
                                    let _ = db.mark_crawl_task_running(summary.book_id as i64);
                                }
                                task.download().await
                            };

                            (summary, result)
                        })
                        .buffer_unordered(concurrency)
                        .collect()
                        .await;

                    for (summary, result) in results {
                        let website_id = summary.book_id as i64;
                        match result {
                            Ok((book, chapters)) => {
                                let book_record = crate::db::BookRecord {
                                    id: 0,
                                    website_book_id: Some(website_id),
                                    path_num: book.num as i64,
                                    title: book.title.clone(),
                                    filename: book.filename.clone(),
                                    author: book.author.clone(),
                                    category: book.category.clone(),
                                    introduce: book.introduce.clone(),
                                    likes: book.likes as i64,
                                    word_count: book.count as i64,
                                    page_count: book.page as i64,
                                    created_at: chrono::Utc::now().timestamp(),
                                    updated_at: chrono::Utc::now().timestamp(),
                                };

                                let chapter_records: Vec<_> = chapters
                                    .iter()
                                    .enumerate()
                                    .map(|(i, ch)| {
                                        let sections = ch
                                            .sections
                                            .as_ref()
                                            .map(|s| {
                                                s.iter()
                                                    .enumerate()
                                                    .map(|(j, sec)| {
                                                        crate::db::SectionRecord {
                                                            id: 0,
                                                            chapter_id: 0,
                                                            book_id: 0,
                                                            url: sec.url.clone(),
                                                            content: sec
                                                                .content
                                                                .clone()
                                                                .unwrap_or_default(),
                                                            section_order: (j + 1) as i64,
                                                        }
                                                    })
                                                    .collect::<Vec<_>>()
                                            })
                                            .unwrap_or_default();
                                        (
                                            crate::db::ChapterRecord {
                                                id: 0,
                                                book_id: 0,
                                                title: ch.title.clone(),
                                                url: ch.url.clone(),
                                                chapter_order: (i + 1) as i64,
                                                word_count: 0,
                                            },
                                            sections,
                                        )
                                    })
                                    .collect();

                                let chapters_count = chapter_records.len() as i64;
                                let db = self.db.lock().await;
                                match db.save_book_with_chapters(
                                    &book_record,
                                    &chapter_records,
                                ) {
                                    Ok(book_id) => {
                                        let _ = db.mark_crawl_task_success(
                                            website_id,
                                            Some(book_id),
                                            chapters_count,
                                        );
                                        total_downloaded += 1;
                                    }
                                    Err(e) => {
                                        error!("保存书籍失败 {}: {}", summary.title, e);
                                        let _ = db.mark_crawl_task_failed(
                                            website_id,
                                            &format!("保存失败: {}", e),
                                        );
                                        total_failed += 1;
                                    }
                                }
                            }
                            Err(e) => {
                                error!("下载失败 {}: {}", summary.title, e);
                                let db = self.db.lock().await;
                                let _ = db.mark_crawl_task_failed(website_id, &format!("{}", e));
                                total_failed += 1;
                            }
                        }
                    }

                    {
                        let mut status = self.status.lock().await;
                        status.books_found = total_found;
                        status.books_downloaded = total_downloaded;
                        status.books_failed = total_failed;
                        status.books_skipped = total_skipped;
                    }

                    self.event_bus.emit(CrawlEvent::Status {
                        running: true,
                        current_page: page as i64,
                        pages_limit: pages_limit as i64,
                        books_found: total_found as i64,
                        books_downloaded: total_downloaded as i64,
                        books_failed: total_failed as i64,
                        books_skipped: total_skipped as i64,
                        message: format!("正在爬取第 {} 页", page),
                    });

                    // 进度日志
                    let progress = (page as f32 / pages_limit as f32 * 100.0) as u32;
                    info!(
                        "[进度 {}/{}页 {}%] 发现 {} 本, 成功 {} 本, 失败 {} 本",
                        page, pages_limit, progress,
                        total_found, total_downloaded, total_failed
                    );
                }
                Err(e) => {
                    error!("获取列表页 {} 失败: {}", page, e);
                    {
                        let mut status = self.status.lock().await;
                        status.message = format!("列表页 {} 失败: {}", page, e);
                    }
                    let msg_clone = format!("列表页 {} 失败: {}", page, e);
                    self.event_bus.emit(CrawlEvent::Status {
                        running: true,
                        current_page: page as i64,
                        pages_limit: pages_limit as i64,
                        books_found: total_found as i64,
                        books_downloaded: total_downloaded as i64,
                        books_failed: total_failed as i64,
                        books_skipped: total_skipped as i64,
                        message: msg_clone,
                    });
                }
            }
        }

        {
            let mut status = self.status.lock().await;
            status.running = false;
            status.last_run = chrono::Local::now()
                .format("%Y-%m-%d %H:%M:%S")
                .to_string();
            status.message = format!(
                "完成: 发现 {} 本, 成功 {} 本, 失败 {} 本",
                total_found, total_downloaded, total_failed
            );
        }

        let final_message = format!(
            "完成: 发现 {} 本, 成功 {} 本, 失败 {} 本",
            total_found, total_downloaded, total_failed
        );
        self.event_bus.emit(CrawlEvent::Status {
            running: false,
            current_page: pages_limit as i64,
            pages_limit: pages_limit as i64,
            books_found: total_found as i64,
            books_downloaded: total_downloaded as i64,
            books_failed: total_failed as i64,
            books_skipped: total_skipped as i64,
            message: final_message,
        });

        info!(
            "[爬取完成] 共 {} 页, 发现 {} 本, 成功 {} 本, 失败 {} 本",
            pages_limit.min(total_found.max(1)),
            total_found,
            total_downloaded,
            total_failed
        );

        Ok(())
    }

    /// 手动下载单本书（按网站 book_id），并写入爬取日志
    pub async fn crawl_book(&self, website_book_id: u32) -> Result<()> {
        self.crawl_book_with_trigger(website_book_id, "manual").await
    }

    /// 重新爬取指定书籍（trigger=retry）
    pub async fn retry_book(&self, website_book_id: u32) -> Result<()> {
        self.crawl_book_with_trigger(website_book_id, "retry").await
    }

    async fn crawl_book_with_trigger(&self, website_book_id: u32, trigger: &str) -> Result<()> {
        let db = self.db.lock().await;
        let exists = db
            .book_exists_by_website_id(website_book_id as i64)
            .unwrap_or(false);
        let existing_chapters = if exists {
            db.get_chapters_count_by_website_id(website_book_id as i64)
                .unwrap_or(0)
        } else {
            0
        };
        let _ = db.upsert_crawl_task_pending(
            website_book_id as i64,
            "",
            trigger,
        );
        drop(db);

        let log_msg = format!("{}爬取开始: book_id={}", trigger, website_book_id);
        let log_id = {
            let db = self.db.lock().await;
            db.insert_crawl_log("INFO", &log_msg).unwrap_or(0)
        };
        self.event_bus.emit(CrawlEvent::Log {
            id: log_id,
            level: "INFO".to_string(),
            message: log_msg,
            timestamp: chrono::Utc::now().timestamp(),
        });

        let task = self.spider.create_download_task(website_book_id);

        {
            let db = self.db.lock().await;
            let _ = db.mark_crawl_task_running(website_book_id as i64);
        }

        // emit TaskUpdate（task 此时为 pending → running）
        if let Ok(Some(task)) = self.db.lock().await.get_crawl_task(website_book_id as i64) {
            if let Ok(task_val) = serde_json::to_value(&task) {
                self.event_bus.emit(CrawlEvent::TaskUpdate { task: task_val });
            }
        }

        let result = if exists && existing_chapters > 0 {
            info!(
                "{}增量更新: book_id={} (已有{}章)",
                trigger, website_book_id, existing_chapters
            );
            task.download_incremental(existing_chapters).await
        } else {
            info!("{}新书下载: book_id={}", trigger, website_book_id);
            task.download().await
        };

        let (book, chapters) = match result {
            Ok(pair) => pair,
            Err(e) => {
                let db = self.db.lock().await;
                let _ = db.mark_crawl_task_failed(website_book_id as i64, &format!("{}", e));
                let log_msg = format!("{}爬取失败 book_id={}: {}", trigger, website_book_id, e);
                let log_id = db.insert_crawl_log("ERROR", &log_msg).unwrap_or(0);
                self.event_bus.emit(CrawlEvent::Log {
                    id: log_id,
                    level: "ERROR".to_string(),
                    message: log_msg,
                    timestamp: chrono::Utc::now().timestamp(),
                });
                // emit TaskUpdate (failed)
                if let Ok(Some(task)) = db.get_crawl_task(website_book_id as i64) {
                    if let Ok(task_val) = serde_json::to_value(&task) {
                        self.event_bus.emit(CrawlEvent::TaskUpdate { task: task_val });
                    }
                }
                return Err(e);
            }
        };

        // 补充任务 title（爬取前不知道）
        {
            let db = self.db.lock().await;
            let _ = db.upsert_crawl_task_pending(website_book_id as i64, &book.title, trigger);
            let _ = db.mark_crawl_task_running(website_book_id as i64);
        }

        let book_record = crate::db::BookRecord {
            id: 0,
            website_book_id: Some(website_book_id as i64),
            path_num: book.num as i64,
            title: book.title.clone(),
            filename: book.filename.clone(),
            author: book.author.clone(),
            category: book.category.clone(),
            introduce: book.introduce.clone(),
            likes: book.likes as i64,
            word_count: book.count as i64,
            page_count: book.page as i64,
            created_at: chrono::Utc::now().timestamp(),
            updated_at: chrono::Utc::now().timestamp(),
        };

        let chapter_records: Vec<_> = chapters
            .iter()
            .enumerate()
            .map(|(i, ch)| {
                let sections = ch
                    .sections
                    .as_ref()
                    .map(|s| {
                        s.iter()
                            .enumerate()
                            .map(|(j, sec)| crate::db::SectionRecord {
                                id: 0,
                                chapter_id: 0,
                                book_id: 0,
                                url: sec.url.clone(),
                                content: sec.content.clone().unwrap_or_default(),
                                section_order: (j + 1) as i64,
                            })
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                (
                    crate::db::ChapterRecord {
                        id: 0,
                        book_id: 0,
                        title: ch.title.clone(),
                        url: ch.url.clone(),
                        chapter_order: (i + 1) as i64,
                        word_count: 0,
                    },
                    sections,
                )
            })
            .collect();

        let chapters_count = chapter_records.len() as i64;
        let db = self.db.lock().await;
        match db.save_book_with_chapters(&book_record, &chapter_records) {
            Ok(book_id) => {
                let _ = db.mark_crawl_task_success(
                    website_book_id as i64,
                    Some(book_id),
                    chapters_count,
                );
                // emit TaskUpdate (success)
                if let Ok(Some(task)) = db.get_crawl_task(website_book_id as i64) {
                    if let Ok(task_val) = serde_json::to_value(&task) {
                        self.event_bus.emit(CrawlEvent::TaskUpdate { task: task_val });
                    }
                }
                let log_msg = format!(
                    "{}爬取成功: {} (book_id={}, {}章)",
                    trigger, book.title, website_book_id, chapters_count
                );
                let log_id = db.insert_crawl_log("INFO", &log_msg).unwrap_or(0);
                self.event_bus.emit(CrawlEvent::Log {
                    id: log_id,
                    level: "INFO".to_string(),
                    message: log_msg,
                    timestamp: chrono::Utc::now().timestamp(),
                });
                info!("{}爬取成功: {} (book_id={})", trigger, book.title, website_book_id);
            }
            Err(e) => {
                let _ = db.mark_crawl_task_failed(
                    website_book_id as i64,
                    &format!("保存失败: {}", e),
                );
                // emit TaskUpdate (failed)
                if let Ok(Some(task)) = db.get_crawl_task(website_book_id as i64) {
                    if let Ok(task_val) = serde_json::to_value(&task) {
                        self.event_bus.emit(CrawlEvent::TaskUpdate { task: task_val });
                    }
                }
                let log_msg = format!("{}爬取保存失败 {}: {}", trigger, book.title, e);
                let log_id = db.insert_crawl_log("ERROR", &log_msg).unwrap_or(0);
                self.event_bus.emit(CrawlEvent::Log {
                    id: log_id,
                    level: "ERROR".to_string(),
                    message: log_msg,
                    timestamp: chrono::Utc::now().timestamp(),
                });
                return Err(e);
            }
        }

        Ok(())
    }
}
