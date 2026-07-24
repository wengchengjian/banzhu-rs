use crate::db::Database;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

const CONFIG_DIR_NAME: &str = ".banzhu";
const CONFIG_FILE_NAME: &str = "config.toml";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub save_db_path: String,
    pub root_url: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        let db_path = get_config_dir()
            .expect("无法获取配置目录")
            .join("banzhu.db");
        Self {
            save_db_path: db_path.to_str().unwrap_or("banzhu.db").to_string(),
            root_url: "https://www.bz11111111.com/".to_string(),
        }
    }
}

fn get_config_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| anyhow!("无法获取用户主目录"))?;
    let config_dir = home.join(CONFIG_DIR_NAME);
    if !config_dir.exists() {
        fs::create_dir_all(&config_dir)?;
    }
    Ok(config_dir)
}

fn get_config_file_path() -> Result<PathBuf> {
    Ok(get_config_dir()?.join(CONFIG_FILE_NAME))
}

pub fn load_app_config() -> Result<AppConfig> {
    let config_path = get_config_file_path()?;
    if !config_path.exists() {
        let default_config = AppConfig::default();
        save_app_config(&default_config)?;
        return Ok(default_config);
    }

    let content = fs::read_to_string(&config_path)?;
    let config: AppConfig = toml::from_str(&content)?;
    Ok(config)
}

pub fn save_app_config(config: &AppConfig) -> Result<()> {
    let config_path = get_config_file_path()?;
    let content = toml::to_string_pretty(config)?;
    fs::write(&config_path, content)?;
    Ok(())
}

pub fn show_config() -> Result<()> {
    let config = load_app_config()?;
    println!("当前配置:");
    println!("  save_db_path = {}", config.save_db_path);
    println!("  root_url     = {}", config.root_url);
    println!();
    println!("配置文件路径: {}", get_config_file_path()?.display());
    Ok(())
}

pub fn set_config(key: &str, value: &str) -> Result<()> {
    let mut config = load_app_config()?;

    match key {
        "save_db_path" => {
            let path = Path::new(value);
            if let Some(parent) = path.parent() {
                if !parent.exists() {
                    fs::create_dir_all(parent)?;
                }
            }
            config.save_db_path = value.to_string();
        }
        "root_url" => {
            if !value.starts_with("http") {
                return Err(anyhow!("root_url 必须以 http:// 或 https:// 开头"));
            }
            config.root_url = value.to_string();
        }
        _ => return Err(anyhow!("未知的配置项: {}，支持: save_db_path, root_url", key)),
    }

    save_app_config(&config)?;
    println!("配置已更新: {} = {}", key, value);
    Ok(())
}

pub fn get_db_path() -> Result<String> {
    let config = load_app_config()?;
    Ok(config.save_db_path)
}

pub fn get_root_url() -> Result<String> {
    let config = load_app_config()?;
    Ok(config.root_url)
}

pub fn open_db() -> Result<Database> {
    let db_path = get_db_path()?;
    Database::open(&db_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert!(!config.save_db_path.is_empty());
        assert!(config.root_url.starts_with("http"));
    }

    #[test]
    fn test_config_serialization() {
        let config = AppConfig::default();
        let toml_str = toml::to_string_pretty(&config).unwrap();
        assert!(toml_str.contains("save_db_path"));
        assert!(toml_str.contains("root_url"));

        let parsed: AppConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.save_db_path, config.save_db_path);
        assert_eq!(parsed.root_url, config.root_url);
    }
}
