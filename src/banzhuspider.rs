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
use std::collections::HashMap;
use std::fmt::Display;
use std::future::Future;
use std::hash::Hash;
use std::ops::Deref;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::io::AsyncWriteExt;
use tokio::sync::{Mutex, Semaphore};
use tokio::time::sleep;
use log::{error, info};

/// Constants for anti-crawling dictionaries
const IMAGE_FANPA_FILE: &str = include_str!("../asset/txt/变形字体库v2.txt");
const FONT_FANPA_FILE: &str = include_str!("../asset/txt/字体反爬库.txt");

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

impl BanzhuSpider {
    /// Create a new spider instance with default configuration
    pub fn new() -> Self {
        let config = Arc::new(Config::builder()
            .add_source(File::with_name("spider.toml"))
            .build()
            .expect("Failed to build spider config"));

        let url = config
            .get_string("root_url")
            .expect("Failed to get root url from config");
       
        let client = Arc::new(Client::builder()
            .cookie_store(true)
            .zstd(true)
            .user_agent(DEFAULT_USER_AGENT)
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap());

        let img_fanpa_dict = Arc::new(init_img_fanpa_dict());
        let font_fanpa_dict = Arc::new(init_font_fanpa_dict());

        Self {
            img_fanpa_dict,
            font_fanpa_dict,
            url,
            config,
            client,
            spider_config: Arc::new(SpiderConfig::default()),
        }
    }

    /// Configure spider settings
    pub fn with_config(mut self, config: SpiderConfig) -> Self {
        self.spider_config = Arc::new(config);
        self
    }

    /// Run the spider with concurrent task processing
    pub async fn run(&mut self) -> Result<(), Error> {
        info!("Starting spider with max concurrent tasks: {}", self.spider_config.max_concurrent_tasks);
        
        let max_num = self.config.get_int("max_num").unwrap_or(1000);
        let start = self.config.get_int("start").unwrap_or(1);
        
        let cf = Arc::new(Mutex::new(CloudflareBypass::new(self.url.clone())));

        cf.lock().await.bypass_cloudflare().await?;

        let multi_pbr = create_multi_pbr();

        // Semaphore for controlling concurrent tasks
        let semaphore = Arc::new(Semaphore::new(self.spider_config.max_concurrent_tasks));
        let mut handles = vec![];

        for book_id in start..max_num {
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
                        let result = task.download().await;
                        match result {
                            Ok(_) => info!("Successfully downloaded book {}", book_id),
                            Err(e) => error!("Failed to download book {}: {}", book_id, e),
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

        info!("Spider completed successfully");
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
        let spider = BanzhuSpider::new()
            .with_config(SpiderConfig {
                max_concurrent_tasks: 8,
                retry_attempts: 5,
                retry_delay: Duration::from_secs(10),
                request_timeout: Duration::from_secs(60),
            });
        
        assert_eq!(spider.spider_config.max_concurrent_tasks, 8);
        assert_eq!(spider.spider_config.retry_attempts, 5);
    }
}
