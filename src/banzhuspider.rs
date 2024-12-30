use crate::bypass::CloudflareBypass;
use crate::task::BanzhuDownloadTask;
use crate::{create_multi_pbr, create_pbr, Error, DEFAULT_USER_AGENT};
use aes::cipher;
use aes::cipher::{ArrayLength, BlockDecrypt, BlockDecryptMut, BlockEncryptMut, KeyInit};
use base64::Engine;
use cipher::KeyIvInit;
use config::{Config, File};
use encoding::Encoding;
use pyo3::unindent::Unindent;
use rand::Rng;
use reqwest::Client;
use std::any::Any;
use std::collections::{HashMap, HashSet};
use std::fmt::Display;
use std::fs::OpenOptions;
use std::future::Future;
use std::hash::Hash;
use std::io::{BufRead, BufReader, Write};
use std::ops::Deref;
use std::{fs, process};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use cipher::typenum::private::Trim;
use itertools::Itertools;
use tokio::io::AsyncWriteExt;
use tokio::sync::{broadcast, mpsc, Mutex, RwLock, Semaphore};
use tokio::time::sleep;
use log::{debug, error, info};
use lazy_static::lazy_static;

/// Constants for anti-crawling dictionaries
const IMAGE_FANPA_FILE: &str = include_str!("../asset/txt/变形字体库v2.txt");
const FONT_FANPA_FILE: &str = include_str!("../asset/txt/字体反爬库.txt");

lazy_static! {
    static ref DOWNLOAD_BOOK_IDS: Arc<RwLock<HashSet<u32>>> = Arc::new(RwLock::new(HashSet::new()));
}
/// Spider configuration
#[derive(Debug)]
pub struct SpiderConfig {
    pub max_concurrent_tasks: usize,
    pub retry_attempts: u32,
    pub retry_delay: Duration,
    pub request_timeout: Duration,
}

impl Default for SpiderConfig {
    fn default() -> Self {
        Self {
            max_concurrent_tasks: 16,
            retry_attempts: 3,
            retry_delay: Duration::from_millis(100),
            request_timeout: Duration::from_secs(5),
        }
    }
}

/// Main spider implementation for web scraping
pub struct BanzhuSpider {
    url: String,
    config: Arc<Config>,
    spider_config: Arc<SpiderConfig>,
    pub client: Arc<Client>,
    pub img_fanpa_dict: Arc<HashMap<String, String>>,
    pub font_fanpa_dict: Arc<HashMap<String, String>>,
    pub cf: Arc<Mutex<CloudflareBypass>>,
}

/// Initialize image anti-crawling dictionary
pub fn init_img_fanpa_dict() -> HashMap<String, String> {
    let mut img_fanpa_dict = HashMap::new();
    for line in IMAGE_FANPA_FILE.split('\n') {
        if let Some((word, url)) = line.split_once(' ') {
            img_fanpa_dict.insert(url.trim().to_string(), word.trim().to_string());
        }
    }
    img_fanpa_dict
}

/// Initialize font anti-crawling dictionary
pub fn init_font_fanpa_dict() -> HashMap<String, String> {
    let mut dict = HashMap::new();
    for line in FONT_FANPA_FILE.split('\n') {
        if let Some((key, val)) = line.split_once('\t') {
            dict.insert(key.trim().to_string(), val.trim().to_string());
        }
    }
    dict
}

pub async fn find_max_id() -> Option<u32> {
    let guard = DOWNLOAD_BOOK_IDS.read().await;

    guard.iter().max().cloned()
}

pub async fn init_download_book_ids() {
    let mut guard = DOWNLOAD_BOOK_IDS.write().await;
    let content = fs::read_to_string("download_ids.txt").unwrap();
    for line in content.lines() {
        let line = line.trim();
        if !line.is_empty() {
            guard.insert(line.parse().unwrap());
        }
    }
}

pub async fn save_download_ids() {
    let guard = DOWNLOAD_BOOK_IDS.read().await;
    let mut file = OpenOptions::new()
        .truncate(true)
        .write(true)
        .create(true)
        .open("download_ids.txt")
        .unwrap();
    let result: Vec<_> = guard.iter().into_iter().sorted().collect();
    let mut content = String::new();
    for i in result {
        content.push_str(&format!("{}\n", i));
    }
    file.write_all(format!("{}\n", content).as_bytes()).unwrap();
}

pub async fn add_download_book_id(book_id: u32) {
    let mut guard = DOWNLOAD_BOOK_IDS.write().await;
    guard.insert(book_id);
}

impl BanzhuSpider {
    /// Create a new spider instance with default configuration
    pub fn new(url: String, config: Arc<Config>) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent(DEFAULT_USER_AGENT)
            .build()
            .unwrap();

        let img_fanpa_dict = init_img_fanpa_dict();
        let font_fanpa_dict = init_font_fanpa_dict();

        BanzhuSpider {
            url: url.clone(),
            config,
            client: Arc::new(client),
            img_fanpa_dict: Arc::new(img_fanpa_dict),
            font_fanpa_dict: Arc::new(font_fanpa_dict),
            spider_config: Arc::new(SpiderConfig::default()),
            cf: Arc::new(Mutex::new(CloudflareBypass::new(url))),
        }
    }

    /// Configure spider settings
    pub fn with_config(mut self, config: SpiderConfig) -> Self {
        self.spider_config = Arc::new(config);
        self
    }

    /// Run the spider with concurrent task processing
    pub async fn run(&mut self) -> Result<(), Error> {
        debug!("Starting spider with max concurrent tasks: {}", self.spider_config.max_concurrent_tasks);
        // 初始化download_ids
        init_download_book_ids().await;

        let max_num: u32 = self.config.get_int("max_num").unwrap_or(1000) as u32;   
        let default_start: u32 = self.config.get_int("start").unwrap_or(1) as u32;
        let start = find_max_id().await.unwrap_or(default_start);
        let cf = self.cf.clone();

        cf.lock().await.bypass_cloudflare().await?;

        let multi_pbr = create_multi_pbr();

        // Semaphore for controlling concurrent tasks
        let semaphore = Arc::new(Semaphore::new(self.spider_config.max_concurrent_tasks));
        let mut handles = vec![];

        // 优雅停机处理
        let running = Arc::new(AtomicBool::new(true));
        let running_clone = Arc::clone(&running);
        let (tx, rx) = broadcast::channel::<()>(self.spider_config.max_concurrent_tasks);
        // 信号处理程序
        ctrlc::set_handler(move || {
            if !running_clone.load(Ordering::SeqCst) {
                return;
            }
            error!("Received Ctrl+C, shutting down gracefully...");
            running_clone.store(false, Ordering::SeqCst);
        }).expect("Error setting Ctrl+C handler");

        for book_id in start..max_num {
            if !running.load(Ordering::SeqCst) {
                drop(tx);
                break;
            }

            if DOWNLOAD_BOOK_IDS.read().await.contains(&book_id) {
                continue;
            }
            
            let mut rx_clone = tx.subscribe();
            let permit = semaphore.clone().acquire_owned().await.unwrap();
            let cf = cf.clone();
            let m_clone_pbr = multi_pbr.clone();
            let spider_config = self.spider_config.clone();
            
            let task = BanzhuDownloadTask::new(
                self.url.clone(),
                book_id,
                self.config.clone(),
                self.img_fanpa_dict.clone(),
                self.font_fanpa_dict.clone(),
                self.client.clone(),
                cf,
                m_clone_pbr,
                spider_config
            );

            let handle = tokio::task::spawn_blocking(move || {
                tokio::runtime::Runtime::new()
                    .unwrap()
                    .block_on(async {
                        tokio::select! {
                            _ = rx_clone.recv() => {}
                            result = task.download() => {
                                match result {
                                    Ok(_) => debug!("Successfully downloaded book {}", book_id),
                                    Err(e) => debug!("Failed to download book {}: {}", book_id, e),
                                }
                            }
                        }
                        drop(permit);
                    });
            });
            handles.push(handle);

            // Optional delay between tasks
            sleep(Duration::from_millis(100)).await;
        }
        
        // Wait for all tasks to complete
        for handle in handles {
            if let Err(e) = handle.await {
                error!("Task join error: {}", e);
            }
        }

        save_download_ids().await;

        debug!("Spider completed successfully");
        Ok(())
    }
}

pub fn time() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_spider_config() {
        let spider = BanzhuSpider::new("https://example.com".to_string(), Arc::new(Config::builder()
            .add_source(File::with_name("spider.toml"))
            .build()
            .expect("Failed to build spider config")));
        
        assert_eq!(spider.spider_config.max_concurrent_tasks, 16);
        assert_eq!(spider.spider_config.retry_attempts, 3);
    }
}
