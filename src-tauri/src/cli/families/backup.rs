//! `ccswitch backup ...`：数据库备份（列表 / 创建 / 恢复 / 改名 / 删除）。
//!
//! 备份文件由桌面版维护在数据库同目录；`filename` 用列表里的名字，不要拼路径。

use serde_json::{json, Value};

use super::super::{Context, Parsed, Payload, Table};
use super::call;

pub fn run(ctx: &Context, parsed: &Parsed) -> Result<Payload, String> {
    match parsed.verb().unwrap_or("list") {
        "list" | "ls" => list(ctx),
        "create" | "add" => create(ctx),
        "restore" => restore(ctx, parsed),
        "rename" => rename(ctx, parsed),
        "rm" | "remove" | "delete" => remove(ctx, parsed),
        other => Err(format!(
            "未知子命令 `backup {other}`（ccswitch backup --help）"
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

fn human_size(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KB", "MB", "GB"];
    let mut size = bytes as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit + 1 < UNITS.len() {
        size /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{size:.1} {}", UNITS[unit])
    }
}

fn list(ctx: &Context) -> Result<Payload, String> {
    let value = call(ctx, "list_db_backups", json!({}))?;
    let entries = match value.as_array() {
        Some(entries) => entries.clone(),
        None => return Err(format!("list_db_backups 返回了意外的形状：{value}")),
    };
    let mut table = Table::new(&["filename", "size", "createdAt"]);
    for entry in &entries {
        let size = entry.get("sizeBytes").and_then(Value::as_u64).unwrap_or(0);
        table.push(vec![
            field_str(entry, "filename"),
            human_size(size),
            field_str(entry, "createdAt"),
        ]);
    }
    Ok(Payload::Table(table))
}

fn create(ctx: &Context) -> Result<Payload, String> {
    let filename = call(ctx, "create_db_backup", json!({}))?;
    let name = filename.as_str().unwrap_or_default();
    Ok(Payload::Text(format!("已创建备份：{name}\n")))
}

fn restore(ctx: &Context, parsed: &Parsed) -> Result<Payload, String> {
    let filename = parsed.require_word(2, "filename")?;
    let message = call(ctx, "restore_db_backup", json!({ "filename": filename }))?;
    match message.as_str() {
        Some(message) if !message.is_empty() => Ok(Payload::Text(format!("{message}\n"))),
        _ => Ok(Payload::Text(format!("已从备份 `{filename}` 恢复数据库\n"))),
    }
}

fn rename(ctx: &Context, parsed: &Parsed) -> Result<Payload, String> {
    let old = parsed.require_word(2, "oldFilename")?;
    let new = parsed.require_word(3, "newName")?;
    let message = call(
        ctx,
        "rename_db_backup",
        json!({ "oldFilename": old, "newName": new }),
    )?;
    match message.as_str() {
        Some(message) if !message.is_empty() => Ok(Payload::Text(format!("{message}\n"))),
        _ => Ok(Payload::Text(format!("已把备份 `{old}` 改名为 `{new}`\n"))),
    }
}

fn remove(ctx: &Context, parsed: &Parsed) -> Result<Payload, String> {
    let filename = parsed.require_word(2, "filename")?;
    call(ctx, "delete_db_backup", json!({ "filename": filename }))?;
    Ok(Payload::Text(format!("已删除备份 `{filename}`\n")))
}
