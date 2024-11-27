use crate::bypass::CloudflareBypass;
use crate::task::BanzhuDownloadTask;
use crate::{Error, DEFAULT_USER_AGENT};
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
use std::time::{Duration, SystemTime};
use futures::future::join_all;
use futures::SinkExt;
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;

const IMAGE_FANPA_FILE: &str = include_str!("../asset/txt/变形字体库v2.txt");
const FONT_FANPA_FILE: &str = include_str!("../asset/txt/字体反爬库.txt");





pub struct BanzhuSpider {
    url: String,
    config: Arc<Config>,
    pub client: Arc<Client>,
    pub img_fanpa_dict: Arc<HashMap<String, String>>,
    pub font_fanpa_dict: Arc<HashMap<String, String>>,
}

pub fn init_img_fanpa_dict() -> HashMap<String, String> {
    let mut img_fanpa_dict = HashMap::new();

    // 初始化反爬字典
    for line in IMAGE_FANPA_FILE.split("\n") {
        let arr = line.split(" ").collect::<Vec<&str>>();
        if arr.len() == 2 {
            let word = arr[0].trim().to_string();
            let url = arr[1].trim().to_string();
            img_fanpa_dict.insert(url, word);
        }
    }

    return img_fanpa_dict;
}

pub fn init_font_fanpa_dict() -> HashMap<String, String> {
    let mut dict = HashMap::new();

    // 初始化反爬字典
    for line in FONT_FANPA_FILE.split("\n") {
        let arr = line.split("\t").collect::<Vec<&str>>();
        if arr.len() == 2 {
            let key = arr[0].trim().to_string();
            let val = arr[1].trim().to_string();
            dict.insert(key, val);
        }
    }
    return dict;
}

impl BanzhuSpider {
    pub fn new() -> BanzhuSpider {
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
            .build().unwrap());

        let  img_fanpa_dict = Arc::new(init_img_fanpa_dict());
        let  font_fanpa_dict = Arc::new(init_font_fanpa_dict());

        // println!("img_fanpa_dict: {:#?}", img_fanpa_dict);
        // println!("font_fanpa_dict: {:#?}", font_fanpa_dict);
        BanzhuSpider {
            img_fanpa_dict,
            font_fanpa_dict,
            url,
            config,
            client,
        }
    }

    

    pub async fn run(&mut self) -> Result<(), Error> {
  
        
        let max_num = self.config.get_int("max_num").unwrap_or(1000);
        let root_url = self
            .config
            .get_string("root_url")
            .expect("Failed to get root url from config");

        let start = self
            .config
            .get_int("start")
            .unwrap_or(1);
        
        let cf = Arc::new(Mutex::new(CloudflareBypass::new(root_url.clone())));
        
        // 读取本地记录
        cf.lock().await.read_ua_cookie().await;
        
        // let mut futures = vec![];
        for book_id in start..max_num {
            let cf = cf.clone();
            let task = BanzhuDownloadTask::new(root_url.clone(),
                                               book_id, self.config.clone(), 
                                               self.img_fanpa_dict.clone(), 
                                               self.font_fanpa_dict.clone(),
                                               self.client.clone(), cf);
            let future = async move {
                match task.download().await {
                    Ok(_) => {
                        println!("Download successful:{}", book_id);
                    }
                    Err(_) => {

                    }
                };
            };
            future.await;
            // futures.push(future);
        }
        
        // join_all(futures).await;
        // join_num(futures, 16).await;
        Ok(())
    }
}

pub async fn join_num(futures: Vec<impl Future<Output = ()> + Sized>, step: usize) {
    
    let start = 0;
    let end =  {
        if futures.len() <= step {
            futures.len()
        } else {
            step
        }
    };
    if start >= end {
        return;
    }
    // let futures_slice = futures[start..end].to_vec();
    
    // join_all(futures_slice).await;
    
}


pub fn time() -> u128 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis()
}

#[cfg(test)]
mod tests {
    use pbr::ProgressBar;
    use std::thread;
    use std::time::Duration;


    #[test]
    fn test_pbr() {
        let count = 1000;
        let mut pb = ProgressBar::new(count);
        pb.format("╢▌▌░╟");
        for _ in 0..count {
            pb.inc();
            thread::sleep_ms(200);
        }
        pb.finish_print("done");
    }

    #[test]
    fn test_unicode() {
        let string1 = '\u{a0}';
        let mut escaped_string = String::new();
        let hex_str = format!("{:x}", string1 as u32);
        let escaped_char = format!("\\u{}", hex_str);
        println!("字符串 '{}' 的\\u格式Unicode转义序列为: {}", string1, escaped_char);
    }

    #[test]
    fn test_rq() {
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            let client = reqwest::Client::new();
            let response = client
                .get("https://www.44yydstxt234.com/1/1/")
                .timeout(Duration::from_secs(5))
                .send()
                .await
                .unwrap();
            assert!(response.status().is_success());
        });
    }
}
