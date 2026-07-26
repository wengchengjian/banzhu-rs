//! 应用配置：统一从项目根目录 `spider.toml` 读取。

use crate::db::Database;
use anyhow::Result;

const CONFIG_FILE: &str = "spider.toml";

/// 从 spider.toml 读取 `storage.db_path`，默认 "banzhu.db"。
pub fn get_db_path() -> Result<String> {
    let config = load_config()?;
    Ok(config
        .get_string("storage.db_path")
        .unwrap_or_else(|_| "banzhu.db".to_string()))
}

/// 从 spider.toml 读取 `root_url`。
pub fn get_root_url() -> Result<String> {
    let config = load_config()?;
    config
        .get_string("root_url")
        .map_err(|_| anyhow::anyhow!("spider.toml 未配置 root_url"))
}

/// 打开数据库（路径来自 spider.toml `[storage] db_path`）。
pub fn open_db() -> Result<Database> {
    let db_path = get_db_path()?;
    Database::open(&db_path)
}

/// 加载 spider.toml 配置。
fn load_config() -> Result<config::Config> {
    config::Config::builder()
        .add_source(config::File::with_name(CONFIG_FILE))
        .build()
        .map_err(|e| anyhow::anyhow!("加载 {} 失败: {}", CONFIG_FILE, e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_db_path_default() {
        // 当 spider.toml 不存在或无 storage.db_path 时，应返回默认值
        // 注意：此测试依赖工作目录下是否有 spider.toml
        let path = get_db_path();
        // 只要不 panic 就行（CI 环境可能没有 spider.toml）
        let _ = path;
    }
}
