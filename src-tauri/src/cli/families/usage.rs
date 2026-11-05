//! `ccswitch usage ...`：用量汇总、趋势与请求日志。
//!
//! 时间范围统一用秒级时间戳传给后端（`startDate`/`endDate`）；命令行上 `--since` 支持
//! `7d`/`12h`/`30m` 这类相对写法、10/13 位时间戳，或 `YYYY-MM-DD` 日期。

use chrono::{Local, NaiveDate, TimeZone};
use serde_json::{json, Value};

use super::super::{Context, Parsed, Payload, Table};
use super::call;

pub fn run(ctx: &Context, parsed: &Parsed) -> Result<Payload, String> {
    match parsed.verb().unwrap_or("summary") {
        "summary" => summary(ctx, parsed),
        "trends" | "trend" => trends(ctx, parsed),
        "logs" | "log" => logs(ctx, parsed),
        other => Err(format!(
            "未知子命令 `usage {other}`（ccswitch usage --help）"
        )),
    }
}

fn field_str(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

fn number(value: &Value, key: &str) -> String {
    match value.get(key) {
        Some(Value::Number(number)) => number.to_string(),
        Some(Value::String(text)) => text.clone(),
        _ => "-".to_string(),
    }
}

/// 后端的时间戳有两种：用量汇总/日志是秒，个别接口给毫秒；统一成毫秒再格式化。
fn format_ts(ts: i64) -> String {
    let ms = if ts.abs() >= 1_000_000_000_000 {
        ts
    } else {
        ts.saturating_mul(1000)
    };
    Local
        .timestamp_millis_opt(ms)
        .single()
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_else(|| ts.to_string())
}

/// 解析 `--since/--until`：相对时长、时间戳或 `YYYY-MM-DD`。
///
/// 用量接口（`startDate` / `endDate` / 日志过滤器）用的是**秒**级时间戳，
/// 所以这里统一返回秒。
fn parse_time(raw: &str, label: &str) -> Result<i64, String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Err(format!("--{label} 不能为空"));
    }
    let now = Local::now().timestamp();
    for (suffix, unit) in [("d", 86_400i64), ("h", 3_600), ("m", 60)] {
        if let Some(number) = raw.strip_suffix(suffix) {
            let amount: i64 = number
                .parse()
                .map_err(|_| format!("--{label} 的时长写法要像 7d、12h、30m，收到 `{raw}`"))?;
            return Ok(now.saturating_sub(amount.saturating_mul(unit)));
        }
    }
    if raw.chars().all(|c| c.is_ascii_digit()) {
        let value: i64 = raw
            .parse()
            .map_err(|_| format!("--{label} 的时间戳无法解析：`{raw}`"))?;
        // 13 位及以上按毫秒处理，换算成秒；10 位就是秒。
        return Ok(if raw.len() >= 13 { value / 1000 } else { value });
    }
    let date = NaiveDate::parse_from_str(raw, "%Y-%m-%d")
        .map_err(|_| format!("--{label} 需要 YYYY-MM-DD 或时间戳，收到 `{raw}`"))?;
    let datetime = date
        .and_hms_opt(0, 0, 0)
        .ok_or_else(|| format!("--{label} 的日期不合法：`{raw}`"))?;
    Local
        .from_local_datetime(&datetime)
        .single()
        .map(|dt| dt.timestamp())
        .ok_or_else(|| format!("--{label} 的本地时间无法换算：`{raw}`"))
}

fn optional_app(parsed: &Parsed) -> Result<Option<String>, String> {
    if parsed.has_flag("app") {
        parsed.app(2).map(Some)
    } else {
        Ok(None)
    }
}

fn time_args(parsed: &Parsed) -> Result<(Option<i64>, Option<i64>), String> {
    let start = match parsed.flag("since") {
        Some(raw) => Some(parse_time(raw, "since")?),
        None => None,
    };
    let end = match parsed.flag("until") {
        Some(raw) => Some(parse_time(raw, "until")?),
        None => None,
    };
    Ok((start, end))
}

fn summary(ctx: &Context, parsed: &Parsed) -> Result<Payload, String> {
    let app = optional_app(parsed)?;
    let (start, end) = time_args(parsed)?;
    let value = call(
        ctx,
        "get_usage_summary",
        json!({
            "startDate": start,
            "endDate": end,
            "appType": app,
        }),
    )?;

    let mut text = String::new();
    if let Some(flag) = parsed.flag("since") {
        text.push_str(&format!("范围: 最近 {flag}\n"));
    }
    text.push_str(&format!(
        "请求数: {}（成功率 {}%）\n",
        number(&value, "totalRequests"),
        number(&value, "successRate")
    ));
    text.push_str(&format!("总花费: ${}\n", number(&value, "totalCost")));
    text.push_str(&format!(
        "Tokens: 输入 {} / 输出 {} / 缓存写 {} / 缓存读 {}（合计 {}）\n",
        number(&value, "totalInputTokens"),
        number(&value, "totalOutputTokens"),
        number(&value, "totalCacheCreationTokens"),
        number(&value, "totalCacheReadTokens"),
        number(&value, "realTotalTokens")
    ));
    // 后端 cacheHitRate 是 0.0–1.0 的比例（successRate 才是百分数）。
    let hit_rate = value
        .get("cacheHitRate")
        .and_then(Value::as_f64)
        .map(|rate| format!("{:.1}%", rate * 100.0))
        .unwrap_or_else(|| "-".to_string());
    text.push_str(&format!("缓存命中率: {hit_rate}\n"));
    Ok(Payload::Text(text))
}

fn trends(ctx: &Context, parsed: &Parsed) -> Result<Payload, String> {
    let app = optional_app(parsed)?;
    let (start, end) = time_args(parsed)?;
    let value = call(
        ctx,
        "get_usage_trends",
        json!({
            "startDate": start,
            "endDate": end,
            "appType": app,
        }),
    )?;
    let buckets = match value.as_array() {
        Some(buckets) => buckets.clone(),
        None => return Err(format!("get_usage_trends 返回了意外的形状：{value}")),
    };
    let mut table = Table::new(&["date", "requests", "tokens", "cost"]);
    for bucket in &buckets {
        table.push(vec![
            field_str(bucket, "date"),
            number(bucket, "requestCount"),
            number(bucket, "totalTokens"),
            number(bucket, "totalCost"),
        ]);
    }
    Ok(Payload::Table(table))
}

fn logs(ctx: &Context, parsed: &Parsed) -> Result<Payload, String> {
    let app = optional_app(parsed)?;
    let limit = match parsed.flag("limit") {
        Some(raw) => raw
            .parse::<u32>()
            .map_err(|_| format!("--limit 需要正整数，收到 `{raw}`"))?,
        None => 20,
    };
    let limit = limit.clamp(1, 500);
    let (start, end) = time_args(parsed)?;
    let value = call(
        ctx,
        "get_request_logs",
        json!({
            "filters": {
                "appType": app,
                "startDate": start,
                "endDate": end,
            },
            "page": 1,
            "pageSize": limit,
        }),
    )?;
    let logs = value
        .get("data")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let total = number(&value, "total");

    let mut table = Table::new(&[
        "time", "app", "model", "provider", "status", "tokens", "cost",
    ]);
    for log in &logs {
        let tokens = [
            "inputTokens",
            "outputTokens",
            "cacheReadTokens",
            "cacheCreationTokens",
        ]
        .iter()
        .map(|key| log.get(*key).and_then(Value::as_u64).unwrap_or(0))
        .sum::<u64>();
        let provider = match log.get("providerName").and_then(Value::as_str) {
            Some(name) if !name.is_empty() => name.to_string(),
            _ => field_str(log, "providerId"),
        };
        let status = number(log, "statusCode");
        table.push(vec![
            log.get("createdAt")
                .and_then(Value::as_i64)
                .map(format_ts)
                .unwrap_or_else(|| "-".to_string()),
            field_str(log, "appType"),
            field_str(log, "model"),
            provider,
            status,
            tokens.to_string(),
            number(log, "totalCostUsd"),
        ]);
    }
    if !ctx.quiet && !logs.is_empty() {
        eprintln!("ccswitch: 共 {total} 条，显示最近 {} 条", logs.len());
    }
    Ok(Payload::Table(table))
}
