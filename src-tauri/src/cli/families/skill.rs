//! `ccswitch skill ...`：技能管理。
//!
//! 统一结构下所有 Skill 只有一份主副本，`apps` 决定同步到哪些应用；备份会保留原目录，
//! 卸载/恢复都走 `uninstall_skill_unified` / `restore_skill_backup`。

use chrono::TimeZone;
use serde_json::{json, Value};

use super::super::{Context, Parsed, Payload, Table};
use super::call;

pub fn run(ctx: &Context, parsed: &Parsed) -> Result<Payload, String> {
    match parsed.verb().unwrap_or("list") {
        "list" | "ls" => list(ctx),
        "backups" => backups(ctx),
        "install" => install(ctx, parsed),
        "uninstall" | "rm" | "remove" => uninstall(ctx, parsed),
        "restore" => restore(ctx, parsed),
        "enable" => toggle(ctx, parsed, true),
        "disable" => toggle(ctx, parsed, false),
        "scan" => scan(ctx),
        "resync" => resync(ctx),
        other => Err(format!(
            "未知子命令 `skill {other}`（ccswitch skill --help）"
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

fn array(value: &Value, command: &str) -> Result<Vec<Value>, String> {
    match value.as_array() {
        Some(items) => Ok(items.clone()),
        None => Err(format!("{command} 返回了意外的形状：{value}")),
    }
}

/// `--app` 不给时用 claude（旧的安装命令默认行为）。
fn current_app(parsed: &Parsed) -> Result<String, String> {
    if parsed.has_flag("app") {
        parsed.app(2)
    } else {
        Ok("claude".to_string())
    }
}

fn list(ctx: &Context) -> Result<Payload, String> {
    let skills = array(&call(ctx, "get_skills", json!({}))?, "get_skills")?;
    let mut table = Table::new(&["name", "id", "installed", "directory", "repo"]);
    for skill in &skills {
        let repo = match (
            skill.get("repoOwner").and_then(Value::as_str),
            skill.get("repoName").and_then(Value::as_str),
        ) {
            (Some(owner), Some(name)) => format!("{owner}/{name}"),
            _ => "-".to_string(),
        };
        table.push(vec![
            field_str(skill, "name"),
            field_str(skill, "key"),
            if skill.get("installed").and_then(Value::as_bool) == Some(true) {
                "yes"
            } else {
                "no"
            }
            .to_string(),
            field_str(skill, "directory"),
            repo,
        ]);
    }
    Ok(Payload::Table(table))
}

fn backups(ctx: &Context) -> Result<Payload, String> {
    let entries = array(
        &call(ctx, "get_skill_backups", json!({}))?,
        "get_skill_backups",
    )?;
    let mut table = Table::new(&["backupId", "name", "createdAt", "path"]);
    for entry in &entries {
        let created_at = entry.get("createdAt").and_then(Value::as_i64).unwrap_or(0);
        let name = entry
            .get("skill")
            .map(|skill| field_str(skill, "name"))
            .unwrap_or_default();
        table.push(vec![
            field_str(entry, "backupId"),
            name,
            format_ts(created_at),
            field_str(entry, "backupPath"),
        ]);
    }
    Ok(Payload::Table(table))
}

fn install(ctx: &Context, parsed: &Parsed) -> Result<Payload, String> {
    let app = current_app(parsed)?;

    if let Some(repo) = parsed.flag("repo") {
        let (owner, name) = repo
            .split_once('/')
            .ok_or_else(|| format!("--repo 需要 owner/name 形式，收到 `{repo}`"))?;
        let discovered = call(
            ctx,
            "discover_repo_skills",
            json!({ "owner": owner, "name": name }),
        )?;
        let skills = discovered
            .get("skills")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        if skills.is_empty() {
            return Err(format!("仓库 `{repo}` 里没有发现可安装的 Skill"));
        }
        let mut installed = Vec::new();
        for skill in skills {
            let installed_skill = call(
                ctx,
                "install_skill_unified",
                json!({ "skill": skill, "currentApp": app }),
            )?;
            let name = installed_skill
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            installed.push(name);
        }
        let mut text = format!(
            "已从仓库 `{repo}` 安装 {} 个 Skill（{app}）：\n",
            installed.len()
        );
        for name in installed {
            text.push_str(&format!("  - {name}\n"));
        }
        return Ok(Payload::Text(text));
    }

    let source = parsed.require_word(2, "source|目录名")?;
    call(
        ctx,
        "install_skill_for_app",
        json!({ "app": app, "directory": source }),
    )?;
    Ok(Payload::Text(format!("已安装 Skill `{source}`（{app}）\n")))
}

fn uninstall(ctx: &Context, parsed: &Parsed) -> Result<Payload, String> {
    let id = parsed.require_word(2, "id")?;
    let result = call(ctx, "uninstall_skill_unified", json!({ "id": id }))?;
    let mut text = format!("已卸载 Skill `{id}`\n");
    if let Some(path) = result.get("backupPath").and_then(Value::as_str) {
        text.push_str(&format!("备份目录: {path}\n"));
    }
    if result.get("piCleanupIncomplete").and_then(Value::as_bool) == Some(true) {
        text.push_str("提示：Pi 目录没有清理干净，可用 `ccswitch skill backups` 查备份\n");
    }
    Ok(Payload::Text(text))
}

fn restore(ctx: &Context, parsed: &Parsed) -> Result<Payload, String> {
    let backup_id = parsed.require_word(2, "backupId")?;
    let app = current_app(parsed)?;
    let skill = call(
        ctx,
        "restore_skill_backup",
        json!({ "backupId": backup_id, "currentApp": app }),
    )?;
    Ok(Payload::Text(format!(
        "已从备份 `{backup_id}` 恢复 Skill `{}`（{app}）\n",
        field_str(&skill, "name")
    )))
}

fn toggle(ctx: &Context, parsed: &Parsed, enabled: bool) -> Result<Payload, String> {
    let id = parsed.require_word(2, "id")?;
    let app = parsed.require_flag("app")?;
    call(
        ctx,
        "toggle_skill_app",
        json!({ "id": id, "app": app, "enabled": enabled }),
    )?;
    let action = if enabled { "启用" } else { "停用" };
    Ok(Payload::Text(format!("已{action} Skill `{id}`（{app}）\n")))
}

fn scan(ctx: &Context) -> Result<Payload, String> {
    let skills = array(
        &call(ctx, "scan_unmanaged_skills", json!({}))?,
        "scan_unmanaged_skills",
    )?;
    let mut table = Table::new(&["name", "directory", "foundIn", "path"]);
    for skill in &skills {
        let found_in: Vec<String> = skill
            .get("foundIn")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default();
        table.push(vec![
            field_str(skill, "name"),
            field_str(skill, "directory"),
            if found_in.is_empty() {
                "-".to_string()
            } else {
                found_in.join(",")
            },
            field_str(skill, "path"),
        ]);
    }
    Ok(Payload::Table(table))
}

fn resync(ctx: &Context) -> Result<Payload, String> {
    let outcomes = array(
        &call(ctx, "resync_skills_to_apps", json!({}))?,
        "resync_skills_to_apps",
    )?;
    let mut table = Table::new(&["app", "ok", "detail"]);
    for outcome in &outcomes {
        let ok = outcome.get("ok").and_then(Value::as_bool).unwrap_or(false);
        let detail = outcome
            .get("error")
            .and_then(Value::as_str)
            .map(str::to_string)
            .or_else(|| {
                let failed = outcome
                    .get("failedSkills")
                    .and_then(Value::as_array)
                    .map(Vec::len)
                    .unwrap_or(0);
                if failed > 0 {
                    Some(format!("{failed} 个 Skill 同步失败"))
                } else {
                    None
                }
            })
            .unwrap_or_else(|| "-".to_string());
        table.push(vec![
            field_str(outcome, "app"),
            if ok { "yes" } else { "no" }.to_string(),
            detail,
        ]);
    }
    Ok(Payload::Table(table))
}
