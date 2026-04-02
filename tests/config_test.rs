use stock_monitor::config::{Config, ConfigError};
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
    assert_eq!(config.stock_threshold, 3.0);
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
poll_interval = 300
"#;
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(content.as_bytes()).unwrap();

    let config = Config::load(file.path().to_str().unwrap()).unwrap();
    let result = config.validate();
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

    let config = Config::load(file.path().to_str().unwrap()).unwrap();
    let result = config.validate();
    assert!(result.is_err());
}

#[test]
fn test_validate_invalid_webhook() {
    let content = r#"
stock_list = ["sh603667"]
feishu_webhook = "not-a-valid-url"
poll_interval = 300
"#;
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(content.as_bytes()).unwrap();

    let config = Config::load(file.path().to_str().unwrap()).unwrap();
    let result = config.validate();
    assert!(result.is_err());
}

#[test]
fn test_default_values() {
    let content = r#"
stock_list = ["sh603667"]
feishu_webhook = "https://open.feishu.cn/open-apis/bot/v2/hook/test"
"#;
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(content.as_bytes()).unwrap();

    let config = Config::load(file.path().to_str().unwrap()).unwrap();
    assert_eq!(config.index_code, "sh000001");
    assert_eq!(config.index_threshold, 1.0);
    assert_eq!(config.stock_threshold, 3.0);
    assert_eq!(config.poll_interval, 300);
    assert_eq!(config.retention_days, 7);
    assert_eq!(config.log_dir, "logs");
}
