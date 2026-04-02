//! 时间工具模块
//!
//! 判断交易时间段和特殊时间节点

use chrono::{Timelike, Datelike};
pub use chrono::DateTime;

const SHANGHAI_OFFSET: i32 = 8 * 3600; // +08:00

/// 使用 Asia/Shanghai 时区
pub type TzShanghai = chrono::FixedOffset;

/// 获取 Asia/Shanghai 时区的当前时间
pub fn now_in_shanghai() -> DateTime<TzShanghai> {
    let offset = chrono::FixedOffset::east_opt(SHANGHAI_OFFSET).unwrap();
    chrono::Local::now().with_timezone(&offset)
}

/// 判断指定时间是否为交易日（周一~周五）
pub fn is_trading_day(dt: DateTime<TzShanghai>) -> bool {
    let weekday: chrono::Weekday = dt.weekday();
    // 周六和周日不是交易日
    weekday != chrono::Weekday::Sat && weekday != chrono::Weekday::Sun
}

/// 判断指定时间是否在交易时间段
/// 交易时间：9:30-11:30, 13:00-15:00
pub fn is_trading_time(dt: DateTime<TzShanghai>) -> bool {
    let hour = dt.hour();
    let minute = dt.minute();

    // 上午: 9:30 - 11:30
    let is_morning = (hour == 9 && minute >= 30) || (hour >= 10 && hour < 11) || (hour == 11 && minute <= 30);
    // 下午: 13:00 - 15:00
    let is_afternoon = hour >= 13 && hour < 15;

    is_morning || is_afternoon
}

/// 判断是否在开盘前（09:30 之前）
pub fn is_before_market_open(dt: DateTime<TzShanghai>) -> bool {
    let hour = dt.hour();
    let minute = dt.minute();
    hour < 9 || (hour == 9 && minute < 30)
}

/// 判断是否已收盘（15:00 之后）
pub fn is_market_close(dt: DateTime<TzShanghai>) -> bool {
    dt.hour() >= 15
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn create_dt(hour: u32, minute: u32) -> DateTime<TzShanghai> {
        let offset = chrono::FixedOffset::east_opt(SHANGHAI_OFFSET).unwrap();
        // 2026-04-02 是周四
        offset.with_ymd_and_hms(2026, 4, 2, hour, minute, 0).unwrap()
    }

    #[test]
    fn test_trading_day() {
        let offset = chrono::FixedOffset::east_opt(SHANGHAI_OFFSET).unwrap();
        // 周四
        let thursday = offset.with_ymd_and_hms(2026, 4, 2, 10, 0, 0).unwrap();
        assert!(is_trading_day(thursday));

        // 周六
        let saturday = offset.with_ymd_and_hms(2026, 4, 4, 10, 0, 0).unwrap();
        assert!(!is_trading_day(saturday));

        // 周日
        let sunday = offset.with_ymd_and_hms(2026, 4, 5, 10, 0, 0).unwrap();
        assert!(!is_trading_day(sunday));
    }

    #[test]
    fn test_trading_time() {
        // 9:30 - 交易中
        assert!(is_trading_time(create_dt(9, 30)));
        // 10:00 - 交易中
        assert!(is_trading_time(create_dt(10, 0)));
        // 11:30 - 交易中
        assert!(is_trading_time(create_dt(11, 30)));
        // 12:00 - 午休
        assert!(!is_trading_time(create_dt(12, 0)));
        // 13:00 - 交易中
        assert!(is_trading_time(create_dt(13, 0)));
        // 14:59 - 交易中
        assert!(is_trading_time(create_dt(14, 59)));
        // 9:00 - 未开盘
        assert!(!is_trading_time(create_dt(9, 0)));
        // 15:00 - 已收盘
        assert!(!is_trading_time(create_dt(15, 0)));
    }

    #[test]
    fn test_before_market_open() {
        assert!(is_before_market_open(create_dt(8, 59)));
        assert!(is_before_market_open(create_dt(9, 0)));
        assert!(is_before_market_open(create_dt(9, 29)));
        assert!(!is_before_market_open(create_dt(9, 30)));
        assert!(!is_before_market_open(create_dt(10, 0)));
    }

    #[test]
    fn test_market_close() {
        assert!(!is_market_close(create_dt(14, 59)));
        assert!(is_market_close(create_dt(15, 0)));
        assert!(is_market_close(create_dt(15, 1)));
        assert!(is_market_close(create_dt(23, 59)));
    }
}
