use banzhu_spider::cli::{Cli, Commands};
use banzhu_spider::Error;
use clap::Parser;
use std::fs;

#[tokio::main]
async fn main() -> Result<(), Error> {
    log_setting();

    let cli = Cli::parse();

    if let Err(e) = match cli.command {
        Commands::Download { id_range } => banzhu_spider::cli::run_download(&id_range).await,
        Commands::Export { id, format, output } => {
            banzhu_spider::cli::run_export(id, &format, &output)
        }
        Commands::Search {
            keyword,
            exact,
            field,
            limit,
            offset,
            prev,
            next,
            rebuild_index,
        } => banzhu_spider::cli::run_search(
            &keyword,
            exact,
            &field,
            limit,
            offset,
            prev,
            next,
            rebuild_index,
        ),
        Commands::Preview { id, chapter_num } => banzhu_spider::cli::run_preview(id, chapter_num),
        Commands::Import { path } => banzhu_spider::import::run_import(&path),
        Commands::Config { action } => banzhu_spider::cli::run_config(&action),
    } {
        eprintln!("错误: {}", e);
        std::process::exit(1);
    }

    Ok(())
}

fn log_setting() {
    use std::io::Write;
    let log_file = fs::File::create("spider.log").expect("Failed to create log file");
    env_logger::Builder::new()
        .format(|buf, record| {
            writeln!(
                buf,
                "{} [{}] - {}",
                buf.timestamp(),
                record.level(),
                record.args()
            )
        })
        .target(env_logger::Target::Pipe(Box::new(log_file)))
        .init();
}
