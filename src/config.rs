//! 配置加载模块
//!
//! 负责解析 config.toml，校验必填字段，设置默认值

use serde::Deserialize;
use std::fs;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("配置文件不存在: {0}")]
    FileNotFound(String),
    #[error("配置解析失败: {0}")]
    ParseError(String),
    #[error("配置校验失败: {0}")]
    ValidationError(String),
    #[error("IO错误: {0}")]
    IoError(#[from] std::io::Error),
}

#[derive(Debug, Deserialize)]
pub struct Config {
    /// 股票代码列表，格式为 市场代码+代码，如 sh603667
    pub stock_list: Vec<String>,
    /// 上证指数代码，默认 sh000001
    #[serde(default = "default_index_code")]
    pub index_code: String,
    /// 上证指数异动阈值（百分比），默认 1.0
    #[serde(default = "default_index_threshold")]
    pub index_threshold: f64,
    /// 个股异动阈值（百分比），默认 3.0
    #[serde(default = "default_stock_threshold")]
    pub stock_threshold: f64,
    /// 开盘异动阈值（百分比），默认 2.0
    #[serde(default = "default_open_threshold")]
    pub open_threshold: f64,
    /// 轮询间隔（秒），范围 60~3600，默认 300
    #[serde(default = "default_poll_interval")]
    pub poll_interval: u64,
    /// 飞书 Webhook URL
    pub feishu_webhook: String,
    /// 告警记录保留天数，默认 7
    #[serde(default = "default_retention_days")]
    pub retention_days: u32,
    /// 日志目录，默认 logs
    #[serde(default = "default_log_dir")]
    pub log_dir: String,
}

fn default_index_code() -> String {
    "sh000001".to_string()
}

fn default_index_threshold() -> f64 {
    1.0
}

fn default_stock_threshold() -> f64 {
    3.0
}

fn default_open_threshold() -> f64 {
    2.0
}

fn default_poll_interval() -> u64 {
    300
}

fn default_retention_days() -> u32 {
    7
}

fn default_log_dir() -> String {
    "logs".to_string()
}

impl Config {
    /// 从指定路径加载配置文件
    pub fn load(path: &str) -> Result<Self, ConfigError> {
        let path = Path::new(path);
        if !path.exists() {
            return Err(ConfigError::FileNotFound(path.display().to_string()));
        }

        let content = fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)
            .map_err(|e| ConfigError::ParseError(e.to_string()))?;

        config.validate()?;
        Ok(config)
    }

    /// 校验配置合法性
    pub fn validate(&self) -> Result<(), ConfigError> {
        // 校验 stock_list 非空
        if self.stock_list.is_empty() {
            return Err(ConfigError::ValidationError(
                "stock_list 不能为空".to_string(),
            ));
        }

        // 校验 poll_interval 范围
        if !(60..=3600).contains(&self.poll_interval) {
            return Err(ConfigError::ValidationError(
                format!("poll_interval {} 超出有效范围 60~3600", self.poll_interval),
            ));
        }

        // 校验 feishu_webhook 是有效 URL
        if !self.feishu_webhook.starts_with("http://")
            && !self.feishu_webhook.starts_with("https://")
        {
            return Err(ConfigError::ValidationError(
                "feishu_webhook 不是有效的 URL".to_string(),
            ));
        }

        // 校验阈值为正数
        if self.index_threshold <= 0.0 || self.stock_threshold <= 0.0 || self.open_threshold <= 0.0
        {
            return Err(ConfigError::ValidationError(
                "阈值必须为正数".to_string(),
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_load_valid_config() {
        let content = r#"
stock_list = ["sh603667", "sz002050"]
index_code = "sh000001"
index_threshold = 1.0
stock_threshold = 3.0
open_threshold = 2.0
poll_interval = 300
feishu_webhook = "https://open.feishu.cn/open-apis/bot/v2/hook/test"
retention_days = 7
log_dir = "logs"
"#;
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(content.as_bytes()).unwrap();

        let config = Config::load(file.path().to_str().unwrap()).unwrap();
        assert_eq!(config.stock_list.len(), 2);
        assert_eq!(config.index_code, "sh000001");
        assert_eq!(config.poll_interval, 300);
    }

    #[test]
    fn test_load_missing_file() {
        let result = Config::load("/nonexistent/path/config.toml");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ConfigError::FileNotFound(_)));
    }

    #[test]
    fn test_validate_empty_stock_list() {
        let content = r#"
stock_list = []
feishu_webhook = "https://open.feishu.cn/open-apis/bot/v2/hook/test"
"#;
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(content.as_bytes()).unwrap();

        let result = Config::load(file.path().to_str().unwrap());
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_poll_interval_out_of_range() {
        let content = r#"
stock_list = ["sh603667"]
feishu_webhook = "https://open.feishu.cn/open-apis/bot/v2/hook/test"
poll_interval = 5000
"#;
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(content.as_bytes()).unwrap();

        let result = Config::load(file.path().to_str().unwrap());
        assert!(result.is_err());
    }
}
