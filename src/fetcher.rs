//! 数据获取模块
//!
//! 调用新浪股票 API 获取实时行情

use crate::models::Quote;
use encoding_rs::GBK;
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum FetchError {
    #[error("请求失败: {0}")]
    RequestError(#[from] reqwest::Error),
    #[error("解析失败: {0}")]
    ParseError(String),
    #[error("股票代码无效: {0}")]
    InvalidCode(String),
}

/// 从新浪 API 获取单只股票行情
pub async fn fetch_quote(code: &str) -> Result<Quote, FetchError> {
    let url = format!("http://hq.sinajs.cn/list={}", code);
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    let mut resp = client.get(&url).send().await?;
    let bytes = resp.bytes().await?;

    // 新浪 API 返回 GBK 编码
    let (result, _, _) = GBK.decode(&bytes);
    let content = result.into_owned();

    parse_quote_from_sina(&content, code)
}

/// 从新浪返回的内容解析行情数据
fn parse_quote_from_sina(content: &str, expected_code: &str) -> Result<Quote, FetchError> {
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // 格式: var hq_str_sh600580="卧龙电驱,10.20,10.50,10.80,...";
        let content_part = line.strip_prefix("var hq_str_").ok_or_else(|| {
            FetchError::ParseError(format!("无法解析行: {}", line))
        })?;

        let (code_part, data_part) = content_part.split_once("=\"").ok_or_else(|| {
            FetchError::ParseError(format!("无法分割数据: {}", line))
        })?;

        let code = code_part.trim();
        if code != expected_code {
            continue;
        }

        let data = data_part.strip_suffix("\";").ok_or_else(|| {
            FetchError::ParseError(format!("数据格式错误: {}", line))
        })?;

        let fields: Vec<&str> = data.split(',').collect();
        if fields.len() < 6 {
            return Err(FetchError::ParseError(format!(
                "字段数量不足: {}, 需要至少 6 个",
                fields.len()
            )));
        }

        let name = fields[0].to_string();
        let open = fields[1].parse::<f64>().unwrap_or(0.0);
        let prev_close = fields[2].parse::<f64>().unwrap_or(0.0);
        let current = fields[3].parse::<f64>().unwrap_or(0.0);
        let high = fields[4].parse::<f64>().unwrap_or(0.0);
        let low = fields[5].parse::<f64>().unwrap_or(0.0);

        let mut quote = Quote {
            code: code.to_string(),
            name,
            open,
            prev_close,
            current,
            high,
            low,
            change: 0.0,
            change_pct: 0.0,
        };
        quote.calculate_change_pct();

        return Ok(quote);
    }

    Err(FetchError::ParseError(format!(
        "未找到股票代码 {} 的数据",
        expected_code
    )))
}

/// 批量获取多只股票的行情
pub async fn fetch_all(codes: &[String]) -> HashMap<String, Quote> {
    use tokio::task::JoinSet;

    let mut set = JoinSet::new();
    for code in codes {
        let code = code.clone();
        set.spawn(async move { fetch_quote(&code).await });
    }

    let mut results = HashMap::new();
    while let Some(res) = set.join_next().await {
        match res {
            Ok(Ok(quote)) => {
                results.insert(quote.code.clone(), quote);
            }
            Ok(Err(e)) => {
                tracing::warn!(code = ?e.to_string(), "获取股票数据失败");
            }
            Err(e) => {
                tracing::warn!(error = ?e, "任务执行失败");
            }
        }
    }
    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_quote_from_sina() {
        let content = r#"var hq_str_sh600580="卧龙电驱,10.20,10.50,10.80,10.45,10.20,10.50,10.80,10543200,10543200,10.49,...";"#;
        let result = parse_quote_from_sina(content, "sh600580");
        assert!(result.is_ok());
        let quote = result.unwrap();
        assert_eq!(quote.name, "卧龙电驱");
        assert_eq!(quote.open, 10.20);
        assert_eq!(quote.prev_close, 10.50);
        assert_eq!(quote.current, 10.80);
    }

    #[test]
    fn test_parse_invalid_content() {
        let content = "invalid content";
        let result = parse_quote_from_sina(content, "sh600580");
        assert!(result.is_err());
    }
}
