use banzhu_spider::banzhuspider::BanzhuSpider;
use banzhu_spider::Error;
use config::{Config, File};
use humantime::format_duration;
use std::fs;
use std::path::Path;
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

    let save_path = config
        .get_string("save_path")
        .expect("Failed to get save path from config"); 

    let total_size = compute_dir_size(&save_path);

    let url = config
        .get_string("root_url")
        .expect("Failed to get root url from config");
    
    let mut spider = BanzhuSpider::new(url, config);
    
    spider.run().await?;
    
    print!("\x1B[2J\x1B[1;1H");

    let elapsed_total_size = compute_dir_size(&save_path);

    println!("Done. Total cost: {}, speed: {}/s", format_duration(now.elapsed()?), format_bytes((elapsed_total_size - total_size) as f64/ now.elapsed().unwrap().as_secs() as f64));
    Ok(())
}

fn compute_dir_size(path: &str) -> u64 {
    let path = Path::new(path);
    let mut size = 0;
    let dir = fs::read_dir(path);
    if let Ok(dir) = dir {
        for entry in dir {
            let entry = entry.unwrap();
            let metadata = entry.metadata().unwrap();
            if metadata.is_file() {
                size += metadata.len();
            } else {
                size += compute_dir_size(entry.path().to_str().unwrap());
            }
        }
    }
    size
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

fn format_bytes(size_in_bytes: f64) -> String {
    if size_in_bytes < 1024.0 {
        format!("{} B", size_in_bytes)
    } else if size_in_bytes < 1024.0 * 1024.0 {
        format!("{:.2} KB", size_in_bytes as f64 / 1024.0)
    } else if size_in_bytes < 1024.0 * 1024.0 * 1024.0 {
        format!("{:.2} MB", size_in_bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB", size_in_bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}


#[cfg(test)]
mod tests {
    use crate::{compute_dir_size, format_bytes};


    #[test]
    fn test_compute_files_size() {
        let size = compute_dir_size("target");
        assert_eq!(format_bytes(size as f64), "59153.00 MB");
    }
}