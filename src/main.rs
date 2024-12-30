use std::sync::Arc;
use std::time::{Duration, SystemTime};
use config::{Config, File};
use humantime::format_duration;
use banzhu_spider::banzhuspider::BanzhuSpider;
use banzhu_spider::Error;

#[tokio::main]
async fn main() -> Result<(), Error> {
    env_logger::init();
    let now = SystemTime::now();
 
    // 获取配置
    let config = Arc::new(Config::builder()
        .add_source(File::with_name("spider.toml"))
        .build()
        .expect("Failed to build spider config"));

    let url = config
        .get_string("root_url")
        .expect("Failed to get root url from config");
    
    let mut spider = BanzhuSpider::new(url, config);
    
    spider.run().await?;

    print!("\x1B[2J\x1B[1;1H");
 
    println!("Done. Total cost: `{}", format_duration(now.elapsed()?));
    Ok(())
}
