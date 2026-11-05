//! `ccswitch provider ...`：供应商管理。

use serde_json::{json, Map, Value};

use super::super::{Context, Parsed, Payload, Table};
use super::call;

pub fn run(ctx: &Context, parsed: &Parsed) -> Result<Payload, String> {
    match parsed.verb().unwrap_or("list") {
        "list" => list(ctx, parsed),
        "current" => current(ctx, parsed),
        "use" | "switch" => switch(ctx, parsed),
        "show" | "get" => show(ctx, parsed),
        "add" => add(ctx, parsed),
        "update" | "edit" => update(ctx, parsed),
        "rm" | "remove" | "delete" => delete(ctx, parsed),
        "apply-live" => apply_live(ctx, parsed, true),
        "unapply-live" => apply_live(ctx, parsed, false),
        other => Err(format!(
            "未知子命令 `provider {other}`（ccswitch provider --help）"
        )),
    }
}

fn app(parsed: &Parsed, index: usize) -> Result<String, String> {
    parsed.app(index)
}

/// `get_providers` 返回 `{id: Provider}`。
fn providers(ctx: &Context, app: &str) -> Result<Map<String, Value>, String> {
    let value = call(ctx, "get_providers", json!({ "app": app }))?;
    match value {
        Value::Object(map) => Ok(map),
        other => Err(format!("get_providers 返回了意外的形状：{other}")),
    }
}

fn current_id(ctx: &Context, app: &str) -> Result<String, String> {
    call(ctx, "get_current_provider", json!({ "app": app }))
        .map(|value| value.as_str().unwrap_or_default().to_string())
}

fn list(ctx: &Context, parsed: &Parsed) -> Result<Payload, String> {
    let app = app(parsed, 2)?;
    let map = providers(ctx, &app)?;
    let current = current_id(ctx, &app).unwrap_or_default();

    let mut table = Table::new(&["id", "name", "category", "current"]);
    for (id, provider) in &map {
        let name = provider
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let category = provider
            .get("category")
            .and_then(Value::as_str)
            .unwrap_or("-");
        table.push(vec![
            id.clone(),
            name.to_string(),
            category.to_string(),
            if *id == current { "*" } else { "" }.to_string(),
        ]);
    }
    Ok(Payload::Table(table))
}

fn current(ctx: &Context, parsed: &Parsed) -> Result<Payload, String> {
    let app = app(parsed, 2)?;
    let id = current_id(ctx, &app)?;
    if id.is_empty() {
        return Ok(Payload::Text("（未设置当前供应商）\n".to_string()));
    }
    let name = providers(ctx, &app)
        .ok()
        .and_then(|map| {
            map.get(&id).and_then(|provider| {
                provider
                    .get("name")
                    .and_then(Value::as_str)
                    .map(str::to_string)
            })
        })
        .unwrap_or_else(|| id.clone());
    Ok(Payload::Text(format!("{name}  ({id})\n")))
}

fn switch(ctx: &Context, parsed: &Parsed) -> Result<Payload, String> {
    let app = app(parsed, 2)?;
    let wanted = parsed.require_word(3, "id|名称")?;
    let map = providers(ctx, &app)?;

    // 允许用名称切换：先按 id 精确匹配，再按名字唯一匹配。
    let id = if map.contains_key(wanted) {
        wanted.to_string()
    } else {
        let matches: Vec<&String> = map
            .iter()
            .filter(|(_, provider)| provider.get("name").and_then(Value::as_str) == Some(wanted))
            .map(|(id, _)| id)
            .collect();
        match matches.len() {
            0 => {
                return Err(format!(
                    "找不到供应商 `{wanted}`（先用 `ccswitch provider list --app {app}` 看看）"
                ))
            }
            1 => matches[0].clone(),
            _ => return Err(format!("名称 `{wanted}` 匹配到多个供应商，请改用 id 指定")),
        }
    };

    let result = call(ctx, "switch_provider", json!({ "app": app, "id": id }))?;
    if let Some(warnings) = result.get("warnings").and_then(Value::as_array) {
        for warning in warnings {
            if let Some(text) = warning.as_str() {
                eprintln!("ccswitch: 提示：{text}");
            }
        }
    }
    let name = map
        .get(&id)
        .and_then(|provider| provider.get("name").and_then(Value::as_str))
        .unwrap_or(&id);
    if let Some(route) = result.get("route") {
        if !route.is_null() {
            eprintln!("ccswitch: 路由状态已更新：{route}");
        }
    }
    Ok(Payload::Text(format!("已切换 {app} → {name} ({id})\n")))
}

fn show(ctx: &Context, parsed: &Parsed) -> Result<Payload, String> {
    let app = app(parsed, 2)?;
    let id = parsed.require_word(3, "id")?;
    let map = providers(ctx, &app)?;
    match map.get(id) {
        Some(provider) => Ok(Payload::Value(provider.clone())),
        None => Err(format!("找不到供应商 `{id}`")),
    }
}

fn read_provider_file(parsed: &Parsed) -> Result<Value, String> {
    let path = parsed
        .require_flag("file")
        .map_err(|_| "缺少 --file <provider.json>（可用 - 从 stdin 读）".to_string())?;
    let text = if path == "-" {
        use std::io::Read;
        let mut buffer = String::new();
        std::io::stdin()
            .read_to_string(&mut buffer)
            .map_err(|error| format!("读取 stdin 失败：{error}"))?;
        buffer
    } else {
        std::fs::read_to_string(path).map_err(|error| format!("读取 {path} 失败：{error}"))?
    };
    serde_json::from_str::<Value>(&text).map_err(|error| format!("{path} 不是合法 JSON：{error}"))
}

fn add(ctx: &Context, parsed: &Parsed) -> Result<Payload, String> {
    let app = app(parsed, 2)?;
    let provider = read_provider_file(parsed)?;
    let add_to_live = parsed
        .flag("add-live")
        .map(|value| value != "false" && value != "0")
        .unwrap_or(true);
    let data = call(
        ctx,
        "add_provider",
        json!({ "app": app, "provider": provider, "addToLive": add_to_live }),
    )?;
    Ok(Payload::Value(data))
}

fn update(ctx: &Context, parsed: &Parsed) -> Result<Payload, String> {
    let app = app(parsed, 2)?;
    let provider = read_provider_file(parsed)?;
    let mut args = json!({ "app": app, "provider": provider });
    if let Some(original) = parsed.flag("id") {
        args["originalId"] = Value::String(original.to_string());
    }
    let data = call(ctx, "update_provider", args)?;
    Ok(Payload::Value(data))
}

fn delete(ctx: &Context, parsed: &Parsed) -> Result<Payload, String> {
    let app = app(parsed, 2)?;
    let id = parsed.require_word(3, "id")?;
    call(ctx, "delete_provider", json!({ "app": app, "id": id }))?;
    Ok(Payload::Text(format!("已删除 {app} / {id}\n")))
}

fn apply_live(ctx: &Context, parsed: &Parsed, apply: bool) -> Result<Payload, String> {
    let app = app(parsed, 2)?;
    let id = parsed.require_word(3, "id")?;
    let command = if apply {
        "add_provider_to_live_config"
    } else {
        "remove_provider_from_live_config"
    };
    call(ctx, command, json!({ "app": app, "id": id }))?;
    Ok(Payload::Text(if apply {
        format!("已把 {app} / {id} 写入 live 配置\n")
    } else {
        format!("已把 {app} / {id} 从 live 配置移除\n")
    }))
}
