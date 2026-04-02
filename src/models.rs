//! 数据模型模块
//!
//! 定义所有核心数据结构

use serde::{Deserialize, Serialize};

/// 股票基础信息
#[derive(Debug, Clone)]
pub struct Stock {
    /// 股票代码，如 "sh603667"
    pub code: String,
    /// 股票名称，如 "五洲新春"
    pub name: String,
    /// 市场代码，"sh" 或 "sz"
    pub market: String,
}

/// 实时行情数据
#[derive(Debug, Clone)]
pub struct Quote {
    /// 股票代码
    pub code: String,
    /// 股票名称
    pub name: String,
    /// 开盘价
    pub open: f64,
    /// 昨收价
    pub prev_close: f64,
    /// 当前价
    pub current: f64,
    /// 最高价
    pub high: f64,
    /// 最低价
    pub low: f64,
    /// 涨跌额
    pub change: f64,
    /// 涨跌幅（%）
    pub change_pct: f64,
}

impl Quote {
    /// 计算涨跌幅
    pub fn calculate_change_pct(&mut self) {
        if self.prev_close > 0.0 {
            self.change = self.current - self.prev_close;
            self.change_pct = (self.change / self.prev_close) * 100.0;
        } else {
            self.change = 0.0;
            self.change_pct = 0.0;
        }
    }
}

/// 告警记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    /// Unix 时间戳
    pub timestamp: i64,
    /// 股票代码
    pub stock_code: String,
    /// 股票名称
    pub stock_name: String,
    /// 当前价
    pub price: f64,
    /// 涨跌幅（%）
    pub change_percent: f64,
    /// 告警类型
    pub alert_type: AlertType,
}

/// 告警类型
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AlertType {
    /// 普通异动（F3 条件）
    Normal,
    /// 开盘异动（F4 条件）
    Open,
}

impl std::fmt::Display for AlertType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AlertType::Normal => write!(f, "Normal"),
            AlertType::Open => write!(f, "Open"),
        }
    }
}

/// 异动方向
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Direction {
    /// 上涨
    Up,
    /// 下跌
    Down,
}

impl std::fmt::Display for Direction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Direction::Up => write!(f, "Up"),
            Direction::Down => write!(f, "Down"),
        }
    }
}
