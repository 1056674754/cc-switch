//! `ccswitch status`：一眼看清当前状态。

use serde_json::{json, Map, Value};

use super::super::{Context, Parsed, Payload};
use super::call;

pub fn run(ctx: &Context, _parsed: &Parsed) -> Result<Payload, String> {
    let health = ctx.client.health();
    let proxy_running = call(ctx, "is_proxy_running", json!({}))
        .ok()
        .and_then(|value| value.as_bool());
    let takeover = call(ctx, "is_live_takeover_active", json!({}))
        .ok()
        .and_then(|value| value.as_bool());

    let mut providers = Map::new();
    for app in crate::app_config::AppType::all() {
        let id = app.as_str();
        let current = call(ctx, "get_current_provider", json!({ "app": id }))
            .ok()
            .and_then(|value| value.as_str().map(str::to_string))
            .unwrap_or_else(|| "-".to_string());
        providers.insert(id.to_string(), Value::String(current));
    }

    if ctx.json {
        let mut out = Map::new();
        out.insert("attached".into(), Value::Bool(ctx.client.is_attached()));
        out.insert("channel".into(), Value::String(ctx.client.describe()));
        if let Some(health) = &health {
            out.insert("desktop".into(), health.clone());
        }
        out.insert(
            "proxy".into(),
            json!({ "running": proxy_running, "takeover": takeover }),
        );
        out.insert("providers".into(), Value::Object(providers));
        return Ok(Payload::Value(Value::Object(out)));
    }

    let mut lines = String::new();
    lines.push_str("CC Switch\n");
    lines.push_str(&format!("通道: {}\n", ctx.client.describe()));
    if let Some(health) = &health {
        if let Some(pid) = health.get("pid").and_then(Value::as_u64) {
            lines.push_str(&format!("桌面进程: pid={pid}\n"));
        }
        if let Some(commands) = health.get("commands").and_then(Value::as_u64) {
            lines.push_str(&format!("可调命令: {commands}\n"));
        }
    }

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
    for (id, current) in &providers {
        let current = current.as_str().unwrap_or("-");
        lines.push_str(&format!("  {id:<14} {current}\n"));
    }

    Ok(Payload::Text(lines))
}
