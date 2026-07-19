use crate::banzhuspider::BanzhuSpider;
use crate::db::Database;
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
    pub books_found: u32,
    pub books_downloaded: u32,
    pub last_run: String,
}

impl Default for CrawlStatus {
    fn default() -> Self {
        Self {
            running: false,
            current_page: 0,
            books_found: 0,
            books_downloaded: 0,
            last_run: String::new(),
        }
    }
}

pub struct Scheduler {
    pub status: Arc<Mutex<CrawlStatus>>,
    spider: Arc<BanzhuSpider>,
    db: Arc<Mutex<Database>>,
    pub config: Arc<Config>,
}

impl Scheduler {
    pub fn new(
        spider: Arc<BanzhuSpider>,
        db: Arc<Mutex<Database>>,
        config: Arc<Config>,
    ) -> Self {
        Self {
            status: Arc::new(Mutex::new(CrawlStatus::default())),
            spider,
            db,
            config,
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
            status.books_found = 0;
            status.books_downloaded = 0;
        }

        info!("Starting incremental crawl (max {} pages)...", pages_limit);

        let mut skipped_streak = 0;
        let mut total_found = 0u32;
        let mut total_downloaded = 0u32;

        for page in 1..=pages_limit {
            {
                let mut status = self.status.lock().await;
                status.current_page = page;
            }

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
                            drop(db);

                            let task = self.spider.create_download_task(summary.book_id);

                            let result = if exists && existing_chapters > 0 {
                                info!("增量更新: {} (id={}, 已有{}章)", summary.title, summary.book_id, existing_chapters);
                                task.download_incremental(existing_chapters).await
                            } else {
                                info!("新书下载: {} (id={})", summary.title, summary.book_id);
                                task.download().await
                            };

                            (summary, result)
                        })
                        .buffer_unordered(concurrency)
                        .collect()
                        .await;

                    for (summary, result) in results {
                        match result {
                            Ok((book, chapters)) => {
                                let book_record = crate::db::BookRecord {
                                    id: 0,
                                    website_book_id: Some(summary.book_id as i64),
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

                                let db = self.db.lock().await;
                                if let Err(e) = db.save_book_with_chapters(
                                    &book_record,
                                    &chapter_records,
                                ) {
                                    error!("保存书籍失败 {}: {}", summary.title, e);
                                } else {
                                    total_downloaded += 1;
                                }
                            }
                            Err(e) => {
                                error!("下载失败 {}: {}", summary.title, e);
                            }
                        }
                    }

                    {
                        let mut status = self.status.lock().await;
                        status.books_found = total_found;
                        status.books_downloaded = total_downloaded;
                    }

                    // 进度日志
                    let progress = (page as f32 / pages_limit as f32 * 100.0) as u32;
                    info!(
                        "[进度 {}/{}页 {}%] 发现 {} 本, 成功 {} 本, 失败 {} 本",
                        page, pages_limit, progress,
                        total_found, total_downloaded,
                        total_found - total_downloaded
                    );
                }
                Err(e) => {
                    error!("获取列表页 {} 失败: {}", page, e);
                }
            }
        }

        {
            let mut status = self.status.lock().await;
            status.running = false;
            status.last_run = chrono::Local::now()
                .format("%Y-%m-%d %H:%M:%S")
                .to_string();
        }

        info!(
            "[爬取完成] 共 {} 页, 发现 {} 本, 成功 {} 本, 失败 {} 本",
            pages_limit.min(total_found.max(1)),
            total_found,
            total_downloaded,
            total_found.saturating_sub(total_downloaded)
        );

        Ok(())
    }
}
