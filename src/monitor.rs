//! 监控调度器模块
//!
//! 核心监控逻辑，实现所有功能需求

use crate::alert::AlertStore;
use crate::config::Config;
use crate::fetcher;
use crate::models::{Alert, AlertType, Direction};
use crate::notify;
use crate::timeutils::{is_before_market_open, is_market_close, is_trading_day, is_trading_time, now_in_shanghai};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

pub struct Monitor {
    config: Config,
    alert_store: Arc<Mutex<AlertStore>>,
}

impl Monitor {
    /// 创建新的监控器
    pub fn new(config: Config, alert_store: AlertStore) -> Self {
        Monitor {
            config,
            alert_store: Arc::new(Mutex::new(alert_store)),
        }
    }

    /// 启动监控循环
    pub async fn run(&self) {
        tracing::info!(
            version = "0.1.0",
            poll_interval = self.config.poll_interval,
            stock_count = self.config.stock_list.len(),
            "股票监测服务已启动"
        );

        // 获取所有股票代码（包括指数）
        let mut all_codes = self.config.stock_list.clone();
        if !all_codes.contains(&self.config.index_code) {
            all_codes.push(self.config.index_code.clone());
        }

        loop {
            let now = now_in_shanghai();

            // 检查是否为交易日
            if !is_trading_day(now) {
                tracing::debug!(date = %now.format("%Y-%m-%d"), "今日非交易日");
                tokio::time::sleep(Duration::from_secs(self.config.poll_interval as u64)).await;
                continue;
            }

            // 执行一轮监测
            if let Err(e) = self.poll_once(&all_codes, now).await {
                tracing::error!(error = ?e, "轮询执行出错");
            }

            // 检查是否收盘，收盘后重置去重表
            if is_market_close(now) {
                let mut store = self.alert_store.lock().await;
                store.reset_daily();
            }

            // 等待下次轮询
            tokio::time::sleep(Duration::from_secs(self.config.poll_interval as u64)).await;
        }
    }

    /// 执行单次轮询
    async fn poll_once(&self, codes: &[String], now: crate::timeutils::DateTime<crate::timeutils::TzShanghai>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        tracing::debug!(time = %now.format("%Y-%m-%d %H:%M:%S"), "开始轮询");

        // 并发获取所有股票数据
        let quotes = fetcher::fetch_all(codes).await;

        if quotes.is_empty() {
            tracing::warn!("未能获取任何股票数据");
            return Ok(());
        }

        // 获取上证指数数据
        let index_quote = quotes.get(&self.config.index_code);

        // F3 条件判断：上证异动 + 个股异动 + 交易时段
        if let Some(index) = index_quote {
            let in_trading = is_trading_time(now);
            let index_move = index.change_pct.abs() >= self.config.index_threshold;

            if index_move && in_trading {
                tracing::info!(
                    index_code = %self.config.index_code,
                    index_change = %index.change_pct,
                    "检测到上证指数异动"
                );

                // 检查个股异动
                for code in &self.config.stock_list {
                    if let Some(quote) = quotes.get(code) {
                        let stock_move = quote.change_pct.abs() >= self.config.stock_threshold;
                        if stock_move {
                            let direction = if quote.change_pct >= 0.0 {
                                Direction::Up
                            } else {
                                Direction::Down
                            };

                            let mut store = self.alert_store.lock().await;
                            if store.should_alert(code, &direction) {
                                let alert = Alert {
                                    timestamp: chrono::Utc::now().timestamp(),
                                    stock_code: quote.code.clone(),
                                    stock_name: quote.name.clone(),
                                    price: quote.current,
                                    change_percent: quote.change_pct,
                                    alert_type: AlertType::Normal,
                                };

                                tracing::info!(
                                    code = %quote.code,
                                    name = %quote.name,
                                    change = %quote.change_pct,
                                    "发送普通异动告警"
                                );

                                store.record_alert(code, direction.clone());
                                if let Err(e) = store.add_alert(alert.clone()) {
                                    tracing::error!(error = ?e, "保存告警记录失败");
                                }
                                drop(store);

                                notify::send_alert(&self.config.feishu_webhook, &alert);
                            }
                        }
                    }
                }
            }
        }

        // F4 条件判断：开盘异动（仅在 09:30 前有效）
        if is_before_market_open(now) {
            for code in &self.config.stock_list {
                if let Some(quote) = quotes.get(code) {
                    // 计算开盘价相对昨收的涨跌幅
                    let open_change_pct = if quote.prev_close > 0.0 {
                        ((quote.open - quote.prev_close) / quote.prev_close) * 100.0
                    } else {
                        0.0
                    };

                    let open_move = open_change_pct.abs() >= self.config.open_threshold;

                    if open_move {
                        let direction = if open_change_pct >= 0.0 {
                            Direction::Up
                        } else {
                            Direction::Down
                        };

                        let mut store = self.alert_store.lock().await;
                        if store.should_alert(code, &direction) {
                            let alert = Alert {
                                timestamp: chrono::Utc::now().timestamp(),
                                stock_code: quote.code.clone(),
                                stock_name: quote.name.clone(),
                                price: quote.open,
                                change_percent: open_change_pct,
                                alert_type: AlertType::Open,
                            };

                            tracing::info!(
                                code = %quote.code,
                                name = %quote.name,
                                open = %quote.open,
                                open_change = %open_change_pct,
                                "发送开盘异动告警"
                            );

                            store.record_alert(code, direction.clone());
                            if let Err(e) = store.add_alert(alert.clone()) {
                                tracing::error!(error = ?e, "保存告警记录失败");
                            }
                            drop(store);

                            notify::send_alert(&self.config.feishu_webhook, &alert);
                        }
                    }
                }
            }
        }

        Ok(())
    }
}
