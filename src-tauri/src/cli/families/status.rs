//! `ccswitch status`：一眼看清当前状态。

use serde_json::{json, Value};

use super::super::{Context, Parsed, Payload};
use super::call;

pub fn run(ctx: &Context, _parsed: &Parsed) -> Result<Payload, String> {
    let mut lines = String::new();
    lines.push_str("CC Switch\n");
    lines.push_str(&format!("通道: {}\n", ctx.client.describe()));
    if let Some(health) = ctx.client.health() {
        if let Some(pid) = health.get("pid").and_then(Value::as_u64) {
            lines.push_str(&format!("桌面进程: pid={pid}\n"));
        }
        if let Some(commands) = health.get("commands").and_then(Value::as_u64) {
            lines.push_str(&format!("可调命令: {commands}\n"));
        }
    }

    let proxy_running = call(ctx, "is_proxy_running", json!({}))
        .ok()
        .and_then(|value| value.as_bool());
    let takeover = call(ctx, "is_live_takeover_active", json!({}))
        .ok()
        .and_then(|value| value.as_bool());
    match proxy_running {
        Some(true) => {
            let takeover = if takeover == Some(true) {
                "，已接管 live 配置"
            } else {
                ""
            };
            lines.push_str(&format!("本地路由: 运行中{takeover}\n"));
        }
        Some(false) => lines.push_str("本地路由: 未运行\n"),
        None => lines.push_str("本地路由: 未知\n"),
    }

    lines.push_str("\n当前供应商:\n");
    for app in crate::app_config::AppType::all() {
        let id = app.as_str();
        let current = call(ctx, "get_current_provider", json!({ "app": id }))
            .ok()
            .and_then(|value| value.as_str().map(str::to_string))
            .unwrap_or_else(|| "-".to_string());
        lines.push_str(&format!("  {id:<14} {current}\n"));
    }

    Ok(Payload::Text(lines))
}
