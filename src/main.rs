//! 股票监测工具
//!
//! 通过轮询免费股票 API 监测上证指数和自选股的异动，
//! 在触发预设条件时通过飞书机器人推送告警。

use clap::Parser;
use stock_monitor::alert::AlertStore;
use stock_monitor::config::Config;
use stock_monitor::monitor::Monitor;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tokio::signal;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[derive(Parser, Debug)]
#[command(name = "stock-monitor")]
#[command(version = "0.1.0")]
#[command(about = "股票监测工具 - 监测上证指数和自选股的异动")]
struct Args {
    /// 配置文件路径
    #[arg(short, long, default_value = "config.toml")]
    config: String,

    /// 日志目录
    #[arg(short, long)]
    log_dir: Option<String>,
}

fn init_logging(log_dir: &str) {
    // 确保日志目录存在
    fs::create_dir_all(log_dir).expect("无法创建日志目录");

    // 创建文件 appender
    let file_appender = RollingFileAppender::new(Rotation::DAILY, log_dir, "stock-monitor.log");

    // 构建 subscriber
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    let subscriber = tracing_subscriber::registry()
        .with(env_filter)
        .with(
            fmt::layer()
                .with_writer(std::io::stdout)
                .with_ansi(true)
                .with_target(true),
        )
        .with(
            fmt::layer()
                .with_writer(file_appender)
                .with_ansi(false)
                .with_target(true),
        );

    subscriber.init();
}

fn load_config(config_path: &str) -> Config {
    // 尝试多个路径
    let paths = [
        config_path,
        "/etc/stock-monitor/config.toml",
        "./config.toml",
    ];

    for path in &paths {
        if Path::new(path).exists() {
            match Config::load(path) {
                Ok(cfg) => {
                    tracing::info!(path = %path, "配置文件加载成功");
                    return cfg;
                }
                Err(e) => {
                    tracing::error!(path = %path, error = ?e, "配置文件加载失败");
                }
            }
        }
    }

    panic!("无法找到有效的配置文件");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // 加载配置
    let config = load_config(&args.config);

    // 确定日志目录
    let log_dir = args.log_dir.unwrap_or(config.log_dir.clone());

    // 初始化日志
    init_logging(&log_dir);

    tracing::info!(
        version = "0.1.0",
        stocks = ?config.stock_list,
        index = %config.index_code,
        poll_interval = config.poll_interval,
        "股票监测服务初始化"
    );

    // 确保日志目录存在
    fs::create_dir_all(&log_dir).map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

    // 初始化告警存储
    let alert_store = match AlertStore::new(&log_dir, config.retention_days) {
        Ok(store) => store,
        Err(e) => {
            tracing::error!(error = ?e, "告警存储初始化失败");
            return Err(Box::new(e) as Box<dyn std::error::Error>);
        }
    };

    // 创建监控器
    let monitor = Monitor::new(config, alert_store);

    // 设置优雅退出
    let monitor = Arc::new(monitor);
    let monitor_clone = monitor.clone();

    tokio::spawn(async move {
        signal::ctrl_c().await.expect("无法捕获 Ctrl+C");
        tracing::info!("收到退出信号，正在优雅关闭...");
    });

    // 运行监控
    monitor_clone.run().await;

    Ok(())
}
