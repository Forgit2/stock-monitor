//! 飞书通知模块
//!
//! 发送飞书机器人消息

use crate::models::{Alert, AlertType};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum NotifyError {
    #[error("请求失败: {0}")]
    RequestError(#[from] reqwest::Error),
    #[error("构建消息失败: {0}")]
    BuildError(String),
}

/// 飞书 Card 消息结构
#[derive(Debug, Serialize)]
struct FeishuCard {
    msg_type: String,
    card: FeishuCardContent,
}

#[derive(Debug, Serialize)]
struct FeishuCardContent {
    header: FeishuCardHeader,
    elements: Vec<serde_json::Value>,
}

#[derive(Debug, Serialize)]
struct FeishuCardHeader {
    title: FeishuText,
    template: String,
}

#[derive(Debug, Serialize)]
struct FeishuText {
    tag: String,
    content: String,
}

/// 发送飞书告警通知
pub fn send_alert(webhook: &str, alert: &Alert) {
    let title = match alert.alert_type {
        AlertType::Normal => "⚠️ 个股异动告警",
        AlertType::Open => "🚀 开盘异动告警",
    };

    let template = "red";
    let direction_symbol = if alert.change_percent >= 0.0 { "+" } else { "" };

    let content = format!(
        "**股票名称**：{}（{})\n**当前价**：{:.2f} 元\n**涨跌幅**：{}{:.2f}%\n**异动时间**：{}",
        alert.stock_name,
        alert.stock_code,
        alert.price,
        direction_symbol,
        alert.change_percent,
        format_timestamp(alert.timestamp)
    );

    let card = FeishuCard {
        msg_type: "interactive".to_string(),
        card: FeishuCardContent {
            header: FeishuCardHeader {
                title: FeishuText {
                    tag: "plain_text".to_string(),
                    content: title.to_string(),
                },
                template: template.to_string(),
            },
            elements: vec![serde_json::json!({
                "tag": "markdown",
                "content": content
            })],
        },
    };

    // 发送请求，不阻塞调用方
    let webhook = webhook.to_string();
    let card_clone = card;

    std::thread::spawn(move || {
        if let Err(e) = send_card_sync(&webhook, &card_clone) {
            tracing::error!(error = ?e, "发送飞书通知失败");
        } else {
            tracing::info!(code = %alert.stock_code, "飞书通知已发送");
        }
    });
}

/// 同步发送 Card 消息
fn send_card_sync(webhook: &str, card: &FeishuCard) -> Result<(), NotifyError> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    let response = client
        .post(webhook)
        .header("Content-Type", "application/json")
        .json(card)
        .send()?;

    if response.status().is_success() {
        Ok(())
    } else {
        Err(NotifyError::BuildError(format!(
            "HTTP错误: {}",
            response.status()
        )))
    }
}

/// 格式化时间戳
fn format_timestamp(timestamp: i64) -> String {
    use chrono::{TimeZone, Utc};
    let dt = Utc.timestamp_opt(timestamp, 0).unwrap();
    dt.format("%Y-%m-%d %H:%M:%S").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_timestamp() {
        let ts = 1743569400; // 2026-04-02 10:30:00
        let formatted = format_timestamp(ts);
        assert!(formatted.contains("2026"));
    }
}
