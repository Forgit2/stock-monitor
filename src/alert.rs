//! 告警记录与去重模块
//!
//! 管理告警记录的持久化和去重逻辑

use crate::models::{Alert, Direction};
use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{BufReader, BufWriter, Write};
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AlertError {
    #[error("IO错误: {0}")]
    IoError(#[from] std::io::Error),
    #[error("JSON解析错误: {0}")]
    JsonError(#[from] serde_json::Error),
}

type StockCode = String;

/// 告警存储结构
pub struct AlertStore {
    /// 去重表: (stock_code, direction) -> last_alert_timestamp
    dedup: HashMap<(StockCode, Direction), i64>,
    /// 告警历史
    alerts: Vec<Alert>,
    /// 记录保留天数
    retention_days: u32,
    /// 告警历史文件路径
    history_file: String,
}

impl AlertStore {
    /// 创建新的告警存储
    pub fn new(log_dir: &str, retention_days: u32) -> Result<Self, AlertError> {
        let history_file = format!("{}/alerts.json", log_dir);

        // 确保日志目录存在
        fs::create_dir_all(log_dir)?;

        // 加载历史告警记录
        let mut store = AlertStore {
            dedup: HashMap::new(),
            alerts: Vec::new(),
            retention_days,
            history_file,
        };

        store.load_history()?;
        store.cleanup_old_records()?;

        Ok(store)
    }

    /// 从文件加载告警历史
    fn load_history(&mut self) -> Result<(), AlertError> {
        let path = Path::new(&self.history_file);
        if !path.exists() {
            return Ok(());
        }

        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let alerts: Vec<Alert> = serde_json::from_reader(reader)?;
        self.alerts = alerts;

        tracing::info!(count = self.alerts.len(), "加载了历史告警记录");
        Ok(())
    }

    /// 清理超过保留期限的记录
    fn cleanup_old_records(&mut self) -> Result<(), AlertError> {
        let now = chrono::Utc::now().timestamp();
        let cutoff = now - (self.retention_days as i64 * 24 * 3600);

        let original_len = self.alerts.len();
        self.alerts.retain(|alert| alert.timestamp >= cutoff);
        let removed = original_len - self.alerts.len();

        if removed > 0 {
            tracing::info!(removed = removed, "清理了过期的告警记录");
            self.save_history()?;
        }

        Ok(())
    }

    /// 保存告警历史到文件
    fn save_history(&self) -> Result<(), AlertError> {
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&self.history_file)?;
        let mut writer = BufWriter::new(file);
        serde_json::to_writer_pretty(&mut writer, &self.alerts)?;
        writer.flush()?;
        Ok(())
    }

    /// 判断是否应该告警（去重检查）
    /// 同一只股票同一方向在同一个交易日只告警一次
    pub fn should_alert(&self, stock_code: &str, direction: &Direction) -> bool {
        !self.dedup.contains_key(&(stock_code.to_string(), direction.clone()))
    }

    /// 记录已发送的告警
    pub fn record_alert(&mut self, stock_code: &str, direction: Direction) {
        let now = chrono::Utc::now().timestamp();
        self.dedup.insert((stock_code.to_string(), direction), now);
    }

    /// 日终重置：收盘后调用，重置所有去重标记
    pub fn reset_daily(&mut self) {
        self.dedup.clear();
        tracing::info!("日终重置去重表");
    }

    /// 添加告警记录并持久化
    pub fn add_alert(&mut self, alert: Alert) -> Result<(), AlertError> {
        self.alerts.push(alert);
        self.save_history()?;
        Ok(())
    }

    /// 获取所有告警历史
    pub fn get_alerts(&self) -> &[Alert] {
        &self.alerts
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_alert() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut store = AlertStore::new(temp_dir.path().to_str().unwrap(), 7).unwrap();

        // 第一次应该告警
        assert!(store.should_alert("sh603667", &Direction::Up));

        // 记录告警
        store.record_alert("sh603667", Direction::Up);

        // 同一方向不应再告警
        assert!(!store.should_alert("sh603667", &Direction::Up));

        // 不同方向可以告警
        assert!(store.should_alert("sh603667", &Direction::Down));

        // 不同股票可以告警
        assert!(store.should_alert("sz002050", &Direction::Up));
    }

    #[test]
    fn test_reset_daily() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut store = AlertStore::new(temp_dir.path().to_str().unwrap(), 7).unwrap();

        store.record_alert("sh603667", Direction::Up);
        assert!(!store.should_alert("sh603667", &Direction::Up));

        store.reset_daily();
        assert!(store.should_alert("sh603667", &Direction::Up));
    }
}
