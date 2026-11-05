//! `ccswitch settings ...`：应用设置（设备级，存 `settings.json`）。
//!
//! 设置对象整体读写：`set --file` 传的是完整设置 JSON（`settings get` 导出的形状），
//! 后端会与现有设置做合并，缺字段不会把已有值清空。

use serde_json::{json, Value};

use super::super::{Context, Parsed, Payload};
use super::call;

pub fn run(ctx: &Context, parsed: &Parsed) -> Result<Payload, String> {
    match parsed.verb().unwrap_or("get") {
        "get" | "show" => get(ctx),
        "set" | "save" | "update" => set(ctx, parsed),
        "path" | "paths" => path(ctx),
        other => Err(format!(
            "未知子命令 `settings {other}`（ccswitch settings --help）"
        )),
    }
}

fn get(ctx: &Context) -> Result<Payload, String> {
    Ok(Payload::Value(call(ctx, "get_settings", json!({}))?))
}

fn read_json_file(path: &str) -> Result<Value, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("读取 {path} 失败：{e}"))?;
    serde_json::from_str(&text).map_err(|e| format!("{path} 不是合法 JSON：{e}"))
}

fn set(ctx: &Context, parsed: &Parsed) -> Result<Payload, String> {
    let file = parsed.require_flag("file")?;
    let settings = read_json_file(file)?;
    if !settings.is_object() {
        return Err(format!(
            "{file} 必须是完整的设置 JSON 对象（可先 `ccswitch settings get > {file}`）"
        ));
    }
    call(ctx, "save_settings", json!({ "settings": settings }))?;
    Ok(Payload::Text("设置已保存\n".to_string()))
}

/// 打印配置目录与设置文件路径。覆盖目录以桌面版正在用的为准（`get_app_config_dir_override`），
/// 桌面版没跑时退回本地读取的同一份配置。
fn path(ctx: &Context) -> Result<Payload, String> {
    let override_dir = call(ctx, "get_app_config_dir_override", json!({}))?;
    let dir = match override_dir.as_str() {
        Some(dir) => std::path::PathBuf::from(dir),
        None => crate::config::get_app_config_dir(),
    };
    let settings = dir.join("settings.json");
    let config = dir.join("config.json");
    Ok(Payload::Text(format!(
        "配置目录: {}\n设置文件: {}\n其他配置: {}\n",
        dir.display(),
        settings.display(),
        config.display()
    )))
}
