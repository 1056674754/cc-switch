//! `ccswitch proxy ...`：本地路由（代理）状态、启停、接管与配置。
//!
//! 接管是设备级状态（各应用是否把 Live 配置指到本地路由），所以 `takeover` 不给
//! `--app` 时按「有直连/路由模式」的应用逐个处理：claude、codex、gemini、grokbuild、
//! claude-desktop。

use serde_json::{json, Value};

use super::super::{Context, Parsed, Payload};
use super::call;

/// 有直连 / 路由模式的应用（与 `AppType::has_route_mode` 保持一致）。
const ROUTE_MODE_APPS: [&str; 5] = ["claude", "codex", "gemini", "grokbuild", "claude-desktop"];

pub fn run(ctx: &Context, parsed: &Parsed) -> Result<Payload, String> {
    match parsed.verb().unwrap_or("status") {
        "status" => status(ctx),
        "start" => start(ctx),
        "stop" => stop(ctx),
        "takeover" => takeover(ctx, parsed),
        "config" => config(ctx, parsed),
        other => Err(format!(
            "未知子命令 `proxy {other}`（ccswitch proxy --help）"
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

fn truthy(value: &Value, key: &str) -> bool {
    value.get(key).and_then(Value::as_bool).unwrap_or(false)
}

fn uptime(seconds: u64) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let rest = seconds % 60;
    if hours > 0 {
        format!("{hours}小时{minutes}分")
    } else if minutes > 0 {
        format!("{minutes}分{rest}秒")
    } else {
        format!("{rest}秒")
    }
}

fn status(ctx: &Context) -> Result<Payload, String> {
    let status = call(ctx, "get_proxy_status", json!({}))?;
    let takeover = call(ctx, "get_proxy_takeover_status", json!({})).unwrap_or(Value::Null);

    let running = truthy(&status, "running");
    let mut text = String::new();
    if running {
        text.push_str(&format!(
            "本地路由: 运行中 http://{}:{}\n",
            field_str(&status, "address"),
            number(&status, "port")
        ));
        text.push_str(&format!(
            "  请求: 共 {}（成功 {} / 失败 {}，成功率 {}%）\n",
            number(&status, "total_requests"),
            number(&status, "success_requests"),
            number(&status, "failed_requests"),
            number(&status, "success_rate")
        ));
        text.push_str(&format!(
            "  活跃连接: {}  故障转移: {}\n",
            number(&status, "active_connections"),
            number(&status, "failover_count")
        ));
        let provider = field_str(&status, "current_provider");
        if !provider.is_empty() {
            text.push_str(&format!("  当前供应商: {provider}\n"));
        }
        if let Some(seconds) = status.get("uptime_seconds").and_then(Value::as_u64) {
            text.push_str(&format!("  运行时长: {}\n", uptime(seconds)));
        }
        if let Some(error) = status.get("last_error").and_then(Value::as_str) {
            if !error.is_empty() {
                text.push_str(&format!("  最近错误: {error}\n"));
            }
        }
    } else {
        text.push_str("本地路由: 未运行（ccswitch proxy start）\n");
    }

    let apps: Vec<String> = match takeover.as_object() {
        Some(map) => map
            .iter()
            .filter(|(_, enabled)| enabled.as_bool().unwrap_or(false))
            .map(|(app, _)| app.clone())
            .collect(),
        None => Vec::new(),
    };
    if apps.is_empty() {
        text.push_str("接管: 无（各应用直连）\n");
    } else {
        text.push_str(&format!("接管: {}\n", apps.join(", ")));
    }
    Ok(Payload::Text(text))
}

fn start(ctx: &Context) -> Result<Payload, String> {
    let info = call(ctx, "start_proxy_server", json!({}))?;
    Ok(Payload::Text(format!(
        "已启动本地路由：http://{}:{}\n",
        field_str(&info, "address"),
        number(&info, "port")
    )))
}

fn stop(ctx: &Context) -> Result<Payload, String> {
    call(ctx, "stop_proxy_with_restore", json!({}))?;
    Ok(Payload::Text(
        "已停止本地路由，并恢复被接管的客户端配置\n".to_string(),
    ))
}

fn on_off(raw: &str) -> Result<bool, String> {
    match raw.to_lowercase().as_str() {
        "on" | "true" | "1" | "yes" | "enable" | "enabled" => Ok(true),
        "off" | "false" | "0" | "no" | "disable" | "disabled" => Ok(false),
        other => Err(format!("期望 on 或 off，收到 `{other}`")),
    }
}

fn takeover(ctx: &Context, parsed: &Parsed) -> Result<Payload, String> {
    let enabled = on_off(parsed.require_word(2, "on|off")?)?;
    let app = if parsed.has_flag("app") {
        Some(parsed.app(2)?)
    } else {
        None
    };
    let apps: Vec<String> = match app {
        Some(app) => vec![app],
        None => ROUTE_MODE_APPS.iter().map(|app| app.to_string()).collect(),
    };

    let mut lines = Vec::new();
    let mut failures = Vec::new();
    for app in &apps {
        let result = call(
            ctx,
            "set_proxy_takeover_for_app",
            json!({ "appType": app, "enabled": enabled }),
        );
        match result {
            Ok(_) => lines.push(format!(
                "{app}: 已{}",
                if enabled { "接管" } else { "取消接管" }
            )),
            Err(error) => {
                lines.push(format!("{app}: 失败（{error}）"));
                failures.push(app.clone());
            }
        }
    }

    let mut text = lines.join("\n");
    text.push('\n');
    if failures.len() == apps.len() {
        return Err(format!(
            "{} 全部失败（{}）",
            if enabled { "接管" } else { "取消接管" },
            failures.join(", ")
        ));
    }
    if !ctx.quiet && !failures.is_empty() {
        eprintln!("ccswitch: 部分应用失败：{}", failures.join(", "));
    }
    Ok(Payload::Text(text))
}

/// `proxy config` 不带第二个词时按 get 处理。
fn config(ctx: &Context, parsed: &Parsed) -> Result<Payload, String> {
    match parsed.word(2).unwrap_or("get") {
        "get" | "show" => Ok(Payload::Value(call(ctx, "get_proxy_config", json!({}))?)),
        "set" => {
            let file = parsed.require_flag("file")?;
            let text =
                std::fs::read_to_string(file).map_err(|e| format!("读取 {file} 失败：{e}"))?;
            let config: Value =
                serde_json::from_str(&text).map_err(|e| format!("{file} 不是合法 JSON：{e}"))?;
            if !config.is_object() {
                return Err(format!(
                    "{file} 必须是完整的代理配置对象（ccswitch proxy config get > {file}）"
                ));
            }
            call(ctx, "update_proxy_config", json!({ "config": config }))?;
            Ok(Payload::Text("代理配置已保存\n".to_string()))
        }
        other => Err(format!(
            "未知子命令 `proxy config {other}`（ccswitch proxy --help）"
        )),
    }
}
