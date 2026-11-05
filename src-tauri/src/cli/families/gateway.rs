//! `ccswitch gateway ...`：网关聚合（目录、各应用的模型配置、接管/释放）。
//!
//! 「接管」把应用的客户端指向网关：claude/codex/gemini/grokbuild/claude-desktop 走
//! 通用的 `adopt_app_to_gateway`（进入路由模式），其余聚合应用各有专用命令。
//! `adopt --file` 先写该应用的模型配置（`set_app_model_config`）再接管。

use serde_json::{json, Value};

use super::super::{Context, Parsed, Payload, Table};
use super::call;

/// 有直连 / 路由模式的应用，用通用的 `adopt_app_to_gateway`。
const ROUTE_MODE_APPS: [&str; 5] = ["claude", "codex", "gemini", "grokbuild", "claude-desktop"];

fn adopt_command(app: &str) -> Option<(&'static str, bool)> {
    Some(match app {
        "opencode" => ("adopt_opencode_to_gateway", false),
        "hermes" => ("adopt_hermes_to_gateway", false),
        "openclaw" => ("adopt_openclaw_to_gateway", false),
        "pi" => ("adopt_pi_to_gateway", false),
        "omp" => ("adopt_omp_to_gateway", false),
        "dsh" => ("adopt_dsh_to_gateway", false),
        app if ROUTE_MODE_APPS.contains(&app) => ("adopt_app_to_gateway", true),
        _ => return None,
    })
}

fn release_command(app: &str) -> Option<(&'static str, bool)> {
    Some(match app {
        "opencode" => ("release_opencode_from_gateway", false),
        "hermes" => ("release_hermes_from_gateway", false),
        "openclaw" => ("release_openclaw_from_gateway", false),
        "pi" => ("release_pi_from_gateway", false),
        "omp" => ("release_omp_from_gateway", false),
        "dsh" => ("release_dsh_from_gateway", false),
        app if ROUTE_MODE_APPS.contains(&app) => ("release_app_from_gateway", true),
        _ => return None,
    })
}

pub fn run(ctx: &Context, parsed: &Parsed) -> Result<Payload, String> {
    match parsed.verb().unwrap_or("catalog") {
        "catalog" => catalog(ctx),
        "app-model" => app_model(ctx, parsed),
        "adopt" => adopt(ctx, parsed),
        "release" => release(ctx, parsed),
        other => Err(format!(
            "未知子命令 `gateway {other}`（ccswitch gateway --help）"
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

fn join_strings(value: Option<&Value>) -> String {
    let items = match value.and_then(Value::as_array) {
        Some(items) => items,
        None => return "-".to_string(),
    };
    let names: Vec<&str> = items.iter().filter_map(Value::as_str).collect();
    if names.is_empty() {
        "-".to_string()
    } else {
        names.join(",")
    }
}

fn catalog(ctx: &Context) -> Result<Payload, String> {
    let entries = call(ctx, "get_gateway_catalog", json!({}))?;
    let entries = match entries.as_array() {
        Some(entries) => entries.clone(),
        None => return Err(format!("get_gateway_catalog 返回了意外的形状：{entries}")),
    };
    let mut table = Table::new(&[
        "reference",
        "upstream",
        "model",
        "display",
        "1M",
        "window",
        "endpoints",
    ]);
    for entry in &entries {
        let upstream = match entry.get("upstreamName").and_then(Value::as_str) {
            Some(name) if !name.is_empty() => name.to_string(),
            _ => field_str(entry, "upstreamId"),
        };
        let display = field_str(entry, "displayName");
        let window = entry
            .get("window")
            .map(|value| value.to_string())
            .filter(|value| value != "null")
            .unwrap_or_else(|| "-".to_string());
        table.push(vec![
            field_str(entry, "reference"),
            upstream,
            field_str(entry, "modelId"),
            if display.is_empty() {
                "-".to_string()
            } else {
                display
            },
            if entry.get("oneM").and_then(Value::as_bool) == Some(true) {
                "yes"
            } else {
                ""
            }
            .to_string(),
            window,
            join_strings(entry.get("endpointIds")),
        ]);
    }
    Ok(Payload::Table(table))
}

fn app_model(ctx: &Context, parsed: &Parsed) -> Result<Payload, String> {
    let app = parsed.app(2)?;
    let value = call(ctx, "get_app_model_config", json!({ "appType": app }))?;
    if value.is_null() {
        return Ok(Payload::Text(format!("（{app} 还没有网关模型配置）\n")));
    }
    Ok(Payload::Value(value))
}

fn adopt(ctx: &Context, parsed: &Parsed) -> Result<Payload, String> {
    let app = parsed.app(2)?;
    let command = adopt_command(&app).ok_or_else(|| {
        format!("{app} 没有网关接管命令（可用应用：opencode/hermes/openclaw/pi/omp/dsh/claude/codex/gemini/grokbuild/claude-desktop）")
    })?;

    if let Some(file) = parsed.flag("file") {
        let text = std::fs::read_to_string(file).map_err(|e| format!("读取 {file} 失败：{e}"))?;
        let config: Value =
            serde_json::from_str(&text).map_err(|e| format!("{file} 不是合法 JSON：{e}"))?;
        call(
            ctx,
            "set_app_model_config",
            json!({ "appType": app, "config": config }),
        )?;
    }

    if command.1 {
        call(ctx, command.0, json!({ "appType": app }))?;
    } else {
        call(ctx, command.0, json!({}))?;
    }
    Ok(Payload::Text(format!(
        "已把 {app} 指向网关（ccswitch gateway release {app} 可撤销）\n"
    )))
}

fn release(ctx: &Context, parsed: &Parsed) -> Result<Payload, String> {
    let app = parsed.app(2)?;
    let command = release_command(&app).ok_or_else(|| {
        format!("{app} 没有网关释放命令（可用应用：opencode/hermes/openclaw/pi/omp/dsh/claude/codex/gemini/grokbuild/claude-desktop）")
    })?;
    if command.1 {
        call(ctx, command.0, json!({ "appType": app }))?;
    } else {
        call(ctx, command.0, json!({}))?;
    }
    Ok(Payload::Text(format!("已释放 {app} 的网关接管\n")))
}
