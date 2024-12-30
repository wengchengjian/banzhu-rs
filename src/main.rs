use banzhu_spider::banzhuspider::BanzhuSpider;
use banzhu_spider::Error;
use config::{Config, File};
use humantime::format_duration;
use std::fs;
use std::sync::Arc;
use std::time::SystemTime;

#[tokio::main]
async fn main() -> Result<(), Error> {
    log_setting();
    
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

fn log_setting() {
    use std::io::Write;
    // 配置日志输出到文件
    let log_file = fs::File::create("spider.log").expect("Failed to create log file");
    env_logger::Builder::new()
        .format(|buf, record| {
            writeln!(buf,
                     "{} [{}] - {}",
                     buf.timestamp(),
                     record.level(),
                     record.args()
            )
        })
        .target(env_logger::Target::Pipe(Box::new(log_file)))
        .init();
}
