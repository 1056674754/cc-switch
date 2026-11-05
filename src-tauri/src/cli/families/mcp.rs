//! `ccswitch mcp ...`：MCP 服务器管理。
//!
//! 命令名与参数 key 对应 `src/ipc/adapters/mcp.rs`；「统一结构」以数据库为准，
//! 各应用是否启用存在 `apps` 对象里，所以列表能直接看出启用了哪些应用。

use serde_json::{json, Value};

use super::super::{Context, Parsed, Payload, Table};
use super::call;

pub fn run(ctx: &Context, parsed: &Parsed) -> Result<Payload, String> {
    match parsed.verb().unwrap_or("list") {
        "list" => list(ctx, parsed),
        "show" | "get" => show(ctx, parsed),
        "upsert" | "add" => upsert(ctx, parsed),
        "rm" | "remove" | "delete" => remove(ctx, parsed),
        "enable" => toggle(ctx, parsed, true),
        "disable" => toggle(ctx, parsed, false),
        "import" => import(ctx, parsed),
        "resync" => resync(ctx, parsed),
        other => Err(format!("未知子命令 `mcp {other}`（ccswitch mcp --help）")),
    }
}

fn all_servers(ctx: &Context) -> Result<serde_json::Map<String, Value>, String> {
    let value = call(ctx, "get_mcp_servers", json!({}))?;
    match value {
        Value::Object(map) => Ok(map),
        other => Err(format!("get_mcp_servers 返回了意外的形状：{other}")),
    }
}

/// `--app` 可选：给了就按该应用的启用状态过滤。
fn optional_app(parsed: &Parsed) -> Result<Option<String>, String> {
    if parsed.has_flag("app") {
        parsed.app(2).map(Some)
    } else {
        Ok(None)
    }
}

fn field_str(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

/// `apps` 对象里为 true 的应用名，逗号分隔。
fn enabled_apps(apps: Option<&Value>) -> String {
    let map = match apps {
        Some(Value::Object(map)) => map,
        _ => return "-".to_string(),
    };
    let names: Vec<&str> = map
        .iter()
        .filter(|(_, enabled)| enabled.as_bool().unwrap_or(false))
        .map(|(name, _)| name.as_str())
        .collect();
    if names.is_empty() {
        "-".to_string()
    } else {
        names.join(",")
    }
}

/// 从 `server` 字段猜传输方式（stdio / http / 自定义 type）。
fn transport(server: Option<&Value>) -> String {
    let server = match server {
        Some(server) => server,
        None => return "-".to_string(),
    };
    if server.get("command").is_some() {
        return "stdio".to_string();
    }
    if let Some(kind) = server.get("type").and_then(Value::as_str) {
        if !kind.is_empty() {
            return kind.to_string();
        }
    }
    if server.get("url").is_some() {
        return "http".to_string();
    }
    "-".to_string()
}

fn list(ctx: &Context, parsed: &Parsed) -> Result<Payload, String> {
    let app = optional_app(parsed)?;
    let servers = all_servers(ctx)?;
    let mut table = Table::new(&["id", "name", "apps", "transport"]);
    for (id, server) in &servers {
        if let Some(app) = &app {
            let enabled = server
                .get("apps")
                .and_then(|apps| apps.get(app.as_str()))
                .and_then(Value::as_bool)
                .unwrap_or(false);
            if !enabled {
                continue;
            }
        }
        table.push(vec![
            id.clone(),
            field_str(server, "name"),
            enabled_apps(server.get("apps")),
            transport(server.get("server")),
        ]);
    }
    Ok(Payload::Table(table))
}

fn show(ctx: &Context, parsed: &Parsed) -> Result<Payload, String> {
    let id = parsed.require_word(2, "id")?;
    let servers = all_servers(ctx)?;
    match servers.get(id) {
        Some(server) => Ok(Payload::Value(server.clone())),
        None => Err(format!("找不到 MCP 服务器 `{id}`（ccswitch mcp list）")),
    }
}

fn read_json_file(path: &str) -> Result<Value, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("读取 {path} 失败：{e}"))?;
    serde_json::from_str(&text).map_err(|e| format!("{path} 不是合法 JSON：{e}"))
}

/// 支持两种文件形状：
/// 1. 完整的 `McpServer` JSON（含 id/name/server/apps）→ `upsert_mcp_server`；
/// 2. `{app, id, spec}`（或配合 `--app/--id` 直接给 spec）→ `upsert_mcp_server_in_config`。
fn upsert(ctx: &Context, parsed: &Parsed) -> Result<Payload, String> {
    let file = parsed.require_flag("file")?;
    let value = read_json_file(file)?;
    let object = match value.as_object() {
        Some(object) => object,
        None => return Err(format!("{file} 必须是 JSON 对象")),
    };

    if object.get("server").is_some() && object.get("id").is_some() {
        let id = field_str(&value, "id");
        let name = field_str(&value, "name");
        call(ctx, "upsert_mcp_server", json!({ "server": value }))?;
        return Ok(Payload::Text(format!(
            "已保存 MCP 服务器 `{name}`（id: {id}）\n"
        )));
    }

    let app = object
        .get("app")
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| parsed.flag("app").map(str::to_string))
        .ok_or_else(|| {
            "--file 里的 JSON 缺少 `app`（可直接给完整 McpServer；也可加 --app）".to_string()
        })?;
    let id = object
        .get("id")
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| parsed.flag("id").map(str::to_string))
        .ok_or_else(|| "--file 里的 JSON 缺少 `id`（也可加 --id）".to_string())?;
    let spec = object.get("spec").cloned().unwrap_or_else(|| value.clone());

    call(
        ctx,
        "upsert_mcp_server_in_config",
        json!({ "app": app, "id": id, "spec": spec }),
    )?;
    Ok(Payload::Text(format!(
        "已在应用 `{app}` 的 MCP 配置里写入 `{id}`\n"
    )))
}

fn remove(ctx: &Context, parsed: &Parsed) -> Result<Payload, String> {
    let id = parsed.require_word(2, "id")?;
    let removed = call(ctx, "delete_mcp_server", json!({ "id": id }))?;
    if removed.as_bool() == Some(false) {
        return Err(format!("找不到 MCP 服务器 `{id}`（ccswitch mcp list）"));
    }
    Ok(Payload::Text(format!("已删除 MCP 服务器 `{id}`\n")))
}

fn toggle(ctx: &Context, parsed: &Parsed, enabled: bool) -> Result<Payload, String> {
    let id = parsed.require_word(2, "id")?;
    let app = parsed.require_flag("app")?;
    call(
        ctx,
        "toggle_mcp_app",
        json!({ "serverId": id, "app": app, "enabled": enabled }),
    )?;
    let action = if enabled { "启用" } else { "停用" };
    Ok(Payload::Text(format!(
        "已{action} MCP 服务器 `{id}`（{app}）\n"
    )))
}

fn import(ctx: &Context, parsed: &Parsed) -> Result<Payload, String> {
    if parsed.has_flag("app") && !ctx.quiet {
        eprintln!("ccswitch: import_mcp_from_apps 不支持按应用过滤，本次导入全部应用");
    }
    let count = call(ctx, "import_mcp_from_apps", json!({}))?;
    Ok(Payload::Text(format!(
        "已从应用配置导入 {} 个 MCP 服务器\n",
        count.as_u64().unwrap_or(0)
    )))
}

fn resync(ctx: &Context, parsed: &Parsed) -> Result<Payload, String> {
    let apps = optional_app(parsed)?;
    let args = match &apps {
        Some(app) => json!({ "apps": [app] }),
        None => json!({ "apps": null }),
    };
    let outcomes = call(ctx, "resync_mcp_to_apps", args)?;
    let outcomes = match outcomes.as_array() {
        Some(outcomes) => outcomes,
        None => return Ok(Payload::Value(outcomes)),
    };
    let mut table = Table::new(&["app", "ok", "error"]);
    for outcome in outcomes {
        let ok = outcome.get("ok").and_then(Value::as_bool).unwrap_or(false);
        let error = outcome
            .get("error")
            .and_then(Value::as_str)
            .unwrap_or("-")
            .to_string();
        table.push(vec![
            field_str(outcome, "app"),
            if ok { "yes" } else { "no" }.to_string(),
            error,
        ]);
    }
    Ok(Payload::Table(table))
}
