use stock_monitor::alert::AlertStore;
use stock_monitor::models::{Alert, AlertType, Direction};

#[test]
fn test_should_alert_first_time() {
    let temp_dir = tempfile::tempdir().unwrap();
    let store = AlertStore::new(temp_dir.path().to_str().unwrap(), 7).unwrap();

    // 第一次应该告警
    assert!(store.should_alert("sh603667", &Direction::Up));
    assert!(store.should_alert("sz002050", &Direction::Down));
}

#[test]
fn test_should_alert_after_record() {
    let temp_dir = tempfile::tempdir().unwrap();
    let mut store = AlertStore::new(temp_dir.path().to_str().unwrap(), 7).unwrap();

    // 第一次告警
    assert!(store.should_alert("sh603667", &Direction::Up));

    // 记录告警
    store.record_alert("sh603667", Direction::Up);

    // 同一方向不应再告警
    assert!(!store.should_alert("sh603667", &Direction::Up));
}

#[test]
fn test_should_alert_different_direction() {
    let temp_dir = tempfile::tempdir().unwrap();
    let mut store = AlertStore::new(temp_dir.path().to_str().unwrap(), 7).unwrap();

    // 记录上涨告警
    store.record_alert("sh603667", Direction::Up);

    // 下跌方向可以告警
    assert!(store.should_alert("sh603667", &Direction::Down));
}

#[test]
fn test_should_alert_different_stock() {
    let temp_dir = tempfile::tempdir().unwrap();
    let mut store = AlertStore::new(temp_dir.path().to_str().unwrap(), 7).unwrap();

    // 记录 sh603667 的告警
    store.record_alert("sh603667", Direction::Up);

    // sz002050 可以告警
    assert!(store.should_alert("sz002050", &Direction::Up));
}

#[test]
fn test_reset_daily() {
    let temp_dir = tempfile::tempdir().unwrap();
    let mut store = AlertStore::new(temp_dir.path().to_str().unwrap(), 7).unwrap();

    // 记录告警
    store.record_alert("sh603667", Direction::Up);
    store.record_alert("sz002050", Direction::Down);

    // 日终重置前不应告警
    assert!(!store.should_alert("sh603667", &Direction::Up));
    assert!(!store.should_alert("sz002050", &Direction::Down));

    // 重置
    store.reset_daily();

    // 重置后可以告警
    assert!(store.should_alert("sh603667", &Direction::Up));
    assert!(store.should_alert("sz002050", &Direction::Down));
}

#[test]
fn test_add_alert() {
    let temp_dir = tempfile::tempdir().unwrap();
    let mut store = AlertStore::new(temp_dir.path().to_str().unwrap(), 7).unwrap();

    let alert = Alert {
        timestamp: chrono::Utc::now().timestamp(),
        stock_code: "sh603667".to_string(),
        stock_name: "五洲新春".to_string(),
        price: 10.50,
        change_percent: 3.25,
        alert_type: AlertType::Normal,
    };

    store.add_alert(alert).unwrap();

    assert_eq!(store.get_alerts().len(), 1);
    assert_eq!(store.get_alerts()[0].stock_code, "sh603667");
}
