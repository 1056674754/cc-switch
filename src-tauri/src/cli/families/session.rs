//! `ccswitch session ...`：会话列表、消息与删除。
//!
//! 会话 id 只在单个来源里唯一，命令面又要求 `get_session_messages`/`delete_session`
//! 同时给 `providerId` + `sourcePath`，所以这里先用 `list_sessions` 把 id 解析成
//! 完整三元组；遇到多个来源同 id 时用 `--provider` 指定。

use serde_json::{json, Value};

use super::super::{Context, Parsed, Payload, Table};
use super::call;

struct SessionRef {
    provider_id: String,
    session_id: String,
    source_path: String,
}

pub fn run(ctx: &Context, parsed: &Parsed) -> Result<Payload, String> {
    match parsed.verb().unwrap_or("list") {
        "list" | "ls" => list(ctx),
        "messages" | "msg" => messages(ctx, parsed),
        "rm" | "remove" | "delete" => remove(ctx, parsed),
        other => Err(format!(
            "未知子命令 `session {other}`（ccswitch session --help）"
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

fn format_ts(ts: i64) -> String {
    let ms = if ts.abs() >= 1_000_000_000_000 {
        ts
    } else {
        ts.saturating_mul(1000)
    };
    use chrono::TimeZone;
    chrono::Local
        .timestamp_millis_opt(ms)
        .single()
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_else(|| ms.to_string())
}

fn truncate(text: &str, limit: usize) -> String {
    let first_line = text.lines().next().unwrap_or_default().trim();
    if first_line.chars().count() <= limit {
        first_line.to_string()
    } else {
        let mut out: String = first_line.chars().take(limit).collect();
        out.push('…');
        out
    }
}

/// 结构化内容（blocks）里没有 `content` 时，把各段的 text 拼起来做预览。
fn collect_text(value: &Value, out: &mut String) {
    match value {
        Value::Object(map) => {
            if let Some(text) = map.get("text").and_then(Value::as_str) {
                if !out.is_empty() {
                    out.push(' ');
                }
                out.push_str(text);
            }
            for nested in map.values() {
                collect_text(nested, out);
            }
        }
        Value::Array(items) => {
            for nested in items {
                collect_text(nested, out);
            }
        }
        _ => {}
    }
}

fn sessions(ctx: &Context) -> Result<Vec<Value>, String> {
    let value = call(ctx, "list_sessions", json!({}))?;
    match value.as_array() {
        Some(items) => Ok(items.clone()),
        None => Err(format!("list_sessions 返回了意外的形状：{value}")),
    }
}

fn list(ctx: &Context) -> Result<Payload, String> {
    let sessions = sessions(ctx)?;
    let mut table = Table::new(&["provider", "id", "title", "lastActive", "project"]);
    for session in &sessions {
        let title = match session.get("title").and_then(Value::as_str) {
            Some(title) if !title.is_empty() => truncate(title, 40),
            _ => match session.get("summary").and_then(Value::as_str) {
                Some(summary) if !summary.is_empty() => truncate(summary, 40),
                _ => "-".to_string(),
            },
        };
        let last_active = session
            .get("lastActiveAt")
            .and_then(Value::as_i64)
            .or_else(|| session.get("createdAt").and_then(Value::as_i64))
            .map(format_ts)
            .unwrap_or_else(|| "-".to_string());
        let project = match session.get("projectDir").and_then(Value::as_str) {
            Some(dir) if !dir.is_empty() => std::path::Path::new(dir)
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_else(|| dir.to_string()),
            _ => "-".to_string(),
        };
        table.push(vec![
            field_str(session, "providerId"),
            field_str(session, "sessionId"),
            title,
            last_active,
            project,
        ]);
    }
    Ok(Payload::Table(table))
}

fn resolve(ctx: &Context, parsed: &Parsed) -> Result<SessionRef, String> {
    let wanted = parsed.require_word(2, "id")?;
    let provider_filter = parsed.flag("provider");
    let sessions = sessions(ctx)?;

    let matches = |session: &Value, prefix: bool| -> bool {
        let id = field_str(session, "sessionId");
        let provider = field_str(session, "providerId");
        let id_matches = if prefix {
            !id.is_empty() && id.starts_with(wanted)
        } else {
            id == wanted
        };
        id_matches && provider_filter.is_none_or(|wanted| wanted == provider)
    };

    let mut candidates: Vec<&Value> = sessions.iter().filter(|s| matches(s, false)).collect();
    if candidates.is_empty() {
        candidates = sessions.iter().filter(|s| matches(s, true)).collect();
    }

    match candidates.len() {
        0 => Err(format!(
            "找不到会话 `{wanted}`（ccswitch session list{}）",
            if provider_filter.is_some() {
                "；--provider 可能过滤过头了"
            } else {
                ""
            }
        )),
        1 => {
            let session = candidates[0];
            let source_path = field_str(session, "sourcePath");
            if source_path.is_empty() {
                return Err(format!(
                    "会话 `{wanted}` 没有 sourcePath，当前后端不支持读取/删除它"
                ));
            }
            Ok(SessionRef {
                provider_id: field_str(session, "providerId"),
                session_id: field_str(session, "sessionId"),
                source_path,
            })
        }
        _ => Err(format!(
            "会话 `{wanted}` 在 {} 个来源里都存在，请用 --provider <应用> 指定",
            candidates.len()
        )),
    }
}

fn messages(ctx: &Context, parsed: &Parsed) -> Result<Payload, String> {
    let session = resolve(ctx, parsed)?;
    let messages = call(
        ctx,
        "get_session_messages",
        json!({
            "providerId": session.provider_id,
            "sourcePath": session.source_path,
        }),
    )?;
    let messages = match messages.as_array() {
        Some(messages) => messages.clone(),
        None => return Err(format!("get_session_messages 返回了意外的形状：{messages}")),
    };

    let mut table = Table::new(&["role", "time", "content"]);
    for message in &messages {
        let content = field_str(message, "content");
        let content = if content.trim().is_empty() {
            let mut text = String::new();
            collect_text(message.get("blocks").unwrap_or(&Value::Null), &mut text);
            if text.trim().is_empty() {
                "(结构化内容)".to_string()
            } else {
                truncate(&text, 80)
            }
        } else {
            truncate(&content, 80)
        };
        let time = message
            .get("ts")
            .and_then(Value::as_i64)
            .map(format_ts)
            .unwrap_or_else(|| "-".to_string());
        table.push(vec![field_str(message, "role"), time, content]);
    }
    Ok(Payload::Table(table))
}

fn remove(ctx: &Context, parsed: &Parsed) -> Result<Payload, String> {
    let session = resolve(ctx, parsed)?;
    let deleted = call(
        ctx,
        "delete_session",
        json!({
            "providerId": session.provider_id,
            "sessionId": session.session_id,
            "sourcePath": session.source_path,
        }),
    )?;
    if deleted.as_bool() == Some(false) {
        return Err(format!(
            "后端没能删除会话 `{}`（可能文件已不存在）",
            session.session_id
        ));
    }
    Ok(Payload::Text(format!(
        "已删除会话 `{}`（{}）\n",
        session.session_id, session.provider_id
    )))
}
