use banzhu_spider::web;
use banzhu_spider::web::InitialCrawl;
use banzhu_spider::Error;
use std::fs;
use std::io::{self, Write};
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Error> {
    log_setting();

    log::info!("banzhu-spider v{} 启动", env!("CARGO_PKG_VERSION"));

    // 命令行参数：
    //   --full       启动时立即执行一次全量爬取
    //   --crawl      启动时立即执行一次增量爬取
    // 默认不执行初始爬取（仅启动 Web 服务，等待手动/定时触发）
    let mut initial = InitialCrawl::None;
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--full" | "--full-crawl" => initial = InitialCrawl::Full,
            "--crawl" => initial = InitialCrawl::Incremental,
            "--no-crawl" => initial = InitialCrawl::None,
            other => {
                log::warn!("未知参数: {other}（支持 --full / --no-crawl）");
            }
        }
    }
    if matches!(initial, InitialCrawl::Full) {
        log::info!("命令行参数 --full：启动后执行全量爬取");
    }

    if let Err(e) = web::run_web(initial).await {
        log::error!("致命错误: {}", e);
        eprintln!("致命错误: {}", e);
        std::process::exit(1);
    }

    log::info!("banzhu-spider 已安全退出");
    Ok(())
}

// ─── 日志系统：控制台 + 文件双输出，大小轮转 ─────────────────────────────────

const LOG_FILE: &str = "spider.log";
const LOG_MAX_SIZE: u64 = 5 * 1024 * 1024; // 5MB 轮转

/// 同时写入 stdout 和日志文件的 Writer，支持大小轮转
struct TeeLogger {
    file: fs::File,
    file_size: u64,
}

impl TeeLogger {
    fn new() -> Self {
        let file_size = Path::new(LOG_FILE)
            .metadata()
            .map(|m| m.len())
            .unwrap_or(0);
        let file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(LOG_FILE)
            .expect("Failed to open log file");
        Self { file, file_size }
    }

    /// 大小轮转：spider.log → spider.log.1（覆盖旧的 .1）
    fn rotate(&mut self) {
        let _ = fs::remove_file(format!("{}.1", LOG_FILE));
        let _ = fs::rename(LOG_FILE, format!("{}.1", LOG_FILE));
        self.file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(LOG_FILE)
            .expect("Failed to reopen log file after rotation");
        self.file_size = 0;
        // 注意：不能在此调用 log::info!。write() 在 env_logger 的 writer 锁内执行，
        // 轮转后再次发日志会重复加锁（std Mutex 不可重入）导致死锁、进程卡死。
        let _ = writeln!(self.file, "日志已轮转 (旧日志: {}.1)", LOG_FILE);
    }
}

impl Write for TeeLogger {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        // 写入 stdout
        let stdout = io::stdout();
        let mut handle = stdout.lock();
        let _ = handle.write_all(buf);

        // 写入文件
        self.file.write_all(buf)?;
        self.file_size += buf.len() as u64;

        // 检查是否需要轮转
        if self.file_size >= LOG_MAX_SIZE {
            self.rotate();
        }

        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        let stdout = io::stdout();
        let mut handle = stdout.lock();
        let _ = handle.flush();
        self.file.flush()
    }
}

fn log_setting() {
    let writer = TeeLogger::new();

    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info"),
    )
    .format(|buf, record| {
            let ts = buf.timestamp_millis();
            let level = record.level();
            let module = record.module_path().unwrap_or("?");
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
        .target(env_logger::Target::Pipe(Box::new(writer)))
        .init();
}
