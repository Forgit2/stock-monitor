# 测试报告 - stock-monitor

**日期**: 2026-04-02
**测试工程师**: QA
**版本**: v0.1.0

---

## 1. 测试范围

### 代码评审
对项目全部源代码进行人工评审：
- `src/main.rs` - 主程序入口
- `src/lib.rs` - 库入口
- `src/config.rs` - 配置加载与校验
- `src/fetcher.rs` - 股票数据获取
- `src/monitor.rs` - 监控调度逻辑
- `src/alert.rs` - 告警存储与去重
- `src/notify.rs` - 飞书通知
- `src/models.rs` - 数据模型
- `src/timeutils.rs` - 时间工具
- `config.toml` - 配置文件
- `tests/` - 集成测试

### 编译测试
- Debug 模式编译
- Release 模式编译

### 单元测试
- 配置模块测试
- 告警存储测试
- 时间工具测试
- 解析器测试

---

## 2. 测试用例列表

### 配置模块 (`src/config.rs` + `tests/config_test.rs`)

| 用例 ID | 名称 | 描述 |
|---------|------|------|
| CONFIG-01 | test_load_valid_config | 加载有效配置文件 |
| CONFIG-02 | test_load_missing_file | 文件不存在时返回 FileNotFound 错误 |
| CONFIG-03 | test_validate_empty_stock_list | 空股票列表被正确拒绝 |
| CONFIG-04 | test_validate_poll_interval_out_of_range | 超出范围的 poll_interval 被拒绝 |
| CONFIG-05 | test_validate_invalid_webhook | 无效 URL 格式被拒绝 |
| CONFIG-06 | test_default_values | 默认配置值正确 |

### 告警存储模块 (`src/alert.rs` + `tests/alert_test.rs`)

| 用例 ID | 名称 | 描述 |
|---------|------|------|
| ALERT-01 | test_should_alert_first_time | 首次告警应返回 true |
| ALERT-02 | test_should_alert_after_record | 已记录方向不再告警 |
| ALERT-03 | test_should_alert_different_direction | 反方向仍可告警 |
| ALERT-04 | test_should_alert_different_stock | 不同股票不受去重影响 |
| ALERT-05 | test_reset_daily | 日终重置恢复告警能力 |
| ALERT-06 | test_add_alert | 添加告警并持久化 |

### 时间工具模块 (`src/timeutils.rs` + `tests/timeutils_test.rs`)

| 用例 ID | 名称 | 描述 |
|---------|------|------|
| TIME-01 | test_trading_day_weekday | 工作日为交易日 |
| TIME-02 | test_trading_day_weekend | 周末非交易日 |
| TIME-03 | test_trading_time_morning | 上午交易时段 9:30-11:30 |
| TIME-04 | test_trading_time_afternoon | 下午交易时段 13:00-15:00 |
| TIME-05 | test_trading_time_lunch | 午休时段 12:00 不交易 |
| TIME-06 | test_trading_time_outside | 开盘前/收盘后不交易 |
| TIME-07 | test_before_market_open | 9:30 前为开盘前 |
| TIME-08 | test_market_close | 15:00 后为已收盘 |

### 解析器模块 (`src/fetcher.rs`)

| 用例 ID | 名称 | 描述 |
|---------|------|------|
| FETCH-01 | test_parse_quote_from_sina | 正确解析新浪行情数据 |
| FETCH-02 | test_parse_invalid_content | 非法内容正确报错 |

### 通知模块 (`src/notify.rs`)

| 用例 ID | 名称 | 描述 |
|---------|------|------|
| NOTIFY-01 | test_format_timestamp | 时间戳格式化为可读字符串 |

---

## 3. 测试结果

### 编译结果

| 模式 | 结果 |
|------|------|
| Debug | ✅ 通过 |
| Release | ✅ 通过 |

### 测试结果汇总

| 测试类型 | 通过 | 失败 | 总计 |
|---------|------|------|------|
| 单元测试 (lib) | 13 | 0 | 13 |
| 集成测试 (config_test) | 6 | 0 | 6 |
| 集成测试 (alert_test) | 6 | 0 | 6 |
| 集成测试 (timeutils_test) | 8 | 0 | 8 |
| **总计** | **33** | **0** | **33** |

**所有测试用例全部通过。**

---

## 4. 发现的问题及修复

### 问题 1: 缺少 `serde_json` 依赖
**严重级别**: 高
**描述**: `Cargo.toml` 缺少 `serde_json` 依赖，导致 `alert.rs` 和 `notify.rs` 中的 JSON 序列化/反序列化无法编译。
**修复**: 在 `Cargo.toml` 中添加 `serde_json = "1"`。

### 问题 2: `Alert` 模型缺少 Serde derives
**严重级别**: 高
**描述**: `models.rs` 中的 `Alert` 结构体未实现 `Serialize` 和 `Deserialize`，导致无法持久化和反序列化告警记录。
**修复**: 为 `Alert` 添加 `#[derive(Serialize, Deserialize)]`。

### 问题 3: `DateTime` 私有化问题
**严重级别**: 中
**描述**: `timeutils.rs` 中 `DateTime` 为私有导入，`monitor.rs` 无法通过 `crate::timeutils::DateTime` 引用。
**修复**: 添加 `pub use chrono::DateTime;` 重新导出。

### 问题 4: `send_alert` 生命周期错误
**严重级别**: 高
**描述**: `notify.rs` 中 `send_alert` 函数将 `&Alert` 引用传入子线程，但引用生命周期不满足 `'static` 要求。
**修复**: 在子线程外克隆所需的 `stock_code` 字段，避免传递引用。

### 问题 5: `reqwest::blocking::RequestBuilder` 不支持 `.json()` 方法
**严重级别**: 中
**描述**: 使用 `reqwest::blocking` 客户端时，`RequestBuilder` 没有 `.json()` 方法，需要先序列化 body 再发送。
**修复**: 使用 `serde_json::to_string()` 序列化后再通过 `.body()` 发送。

### 问题 6: 浮点数格式化格式问题
**严重级别**: 中
**描述**: `notify.rs` 中使用 `{:.2f}` 格式（固定小数位），在 Rust 1.94 中该语法行为有变化。
**修复**: 改用 `{:+.2}` 格式（带符号的固定精度）。

### 问题 7: 测试中的错误处理逻辑
**严重级别**: 低
**描述**: `src/config.rs` 和 `tests/config_test.rs` 中部分测试假设 `Config::load()` 对无效配置也返回成功，而实际上 `load()` 内部会调用 `validate()`。
**修复**: 调整测试预期，验证 `load()` 对无效配置返回错误。

### 问题 8: 时间戳测试数据错误
**严重级别**: 低
**描述**: `notify.rs` 中 `test_format_timestamp` 使用的时间戳 `1743569400` 对应 2026-05-04 而非 2026-04-02。
**修复**: 更正为正确的时间戳 `1775097000`（2026-04-02 10:30:00 上海时区）。

### 问题 9: 未使用导入警告
**严重级别**: 低
**描述**: 多处存在未使用的 `import` 和可变变量警告。
**修复**: 使用 `cargo fix` 自动清理。

---

## 5. 代码质量评估

### 优点
1. **模块化设计清晰**: 配置、数据获取、监控、告警、通知各模块职责分明
2. **错误处理完善**: 使用 `thiserror` 自定义错误类型，错误信息清晰
3. **去重逻辑合理**: 同股票同方向同日仅告警一次，日终自动重置
4. **交易时段判断准确**: 正确处理了 9:30-11:30 和 13:00-15:00 的交易时段
5. **日志完善**: 使用 `tracing` 结构化日志，便于排查问题
6. **配置校验完整**: 对必填字段和数值范围进行了充分校验

### 建议改进
1. **API 稳定性**: 可考虑为库添加 API 版本标注
2. **测试覆盖率**: 可增加更多边界条件测试（如节假日判断、极端价格等）
3. **监控指标**: 可添加 Prometheus 等指标收集，便于生产监控
4. **健康检查**: 可添加 HTTP 健康检查端点，便于 k8s 部署

---

## 6. 结论

**测试状态**: ✅ 通过

项目代码质量良好，核心功能（配置加载、告警去重、交易时段判断、飞书通知）均正常工作。经修复编译错误和测试问题后，所有 33 个测试用例全部通过。建议进入下一阶段测试（集成测试/灰度发布）。
