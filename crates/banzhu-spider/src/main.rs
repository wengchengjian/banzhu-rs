use banzhu_spider::web;
use banzhu_spider::Error;
use std::fs;

#[tokio::main]
async fn main() -> Result<(), Error> {
    log_setting();

    log::info!("banzhu-spider v{} 启动", env!("CARGO_PKG_VERSION"));

    if let Err(e) = web::run_web().await {
        log::error!("致命错误: {}", e);
        eprintln!("致命错误: {}", e);
        std::process::exit(1);
    }

    log::info!("banzhu-spider 已安全退出");
    Ok(())
}

fn log_setting() {
    use std::io::Write;

    let log_file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("spider.log")
        .expect("Failed to open log file");

    env_logger::Builder::new()
        .filter_level(log::LevelFilter::Info)
        .format(move |buf, record| {
            let ts = buf.timestamp_millis();
            let level = record.level();
            let module = record.module_path().unwrap_or("?");
            // 简化模块路径：banzhu_spider::cf::mod → cf
            let short_module = module
                .strip_prefix("banzhu_spider::")
                .unwrap_or(module)
                .split("::")
                .next()
                .unwrap_or(module);

            writeln!(
                buf,
                "{} {:<5} [{}] {}",
                ts,
                level,
                short_module,
                record.args()
            )
        })
        .target(env_logger::Target::Pipe(Box::new(log_file)))
        .init();
}
