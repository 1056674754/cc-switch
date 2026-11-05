//! `ccswitch prompt ...`：提示词（每个应用各自维护一份，启用项会写进 Live 配置）。
//!
//! `upsert --file` 的文件形状有两种：
//! 1. `{"app": "...", "id": "...", "prompt": {...}}`：完整参数都写在文件里；
//! 2. 直接给 `Prompt` 对象，用 `--app/--id` 指定归属。

use serde_json::{json, Value};

use super::super::{Context, Parsed, Payload, Table};
use super::call;

pub fn run(ctx: &Context, parsed: &Parsed) -> Result<Payload, String> {
    match parsed.verb().unwrap_or("list") {
        "list" | "ls" => list(ctx, parsed),
        "enable" => enable(ctx, parsed),
        "rm" | "remove" | "delete" => remove(ctx, parsed),
        "upsert" | "add" | "save" => upsert(ctx, parsed),
        other => Err(format!(
            "未知子命令 `prompt {other}`（ccswitch prompt --help）"
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

/// `--app` 不给时用 claude。
fn current_app(parsed: &Parsed) -> Result<String, String> {
    if parsed.has_flag("app") {
        parsed.app(2)
    } else {
        Ok("claude".to_string())
    }
}

fn app_and_id(parsed: &Parsed) -> Result<(String, String), String> {
    super::two_words(parsed, "app", "id")
}

fn read_json_file(path: &str) -> Result<Value, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("读取 {path} 失败：{e}"))?;
    serde_json::from_str(&text).map_err(|e| format!("{path} 不是合法 JSON：{e}"))
}

fn list(ctx: &Context, parsed: &Parsed) -> Result<Payload, String> {
    let app = current_app(parsed)?;
    let prompts = call(ctx, "get_prompts", json!({ "app": app }))?;
    let mut table = Table::new(&["id", "name", "enabled", "description"]);

    // 后端返回的是 `{ id: Prompt }`（IndexMap）；也兼容数组，防止命令实现变化。
    let mut push_prompt = |fallback_id: &str, prompt: &Value| {
        let id = field_str(prompt, "id");
        let description = field_str(prompt, "description");
        table.push(vec![
            if id.is_empty() {
                fallback_id.to_string()
            } else {
                id
            },
            field_str(prompt, "name"),
            if prompt.get("enabled").and_then(Value::as_bool) == Some(true) {
                "yes"
            } else {
                "no"
            }
            .to_string(),
            if description.is_empty() {
                "-".to_string()
            } else {
                description
            },
        ]);
    };

    match &prompts {
        Value::Object(prompts) => {
            for (id, prompt) in prompts {
                push_prompt(id, prompt);
            }
        }
        Value::Array(prompts) => {
            for prompt in prompts {
                push_prompt("", prompt);
            }
        }
        other => return Err(format!("get_prompts 返回了意外的形状：{other}")),
    }

    Ok(Payload::Table(table))
}

fn enable(ctx: &Context, parsed: &Parsed) -> Result<Payload, String> {
    let (app, id) = app_and_id(parsed)?;
    call(ctx, "enable_prompt", json!({ "app": app, "id": id }))?;
    Ok(Payload::Text(format!("已启用提示词 `{id}`（{app}）\n")))
}

fn remove(ctx: &Context, parsed: &Parsed) -> Result<Payload, String> {
    let (app, id) = app_and_id(parsed)?;
    call(ctx, "delete_prompt", json!({ "app": app, "id": id }))?;
    Ok(Payload::Text(format!("已删除提示词 `{id}`（{app}）\n")))
}

fn upsert(ctx: &Context, parsed: &Parsed) -> Result<Payload, String> {
    let file = parsed.require_flag("file")?;
    let value = read_json_file(file)?;
    if !value.is_object() {
        return Err(format!("{file} 必须是 JSON 对象"));
    }

    let (app, id, prompt) =
        if value.get("app").is_some() && value.get("id").is_some() && value.get("prompt").is_some()
        {
            (
                field_str(&value, "app"),
                field_str(&value, "id"),
                value.get("prompt").cloned().unwrap_or(Value::Null),
            )
        } else {
            let app = parsed
                .flag("app")
                .map(str::to_string)
                .or_else(|| value.get("app").and_then(Value::as_str).map(str::to_string))
                .ok_or_else(|| format!("{file} 缺少 app（或用 --app 指定）"))?;
            let id = parsed
                .flag("id")
                .map(str::to_string)
                .or_else(|| value.get("id").and_then(Value::as_str).map(str::to_string))
                .ok_or_else(|| format!("{file} 缺少 id（或用 --id 指定）"))?;
            let prompt = value
                .get("prompt")
                .cloned()
                .unwrap_or_else(|| value.clone());
            (app, id, prompt)
        };

    if app.is_empty() || id.is_empty() {
        return Err("app 和 id 都不能为空".to_string());
    }

    call(
        ctx,
        "upsert_prompt",
        json!({ "app": app, "id": id, "prompt": prompt }),
    )?;
    Ok(Payload::Text(format!("已保存提示词 `{id}`（{app}）\n")))
}
