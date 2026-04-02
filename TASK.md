# 开发任务文档

**文档版本：** v1.0
**编写日期：** 2026-04-02
**作者：** 架构团队

---

## 一、项目概述

股票监测工具 `stock-monitor` 是一个运行在 Linux 上的长期守护进程，通过轮询免费股票 API 监测上证指数和自选股的异动，在触发预设条件时通过飞书机器人推送告警。

---

## 二、环境准备

### 2.1 开发环境

```bash
# 安装 Rust 1.70+（如果尚未安装）
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustc --version  # 确认 >= 1.70

# 安装交叉编译工具链（可选，用于 musl 静态编译）
rustup target add x86_64-unknown-linux-musl
```

### 2.2 初始化项目

```bash
cd /home/wy/workspace/team/output/stock-monitor
cargo init --name stock-monitor
```

---

## 三、配置文件说明（config.toml）

### 3.1 字段清单

| 字段名 | 类型 | 必填 | 默认值 | 说明 |
|-------|------|------|--------|------|
| `stock_list` | Vec\<String\> | 是 | — | 股票代码列表，格式为 `市场代码+代码`，如 `sh603667` |
| `index_code` | String | 否 | `sh000001` | 上证指数代码 |
| `index_threshold` | f64 | 否 | `1.0` | 上证指数异动阈值（百分比） |
| `stock_threshold` | f64 | 否 | `3.0` | 个股异动阈值（百分比） |
| `open_threshold` | f64 | 否 | `2.0` | 开盘异动阈值（百分比） |
| `poll_interval` | u64 | 否 | `300` | 轮询间隔（秒），范围 60~3600 |
| `feishu_webhook` | String | 是 | — | 飞书 Webhook URL |
| `retention_days` | u32 | 否 | `7` | 告警记录保留天数 |
| `log_dir` | String | 否 | `logs` | 日志目录 |

### 3.2 股票代码格式说明

- 上交所：前缀 `sh`，如 `sh603667`（五洲新春）
- 深交所：前缀 `sz`，如 `sz002050`（三花智控）
- 上证指数：`sh000001`

### 3.3 配置示例

```toml
stock_list = ["sh603667", "sz002050", "sh600580", "sz002896"]
index_code = "sh000001"
index_threshold = 1.0
stock_threshold = 3.0
open_threshold = 2.0
poll_interval = 300
feishu_webhook = "https://open.feishu.cn/open-apis/bot/v2/hook/f422ffbd-8754-4bba-b992-2e25abc14750"
retention_days = 7
log_dir = "logs"
```

---

## 四、模块开发要求

### 4.1 config.rs - 配置加载

**职责：** 解析 `config.toml`，校验必填字段，设置默认值。

**实现要点：**
- 使用 `toml` crate 解析配置文件
- 使用 `serde` 的 `Deserialize` 自动映射字段
- 校验 `poll_interval` 范围（60~3600）
- 校验 `stock_list` 非空
- 校验 `feishu_webhook` 是有效的 URL
- 如果 `config.toml` 不存在，返回带提示信息的错误（不是 panic）

```rust
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub stock_list: Vec<String>,
    pub index_code: Option<String>,  // 有默认值
    pub index_threshold: Option<f64>,
    pub stock_threshold: Option<f64>,
    pub open_threshold: Option<f64>,
    pub poll_interval: Option<u64>,
    pub feishu_webhook: String,
    pub retention_days: Option<u32>,
    pub log_dir: Option<String>,
}

impl Config {
    pub fn load(path: &str) -> Result<Self, ConfigError> { ... }
    pub fn validate(&self) -> Result<(), ConfigError> { ... }
}
```

### 4.2 models.rs - 数据模型

**职责：** 定义所有核心数据结构。

**实现要点：**
- `Stock`：股票基础信息（代码、名称、市场）
- `Quote`：实时行情（当前价、昨收、今开、最高、最低、涨跌额、涨跌幅）
- `Alert`：告警记录（时间戳、股票代码、名称、当前价、涨跌幅、告警类型）
- `AlertType`：`Normal`（普通异动F3）或 `Open`（开盘异动F4）
- `Direction`：`Up`（涨）或 `Down`（跌）
- 所有字段使用 `pub` 供其他模块访问

```rust
pub struct Stock {
    pub code: String,       // "sh603667"
    pub name: String,       // "五洲新春"
    pub market: String,     // "sh" 或 "sz"
}

pub struct Quote {
    pub code: String,
    pub name: String,
    pub open: f64,          // 开盘价
    pub prev_close: f64,   // 昨收价
    pub current: f64,       // 当前价
    pub high: f64,          // 最高价
    pub low: f64,           // 最低价
    pub change: f64,        // 涨跌额
    pub change_pct: f64,   // 涨跌幅（%）
}

pub struct Alert {
    pub timestamp: i64,           // Unix 时间戳
    pub stock_code: String,
    pub stock_name: String,
    pub price: f64,
    pub change_percent: f64,
    pub alert_type: AlertType,
}

pub enum AlertType { Normal, Open }
pub enum Direction { Up, Down }
```

### 4.3 fetcher.rs - 数据获取

**职责：** 调用新浪股票 API 获取实时行情。

**实现要点：**
- 新浪 API：`http://hq.sinajs.cn/list={codes}`，多个代码用逗号分隔
- 返回数据为 GBK 编码，需转换为 UTF-8
- 解析新浪返回的 JS 变量格式：`var hq_str_sh600580="卧龙电驱,10.20,10.50,10.80,...";`
- 字段解析顺序（按索引）：0=名称,1=今开,2=昨收,3=当前,4=最高,5=最低
- 使用 `reqwest` 异步并发请求，超时 10 秒
- 异常处理：单只股票解析失败不影响其他股票
- 返回 `HashMap<code, Result<Quote, FetchError>>`

**新浪 API 字段对应：**
| 索引 | 字段名 | 说明 |
|-----|-------|------|
| 0 | name | 股票名称 |
| 1 | open | 今开 |
| 2 | prev_close | 昨收 |
| 3 | current | 当前价 |
| 4 | high | 最高 |
| 5 | low | 最低 |

### 4.4 timeutils.rs - 时间工具

**职责：** 判断交易时间段和特殊时间节点。

**实现要点：**
- 强制使用 `Asia/Shanghai` 时区
- `is_trading_day()`：判断今天是否为交易日（周一~周五）
- `is_trading_time()`：判断当前是否在交易时间段（9:30-11:30, 13:00-15:00）
- `is_before_market_open()`：判断是否在 09:30 之前（用于开盘异动判断）
- `is_market_close()`：判断是否在 15:00 之后（用于日终重置去重标记）
- 接受 `chrono::DateTime<Asia/Shanghai>` 参数，不依赖系统本地时间

### 4.5 alert.rs - 告警记录与去重

**职责：** 管理告警记录的持久化和去重逻辑。

**实现要点：**
- `AlertStore` 结构体，管理内存中的去重表和告警历史
- 去重表：`HashMap<(stock_code, Direction), i64>`，记录该股票上次告警的 Unix 时间戳
- 同一只股票同一方向（涨/跌）在同一交易日内只告警一次
- 告警历史持久化到 `logs/alerts.json`（JSON 数组格式）
- 启动时清理超过 `retention_days` 的历史记录
- `should_alert(stock_code, direction) -> bool`：判断是否应该告警（去重检查）
- `record_alert(stock_code, direction) -> ()`：记录已告警
- `reset_daily() -> ()`：日终（15:00后）调用，重置所有去重标记

**告警历史 JSON 格式：**
```json
[
  {
    "timestamp": 1743569400,
    "stock_code": "sh603667",
    "stock_name": "五洲新春",
    "price": 10.50,
    "change_percent": 3.25,
    "alert_type": "Normal"
  }
]
```

### 4.6 notify.rs - 飞书通知

**职责：** 发送飞书机器人消息。

**实现要点：**
- 使用飞书 Card 消息格式（Markdown）
- POST JSON 到 Webhook URL
- 消息卡片内容：
  - 标题：告警类型（普通异动 / 开盘异动）
  - 股票名称 + 代码
  - 当前价 + 涨跌幅
  - 触发时间
- 发送失败记录 `ERROR` 日志，不抛异常，不重试，不阻塞调用方
- 使用 `reqwest` 同步 POST（简单场景无需异步）

**Card 消息示例：**
```json
{
  "msg_type": "interactive",
  "card": {
    "header": {
      "title": { "tag": "plain_text", "content": "⚠️ 个股异动告警" },
      "template": "red"
    },
    "elements": [
      { "tag": "markdown", "content": "**股票名称**：五洲新春（sh603667）\n**当前价**：10.50 元\n**涨跌幅**：+3.25%\n**异动时间**：2026-04-02 10:30:00" }
    ]
  }
}
```

### 4.7 monitor.rs - 监控调度器

**职责：** 核心监控逻辑，实现所有功能需求。

**实现要点：**
- `Monitor` 结构体，持有一个 `Config` 和 `AlertStore`
- `run()` 方法：启动无限循环，按 `poll_interval` 轮询
- 每轮执行步骤：
  1. 获取当前时间（Asia/Shanghai）
  2. 判断是否交易日
  3. 获取所有股票数据（并发）
  4. 判断每只股票是否异动（相对于昨收）
  5. 判断上证指数是否异动
  6. **F3 条件判断**：上证异动 + 个股异动 + 交易时段 → 发送飞书
  7. **F4 条件判断**：开盘异动（仅 09:30 前有效）→ 独立发送飞书
  8. 检查是否日终（15:00 后），是则重置去重表
  9. 睡眠等待下次轮询
- 异常处理：单次轮询出错记录日志，继续下次轮询

**异动计算公式：**
```
涨跌幅 = (当前价 - 昨收价) / 昨收价 * 100
涨跌额 = 当前价 - 昨收价
```

**开盘异动计算公式：**
```
开盘价差值 = (今开 - 昨收) / 昨收 * 100
```

### 4.8 main.rs - 程序入口

**职责：** 初始化日志，加载配置，启动监控。

**实现要点：**
- 初始化 `tracing-subscriber` 日志（stdout + 文件）
- 加载 `config.toml`（从当前目录或 `/etc/stock-monitor/`）
- 创建 `logs/` 目录（如不存在）
- 构建并启动 `Monitor::run()`
- 优雅退出：捕获 SIGINT/SIGTERM，设置 shutdown 标志
- 打印启动信息（版本、配置摘要）

---

## 五、关键实现细节

### 5.1 GBK 转 UTF-8

新浪 API 返回 GBK 编码，使用 `encoding_rs` crate 处理：

```rust
use encoding_rs::{GBK, Encoding};

fn gbk_to_utf8(bytes: &[u8]) -> String {
    let (result, _, _) = GBK.decode(bytes);
    result.into_owned()
}
```

### 5.2 解析股票数据

新浪返回示例：
```
var hq_str_sh600580="卧龙电驱,10.20,10.50,10.80,10.45,10.20,10.50,10.80,10543200,10543200,10.49,...";
```

解析逻辑：
```rust
fn parse_quote(line: &str) -> Option<Quote> {
    // 去掉 "var hq_str_sh600580=" 和末尾的 ";"
    let content = line.trim();
    let content = content.strip_prefix("var hq_str_").?;
    let (code_part, data_part) = content.split_once("=\"")?;
    let code = code_part.trim(); // "sh600580"
    let data = data_part.strip_suffix("\";")?;
    let fields: Vec<&str> = data.split(',').collect();
    // fields[0]=名称, [1]=今开, [2]=昨收, [3]=当前, [4]=最高, [5]=最低
    Some(Quote { code: code.to_string(), ... })
}
```

### 5.3 并发请求

```rust
use tokio::task::JoinSet;

async fn fetch_all(codes: &[String]) -> HashMap<String, Quote> {
    let mut set = JoinSet::new();
    for code in codes {
        let code = code.clone();
        set.spawn(async move { fetch_quote(&code).await });
    }
    let mut results = HashMap::new();
    while let Some(res) = set.join_next().await {
        if let Ok(Some(quote)) = res {
            results.insert(quote.code.clone(), quote);
        }
    }
    results
}
```

### 5.4 静态编译（生产部署）

```bash
# macOS/Linux 交叉编译为 musl 静态二进制
cargo build --release --target x86_64-unknown-linux-musl

# 结果文件在 target/x86_64-unknown-linux-musl/release/stock-monitor
# 可直接拷贝到目标 Linux 服务器运行
```

---

## 六、systemd 服务配置

创建 `stock-monitor.service`：

```ini
[Unit]
Description=Stock Monitor Service
After=network.target

[Service]
Type=simple
User=wy
WorkingDirectory=/home/wy/stock-monitor
ExecStart=/home/wy/stock-monitor/stock-monitor
Restart=always
RestartSec=10
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
```

部署步骤：
```bash
sudo cp stock-monitor.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable stock-monitor
sudo systemctl start stock-monitor
```

---

## 七、测试要求

### 7.1 单元测试

每个模块应有对应的单元测试：
- `config_test`：校验配置加载、默认值、错误处理
- `timeutils_test`：校验交易时间段判断（边界条件）
- `alert_test`：校验去重逻辑和记录清理

### 7.2 集成测试

- 手动测试：修改 `config.toml` 中的 Webhook 为测试地址，观察告警消息格式
- 使用真实 API 验证数据解析正确性

---

## 八、编码规范

- 遵循 Rust 官方编码规范（`rustfmt`）
- 所有公共函数写文档注释 `///`
- 错误处理使用 `Result<T, E>`，不轻易使用 `panic`
- 日志消息使用中文（便于运维）
- 代码中避免硬编码常量，统一放到 `config.rs` 或定义常量

---

## 九、交付检查清单

- [ ] `Cargo.toml` 依赖完整
- [ ] `src/` 下所有模块实现完成
- [ ] `config.toml` 配置文件可正常加载
- [ ] 所有功能需求（F1-F8）代码实现完整
- [ ] 单元测试覆盖核心逻辑
- [ ] `stock-monitor.service` systemd 服务文件
- [ ] 可成功编译：`cargo build --release`
- [ ] 二进制文件可独立运行（无运行时依赖）
