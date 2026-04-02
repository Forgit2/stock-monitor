# 技术设计文档

**文档版本：** v1.0
**编写日期：** 2026-04-02
**作者：** 架构团队

---

## 一、技术选型

### 1.1 语言与运行时

- **语言：** Rust（1.70+）
- **编译目标：** Linux x86_64，静态链接（musl），无外部运行时依赖
- **二进制部署：** 单文件可执行文件 + 配置文件

### 1.2 核心 Crates 选型

| 功能模块 | Crate | 版本 | 说明 |
|---------|-------|------|------|
| HTTP 客户端 | `reqwest` | 0.12 | 异步 HTTP 请求，支持 HTTPS |
| JSON 解析 | `serde` + `serde_json` | 1.0 | 序列化/反序列化 |
| 配置文件 | `toml` | 0.8 | 解析 TOML 配置文件 |
| 日期时间 | `chrono` | 0.4 | 时区处理（强制 Asia/Shanghai） |
| 日志 | `tracing` + `tracing-subscriber` | 0.1 | 结构化日志 |
| 异步运行时 | `tokio` | 1.0 | 异步 I/O 运行时 |
| 定时任务 | `tokio::time` | 内置 | 轮询间隔控制 |
| 持久化 | 标准库 `fs` | 内置 | JSON 文件读写 |
| 错误处理 | `thiserror` | 2.0 | 自定义错误类型 |

### 1.3 数据源

使用新浪股票数据 API（免费，无需认证）：

- 个股行情：`http://hq.sinajs.cn/list=sh600580`
- 上证指数：`http://hq.sinajs.cn/list=s_sh000001`
- 开盘价/昨收价：可从同一接口数据中提取

备选数据源：腾讯股票 API `http://qt.gtimg.cn/q=sh600580`

---

## 二、系统架构

```
┌──────────────────────────────────────────────────────────┐
│                      stock-monitor                        │
│                                                          │
│  ┌──────────────┐    ┌──────────────┐    ┌───────────┐ │
│  │ config.rs    │    │  monitor.rs  │    │ notify.rs │ │
│  │  配置加载    │───▶│  监控调度器   │───▶│ 飞书通知  │ │
│  └──────────────┘    └──────┬───────┘    └───────────┘ │
│                             │                            │
│  ┌──────────────┐    ┌──────▼───────┐    ┌───────────┐ │
│  │  stock.rs    │◀───│  fetcher.rs │    │ alert.rs  │ │
│  │  股票数据模型 │    │  数据获取    │    │ 告警记录  │ │
│  └──────────────┘    └──────────────┘    └───────────┘ │
│                                                          │
└──────────────────────────────────────────────────────────┘
```

### 模块职责

| 模块 | 职责 | 对外接口 |
|------|------|---------|
| `config` | 加载并校验 `config.toml` | `Config` 结构体 |
| `models` | 定义股票、告警、配置的数据模型 | `Stock`, `Alert`, `Config` |
| `fetcher` | 调用新浪/腾讯 API 获取实时行情 | `fetch_quotes()` |
| `monitor` | 定时轮询，判断异动条件 | `Monitor::run()` |
| `notify` | 发送飞书 Webhook 消息 | `send_feishu()` |
| `alert` | 告警记录持久化与去重 | `AlertStore` |
| `timeutils` | 交易时间段判断 | `is_trading_time()`, `is_market_open()` |

---

## 三、模块详细设计

### 3.1 config 模块

```rust
// 配置结构体（对应 config.toml）
pub struct Config {
    pub stock_list: Vec<String>,       // ["sh603667", "sz002050", ...]
    pub index_code: String,            // "sh000001"
    pub index_threshold: f64,         // 1.0 (%)
    pub stock_threshold: f64,          // 3.0 (%)
    pub open_threshold: f64,          // 2.0 (%)
    pub poll_interval_secs: u64,      // 300
    pub feishu_webhook: String,
    pub retention_days: u32,          // 7
    pub log_dir: String,               // "logs"
}
```

启动时从 `config.toml` 加载，校验必填字段，缺失使用默认值。

### 3.2 fetcher 模块

- 使用 `reqwest` 异步并发请求多个股票（单次轮询 HTTP 连接复用）
- 新浪接口返回 GBK 编码，需转 UTF-8
- 解析格式：`var hq_str_sh600580="六洲新春,10.50,10.20,10.80,10.45,10.20,10.50,10.80,10.49,...";`
  - 字段顺序：名称,今开,昨收,当前,最高,最低,...
- 超时统一设为 10 秒，单只股票超时不影响其他股票

### 3.3 monitor 模块

核心轮询循环：

```
每 poll_interval 秒：
  1. 获取当前时间（Asia/Shanghai）
  2. 判断是否交易时间段（9:30-11:30, 13:00-15:00，周末跳过）
  3. 并发获取所有股票 + 上证指数数据
  4. 遍历股票判断异动：
     - 计算涨跌额：(当前价 - 昨收价) / 昨收价 * 100
     - 判断是否超过阈值
     - 检查去重标记
  5. 判断上证指数是否异动
  6. 判断开盘异动（仅 09:30 前有效）
  7. 组装告警消息并发送飞书
  8. 持久化告警记录
  9. 日终（15:00）重置去重标记
```

### 3.4 notify 模块

- 使用飞书自定义机器人 Webhook（Card 消息格式）
- 消息卡片为 Markdown，包含：股票名称、代码、当前价、涨跌幅、异动时间
- 发送失败只记录日志，不重试，不阻塞
- HTTP POST JSON body

### 3.5 alert 模块

- 告警记录结构：`{timestamp, stock_code, stock_name, price, change_percent, alert_type}`
- 告警类型枚举：`Normal`（普通异动）、`Open`（开盘异动）
- 去重表：`HashMap<(stock_code, direction), bool>`，key = (代码, 涨跌方向)
- 持久化到 `logs/alerts.json`
- 启动时加载并清理超过 7 天的记录

---

## 四、配置文件格式（config.toml）

```toml
# stock-monitor 配置文件

# 股票列表（格式：市场代码+股票代码，市场代码 sh=上交所，sz=深交所）
stock_list = ["sh603667", "sz002050", "sh600580", "sz002896"]

# 上证指数代码
index_code = "sh000001"

# 异动阈值（%）
index_threshold = 1.0    # 上证指数异动阈值
stock_threshold = 3.0    # 个股异动阈值
open_threshold = 2.0     # 开盘异动阈值

# 监控周期（秒），允许范围 60~3600
poll_interval = 300

# 飞书 Webhook 地址
feishu_webhook = "https://open.feishu.cn/open-apis/bot/v2/hook/your-webhook-id"

# 告警记录保留天数
retention_days = 7

# 日志目录
log_dir = "logs"
```

---

## 五、数据流设计

```
[配置文件]
    │ config.toml
    ▼
[Config 加载] ──▶ 校验 + 设置默认值
    │
    ▼
[Monitor 主循环] ◀── 定时器（poll_interval）
    │
    ├─▶ [获取当前时间] ──▶ [判断是否交易时段]
    │
    ├─▶ [并发请求股票API] ──▶ [解析响应]
    │                              │
    │   ┌──────────────────────────┘
    │   ▼
    │ [计算异动]
    │   │
    │   ├─▶ [去重检查] ──▶ [AlertStore]
    │   │
    │   └─▶ [是否满足告警条件]
    │           │
    │           ├─ F3: 上证异动 + 个股异动 + 交易时段 → 飞书通知
    │           └─ F4: 开盘异动 → 独立飞书通知
    │
    ▼
[持久化告警] ──▶ logs/alerts.json
```

---

## 六、错误处理

| 错误场景 | 处理策略 | 日志级别 |
|---------|---------|---------|
| API 请求超时 | 跳过本次该股票，继续轮询 | WARN |
| API 返回异常数据 | 跳过该股票，记录错误 | WARN |
| 飞书通知发送失败 | 记录日志，不重试，不阻塞 | ERROR |
| 配置文件缺失/格式错误 | 程序退出，日志输出错误信息 | ERROR |
| 磁盘写入失败（alerts.json） | 记录日志，继续运行 | ERROR |
| 所有 API 不可用 | 日志记录，等待下次轮询 | WARN |
| 系统时间异常 | 记录警告，跳过本次轮询 | WARN |

---

## 七、日志设计

- 使用 `tracing` 结构化日志
- 日志输出到 stdout + 文件（`logs/app.log`）
- 日志格式：`[2026-04-02 10:30:00] [INFO] message`
- 关键事件：
  - `INFO`: 服务启动、配置加载、告警发送成功
  - `WARN`: API 超时、飞书发送失败、数据解析异常
  - `ERROR`: 配置错误、持久化失败
  - `DEBUG`: API 响应内容（仅在调试时开启）

---

## 八、文件结构

```
stock-monitor/
├── Cargo.toml
├── src/
│   ├── main.rs          # 入口，初始化日志和监控
│   ├── config.rs        # 配置加载与校验
│   ├── models.rs        # 数据模型定义
│   ├── fetcher.rs       # 股票数据获取
│   ├── monitor.rs       # 监控调度器
│   ├── notify.rs        # 飞书通知
│   ├── alert.rs         # 告警记录与去重
│   └── timeutils.rs     # 时间工具函数
├── config.toml          # 配置文件
├── stock-monitor.service # systemd 服务单元
├── logs/                # 日志目录（运行时创建）
└── alerts.json          # 告警记录（运行时创建）
```

---

## 九、关键设计决策

### 9.1 为什么用异步（tokio）而不是多线程？

- 监控任务以网络 I/O 为主，异步可大幅降低资源占用
- 单线程异步 + 少量线程即可满足需求
- tokio 生态成熟，与 reqwest/serde 配合良好

### 9.2 为什么用 TOML 而不是 JSON 做配置文件？

- TOML 语义更清晰，适合配置文件
- 支持注释，便于运维人员理解
- Rust 生态中 `toml` crate 成熟稳定

### 9.3 为什么告警记录用 JSON 而不是 SQLite？

- JSON 文件足够轻量，无需额外依赖
- 数据量小（每日几十条记录），文件 I/O 无性能问题
- 便于人工检查和调试

### 9.4 时区强制使用 Asia/Shanghai

- A股交易时间以北京时间为准
- 服务器时区可能不一致，必须强制覆盖
- 使用 `chrono` 的 `FixedTimeZone` 实现
