use crate::banzhuspider::time;
use crate::DEFAULT_USER_AGENT;
use futures::lock::Mutex;
use lazy_static::lazy_static;
use log::{info, warn};
use mouse_rs::types::keys::Keys;
use mouse_rs::Mouse;
use opencv::core::{min_max_loc, Point};
use opencv::imgcodecs::{imdecode, imread, IMREAD_COLOR};
use opencv::imgproc::{match_template_def, TM_CCOEFF_NORMED};
use opencv::prelude::*;
use pyo3::ffi::c_str;
use pyo3::prelude::PyAnyMethods;
use pyo3::types::PyModule;
use pyo3::Python;
use rand::Rng;
use regex::Regex;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, ACCEPT_ENCODING, COOKIE, USER_AGENT};
use serde_json::Value;
use std::collections::HashMap;
use std::ops::DerefMut;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use std::{fs, thread};
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;

lazy_static! {
static ref LANGUAGE_DICTS: HashMap<&'static str, &'static str> =  {
        let mut m = HashMap::new();
        m.insert("en-us", "Just a moment");
        m.insert("zh-cn","请稍候");
        m
    };

static ref BYPASS_REGEX: Regex = Regex::new(r"<title>(?P<title>.*?)</title>").unwrap();

}
lazy_static! {
    static ref IMAGE_PATH_DICT: HashMap<&'static str, &'static str> = {
        let mut m = HashMap::new();
        m.insert("zh-cn","asset/img/zh-cn.jpg");
        m
    };
}


pub fn is_bypassed(html: &str) -> bool {
    if let Some(cap) = BYPASS_REGEX.captures(html) {
        let title = &cap["title"];
        for val in LANGUAGE_DICTS.values() {
            if title.contains(*val) {
                return false;
            }
        }
    }
    true
}

#[derive(Debug)]
pub struct CloudflareBypass {
    pub url: String,
    pub is_click: bool,
    pub img_dict: HashMap<String, Mat>,
    pub last_bypassed: u128,
    pub headers: HashMap<&'static str, String>,
}




impl CloudflareBypass {
    pub fn new(url: String) -> CloudflareBypass {
        let mut img_dict = HashMap::new();
        for key in IMAGE_PATH_DICT.keys() {
            let val = IMAGE_PATH_DICT.get(key).unwrap();
            let image = imread(val, IMREAD_COLOR).unwrap();
            img_dict.insert(key.to_string(), image);
        }
        CloudflareBypass {
            url,
            is_click: false,
            img_dict,
            last_bypassed: 0,
            headers: Default::default(),
        }
    }

    pub async fn get_headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        let headers_map;
        {
            headers_map = self.headers.clone()
        }
        headers.insert(USER_AGENT, HeaderValue::from_str(headers_map.get("User-Agent").unwrap_or(&DEFAULT_USER_AGENT.to_string())).unwrap());
        if let Some(cookie) = headers_map.get("Cookie") {
            headers.insert(COOKIE, HeaderValue::from_str(cookie).unwrap());
        }
        // 压缩请求
        headers.insert(ACCEPT_ENCODING, HeaderValue::from_static("gzip, deflate, zstd"));
        
        return headers;
    }
    
    pub async fn read_ua_cookie(&mut self) {
        let headers = &mut self.headers;
        let path = Path::new("agent.json");
        let content = tokio::fs::read_to_string(&path).await.unwrap();
        if content.len() > 0 {
            let value: Value = serde_json::from_str(&content).unwrap();
            if let Some(cookie) = value.get("Cookie") {
                if let Some(cookie) = cookie.as_str() {
                    headers.insert("Cookie", cookie.to_string());
                }
            }
            if let Some(ua) = value.get("User-Agent") {
                if let Some(ua) = ua.as_str() {
                    headers.insert("User-Agent", ua.to_string());
                }
            }

            if let Some(v) = value.get("last_bypassed") {
                if let Some(v) = v.as_str() {
                    self.last_bypassed = v.parse::<u128>().unwrap_or(0);
                }
            }
        }
    }

    pub async fn bypass_cloudflare(&mut self) -> anyhow::Result<()> {

        let now = time();

        if now - self.last_bypassed > 60 * 1000 {
            info!("\n***************** bypass cloudflare *****************");
            

            let (ua, cookie) = self.bypass().await?;
            info!("User-Agent:{ua}");
            info!("Cookie:{cookie}");
            let mut headers = &mut self.headers;
            
            if cookie.len() != 0 {
                headers.insert("Cookie", cookie);
            }
            if ua.len() != 0 {
                headers.insert("User-Agent", ua);
            }

            if headers.len() > 0 {
                // 记录cookie到本地
                record_ua_cookie(&*headers).await;
            }
            info!("***************** bypass cloudflare *****************\n");

            self.last_bypassed = time();
        } else {
            warn!("the distance from the last bypass cloudflare is no more than 60 seconds, ignore this bypass");
        }

        Ok(())
    }
    
    pub async fn bypass(&self) -> anyhow::Result<(String, String)> {
        let url = self.url.clone();
        let mut rng = rand::thread_rng();
        Python::with_gil(|py| {
            let code = c_str!(include_str!("bypass.py"));
            let bypass = PyModule::from_code(py,code, c_str!("bypass.py"), c_str!("bypass"))?;
            // 打开网页
            bypass.getattr("open_url")?.call1((url,))?;

            // 获取标题
            loop {
                let title_ret = bypass.getattr("get_title")?.call0()?;
                let title: &str = title_ret.extract()?;

                if self.is_bypassed(title) {
                    break;
                }
                //截屏
                let data: Vec<u8> = bypass.getattr("screenshot")?.call0()?.extract()?;

                // fs::write("screenshot.png", &data)?;
                let screenshot = Mat::from_bytes::<u8>(&data).unwrap();
                let screenshot = imdecode(&screenshot, IMREAD_COLOR).unwrap();
                for target in self.img_dict.values() {
                    let coords = image_search(&screenshot, target);
                    let location: Vec<i32> = bypass.getattr("get_page_location")?.call0()?.extract()?;
                    self.click_button(coords.0 + location[0] + 10, coords.1 + location[1] + 10);
                }
                thread::sleep(Duration::from_secs(rng.gen_range(2..3)));
            }
            // 获取信息
            let ua: String = bypass.getattr("get_ua")?.call0()?.extract()?;

            let cookie: String = bypass.getattr("get_cookie")?.call0()?.extract()?;

            // 退出
            bypass.getattr("quit")?.call0()?;
            Ok((ua, cookie))
        })
    }

    // bypass了吗
    pub fn is_bypassed(&self, page_title: &str) -> bool {
        for title in LANGUAGE_DICTS.values() {
            if page_title.contains(*title) {
                return false;
            }
        }
        true
    }

    pub fn click_button(&self, x: i32, y: i32){
        info!("Click cloudflare button for {}-{}", x, y);
        let mouse = Mouse::new();
        mouse.move_to(x, y).expect("Unable to move mouse");
        mouse.click(&Keys::LEFT).expect("Unable to click button");
    }
}



fn image_search(image: &Mat, target: &Mat) -> (i32, i32) {
    let mut result = Mat::default();
    match_template_def(image, target, &mut result, TM_CCOEFF_NORMED).expect("Image search failed");
    let mut min_val = 0.8;
    let mut max_val = 1.0;
    let mut min_loc = Point::new(0, 0);
    let mut max_loc = Point::new(0, 0);
    let mask = Mat::default();
    min_max_loc(&result,Some(&mut min_val), Some(&mut max_val), Some(&mut min_loc) ,Some(&mut max_loc), &mask).unwrap();

    let start_x = max_loc.x;
    let start_y = max_loc.y;
    return (start_x, start_y);
}


async fn record_ua_cookie(headers: &HashMap<&str, String>) {
    let mut map = headers.clone();
    map.insert("last_bypassed", time().to_string());
    let path = Path::new("agent.json");

    let content = serde_json::to_string_pretty(&map).unwrap();

    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(path).await.unwrap();
    file.write(content.as_bytes()).await.unwrap();
}


#[cfg(test)]
mod tests {
    use pyo3::ffi::c_str;
    use pyo3::prelude::*;
    use std::fs;


    #[test]
    fn test_pyo3() -> PyResult<()> {

        Python::with_gil(|py| {
            let url = "https://www.44yydstxt234.com";
            let code = c_str!(include_str!("bypass.py"));
            let bypass = PyModule::from_code(py,code, c_str!("bypass.py"), c_str!("bypass"))?;

            bypass.getattr("open_url")?.call1((url,))?;

            let data: Vec<u8> = bypass.getattr("screenshot")?.call0()?.extract()?;
            fs::write("screenshot.jpg", data)?;
            Ok(())
        })
    }
}
