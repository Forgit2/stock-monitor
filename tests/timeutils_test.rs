use stock_monitor::timeutils::{
    is_before_market_open, is_market_close, is_trading_day, is_trading_time, now_in_shanghai,
    TzShanghai,
};
use chrono::TimeZone;

fn create_dt(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> chrono::DateTime<TzShanghai> {
    let offset = chrono::FixedOffset::east_opt(8 * 3600).unwrap();
    offset.with_ymd_and_hms(year, month, day, hour, minute, 0).unwrap()
}

#[test]
fn test_is_trading_day_weekday() {
    // 2026-04-02 是周四（交易日）
    let thursday = create_dt(2026, 4, 2, 10, 0);
    assert!(is_trading_day(thursday));

    // 2026-04-03 是周五（交易日）
    let friday = create_dt(2026, 4, 3, 10, 0);
    assert!(is_trading_day(friday));

    // 2026-04-06 是周一（交易日）
    let monday = create_dt(2026, 4, 6, 10, 0);
    assert!(is_trading_day(monday));
}

#[test]
fn test_is_trading_day_weekend() {
    // 周六不是交易日
    let saturday = create_dt(2026, 4, 4, 10, 0);
    assert!(!is_trading_day(saturday));

    // 周日不是交易日
    let sunday = create_dt(2026, 4, 5, 10, 0);
    assert!(!is_trading_day(sunday));
}

#[test]
fn test_is_trading_time_morning() {
    // 9:30 开盘
    assert!(is_trading_time(create_dt(2026, 4, 2, 9, 30)));
    // 10:00
    assert!(is_trading_time(create_dt(2026, 4, 2, 10, 0)));
    // 11:00
    assert!(is_trading_time(create_dt(2026, 4, 2, 11, 0)));
    // 11:30 上午休市
    assert!(is_trading_time(create_dt(2026, 4, 2, 11, 30)));
}

#[test]
fn test_is_trading_time_afternoon() {
    // 13:00 下午开盘
    assert!(is_trading_time(create_dt(2026, 4, 2, 13, 0)));
    // 14:00
    assert!(is_trading_time(create_dt(2026, 4, 2, 14, 0)));
    // 14:59
    assert!(is_trading_time(create_dt(2026, 4, 2, 14, 59)));
}

#[test]
fn test_is_trading_time_lunch() {
    // 12:00 午休
    assert!(!is_trading_time(create_dt(2026, 4, 2, 12, 0)));
    // 12:30
    assert!(!is_trading_time(create_dt(2026, 4, 2, 12, 30)));
}

#[test]
fn test_is_trading_time_outside() {
    // 9:00 未开盘
    assert!(!is_trading_time(create_dt(2026, 4, 2, 9, 0)));
    // 15:00 已收盘
    assert!(!is_trading_time(create_dt(2026, 4, 2, 15, 0)));
    // 8:00
    assert!(!is_trading_time(create_dt(2026, 4, 2, 8, 0)));
}

#[test]
fn test_is_before_market_open() {
    // 8:59
    assert!(is_before_market_open(create_dt(2026, 4, 2, 8, 59)));
    // 9:00
    assert!(is_before_market_open(create_dt(2026, 4, 2, 9, 0)));
    // 9:29
    assert!(is_before_market_open(create_dt(2026, 4, 2, 9, 29)));
    // 9:30 开市，不算之前
    assert!(!is_before_market_open(create_dt(2026, 4, 2, 9, 30)));
    // 10:00
    assert!(!is_before_market_open(create_dt(2026, 4, 2, 10, 0)));
}

#[test]
fn test_is_market_close() {
    // 14:59
    assert!(!is_market_close(create_dt(2026, 4, 2, 14, 59)));
    // 15:00 收盘
    assert!(is_market_close(create_dt(2026, 4, 2, 15, 0)));
    // 15:01
    assert!(is_market_close(create_dt(2026, 4, 2, 15, 1)));
    // 23:59
    assert!(is_market_close(create_dt(2026, 4, 2, 23, 59)));
}
