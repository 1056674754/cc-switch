//! `ccswitch profile ...`：配置档案（按分组快照供应商 / MCP / Skill / 提示词）。
//!
//! 档案本身没有「作用分组」字段，分组由快照里哪些槽位非空推断；所以 `apply` 不给
//! `--group` 时，只在快照恰好落在一个分组时自动选择。
//!
//! 分组用 `--scope`（也接受 `--group` / `--mode`）；「以当前状态更新快照」用
//! `--resnapshot`（也接受 `--force`）。

use chrono::TimeZone;
use serde_json::{json, Value};

use super::super::{Context, Parsed, Payload, Table};
use super::call;

const SCOPES: [&str; 3] = ["claude", "claude-desktop", "codex"];
const SLOTS: [&str; 4] = ["providers", "mcp", "skills", "prompts"];

pub fn run(ctx: &Context, parsed: &Parsed) -> Result<Payload, String> {
    match parsed.verb().unwrap_or("list") {
        "list" | "ls" => list(ctx),
        "create" | "add" => create(ctx, parsed),
        "update" | "edit" => update(ctx, parsed),
        "rm" | "remove" | "delete" => remove(ctx, parsed),
        "apply" | "use" => apply(ctx, parsed),
        "clear" => clear(ctx, parsed),
        other => Err(format!(
            "未知子命令 `profile {other}`（ccswitch profile --help）"
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

fn format_ts(ms: i64) -> String {
    let ms = if ms.abs() >= 1_000_000_000_000 {
        ms
    } else {
        ms.saturating_mul(1000)
    };
    chrono::Local
        .timestamp_millis_opt(ms)
        .single()
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_else(|| ms.to_string())
}

/// 分组选项：`--scope`（规范名）以及 `--group` / `--mode`（兼容写法）。
fn group_flag(parsed: &Parsed) -> Option<String> {
    parsed
        .flag("scope")
        .or_else(|| parsed.flag("group"))
        .or_else(|| parsed.flag("mode"))
        .map(|value| value.trim().to_lowercase())
}

fn read_json_file(path: &str) -> Result<Value, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("读取 {path} 失败：{e}"))?;
    serde_json::from_str(&text).map_err(|e| format!("{path} 不是合法 JSON：{e}"))
}

/// `--file` 只作为「参数文件」用：JSON 里可给 name / scope / id / resnapshot。
fn file_args(parsed: &Parsed) -> Result<Option<Value>, String> {
    match parsed.flag("file") {
        Some(file) => Ok(Some(read_json_file(file)?)),
        None => Ok(None),
    }
}

fn profiles(ctx: &Context) -> Result<(Vec<Value>, Value), String> {
    let response = call(ctx, "list_profiles", json!({}))?;
    let profiles = response
        .get("profiles")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let current = response.get("currentIds").cloned().unwrap_or(Value::Null);
    Ok((profiles, current))
}

/// 快照里非空的槽位对应的分组。
fn captured_scopes(profile: &Value) -> Vec<&'static str> {
    let payload = match profile.get("payload") {
        Some(payload) => payload,
        None => return Vec::new(),
    };
    let mut scopes = Vec::new();
    for scope in SCOPES {
        let captured = SLOTS.iter().any(|slot| {
            payload
                .get(*slot)
                .and_then(|per_app| per_app.get(scope))
                .map(|value| !value.is_null())
                .unwrap_or(false)
        });
        if captured {
            scopes.push(scope);
        }
    }
    scopes
}

fn scope_of_current(current: &Value, id: &str) -> Vec<&'static str> {
    let mut groups = Vec::new();
    for (key, label) in [
        ("claude", "claude"),
        ("claudeDesktop", "claude-desktop"),
        ("codex", "codex"),
    ] {
        if current.get(key).and_then(Value::as_str) == Some(id) {
            groups.push(label);
        }
    }
    groups
}

fn list(ctx: &Context) -> Result<Payload, String> {
    let (profiles, current) = profiles(ctx)?;
    let mut table = Table::new(&["id", "name", "groups", "current", "updated"]);
    for profile in &profiles {
        let id = field_str(profile, "id");
        let scopes = captured_scopes(profile);
        let current_groups = scope_of_current(&current, &id);
        let updated = profile
            .get("updatedAt")
            .and_then(Value::as_i64)
            .map(format_ts)
            .unwrap_or_else(|| "-".to_string());
        table.push(vec![
            id,
            field_str(profile, "name"),
            if scopes.is_empty() {
                "-".to_string()
            } else {
                scopes.join(",")
            },
            if current_groups.is_empty() {
                "-".to_string()
            } else {
                current_groups.join(",")
            },
            updated,
        ]);
    }
    Ok(Payload::Table(table))
}

fn create(ctx: &Context, parsed: &Parsed) -> Result<Payload, String> {
    let file = file_args(parsed)?;
    let name = parsed
        .flag("name")
        .map(str::to_string)
        .or_else(|| {
            file.as_ref()
                .and_then(|value| value.get("name"))
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .ok_or_else(|| {
            "缺少 --name（或 --file 的 JSON 里给 name）。注意 create_profile 没有快照入参，--file 只能提供参数"
                .to_string()
        })?;
    let scope = group_flag(parsed)
        .or_else(|| {
            file.as_ref()
                .and_then(|value| value.get("scope"))
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .unwrap_or_else(|| "claude".to_string());
    let created = call(
        ctx,
        "create_profile",
        json!({ "name": name, "scope": scope }),
    )?;
    Ok(Payload::Text(format!(
        "已创建配置档案 `{name}`（id: {}，分组: {scope}）\n",
        field_str(&created, "id")
    )))
}

fn update(ctx: &Context, parsed: &Parsed) -> Result<Payload, String> {
    let file = file_args(parsed)?;
    let id = match parsed.word(2) {
        Some(id) => id.to_string(),
        None => parsed
            .flag("id")
            .map(str::to_string)
            .or_else(|| {
                file.as_ref()
                    .and_then(|value| value.get("id"))
                    .and_then(Value::as_str)
                    .map(str::to_string)
            })
            .ok_or_else(|| "缺少参数 <id>（或用 --id/--file 给出）".to_string())?,
    };
    let name = parsed.flag("name").map(str::to_string).or_else(|| {
        file.as_ref()
            .and_then(|value| value.get("name"))
            .and_then(Value::as_str)
            .map(str::to_string)
    });
    let scope = group_flag(parsed).or_else(|| {
        file.as_ref()
            .and_then(|value| value.get("scope"))
            .and_then(Value::as_str)
            .map(str::to_string)
    });
    let resnapshot = parsed.has_flag("resnapshot")
        || parsed.has_flag("force")
        || file
            .as_ref()
            .and_then(|value| value.get("resnapshot"))
            .and_then(Value::as_bool)
            .unwrap_or(false);
    if name.is_none() && scope.is_none() && !resnapshot {
        return Err(
            "没有要更新的内容：可加 --name、--group <claude|claude-desktop|codex>，或用 --force 按当前状态重拍快照"
                .to_string(),
        );
    }
    let updated = call(
        ctx,
        "update_profile",
        json!({
            "id": id,
            "name": name,
            "scope": scope,
            "resnapshot": resnapshot,
        }),
    )?;
    Ok(Payload::Text(format!(
        "已更新配置档案 `{}`（id: {id}）\n",
        field_str(&updated, "name")
    )))
}

fn remove(ctx: &Context, parsed: &Parsed) -> Result<Payload, String> {
    let id = parsed.require_word(2, "id")?;
    call(ctx, "delete_profile", json!({ "id": id }))?;
    Ok(Payload::Text(format!("已删除配置档案 `{id}`\n")))
}

fn apply(ctx: &Context, parsed: &Parsed) -> Result<Payload, String> {
    let id = parsed.require_word(2, "id")?;
    let (profiles, current) = profiles(ctx)?;
    let profile = profiles
        .iter()
        .find(|profile| field_str(profile, "id") == id)
        .ok_or_else(|| format!("找不到配置档案 `{id}`（ccswitch profile list）"))?;

    let scope = match group_flag(parsed) {
        Some(scope) => scope,
        None => {
            let captured = captured_scopes(profile);
            match captured.len() {
                1 => captured[0].to_string(),
                0 => {
                    return Err(format!(
                        "档案 `{id}` 的快照里没有可应用的分组内容（ccswitch profile list）"
                    ))
                }
                _ => {
                    return Err(format!(
                        "档案 `{id}` 同时包含多个分组（{}），请用 --group <claude|claude-desktop|codex> 指定",
                        captured.join(",")
                    ))
                }
            }
        }
    };

    let warnings = call(ctx, "apply_profile", json!({ "id": id, "scope": scope }))?;
    if !ctx.quiet {
        if let Some(warnings) = warnings.as_array() {
            for warning in warnings.iter().filter_map(Value::as_str) {
                eprintln!("ccswitch: 提示：{warning}");
            }
        }
    }
    let name = field_str(profile, "name");
    let current_groups = scope_of_current(&current, id);
    let mut text = format!("已应用配置档案 `{name}`（id: {id}，分组: {scope}）\n");
    if !current_groups.is_empty() {
        text.push_str(&format!("当前使用中: {}\n", current_groups.join(",")));
    }
    Ok(Payload::Text(text))
}

fn clear(ctx: &Context, parsed: &Parsed) -> Result<Payload, String> {
    let scopes: Vec<String> = match group_flag(parsed) {
        Some(scope) => vec![scope],
        None => SCOPES.iter().map(|scope| scope.to_string()).collect(),
    };
    let mut lines = Vec::new();
    for scope in &scopes {
        call(ctx, "clear_current_profile", json!({ "scope": scope }))?;
        lines.push(format!("{scope}: 已清除当前档案"));
    }
    let mut text = lines.join("\n");
    text.push('\n');
    Ok(Payload::Text(text))
}
