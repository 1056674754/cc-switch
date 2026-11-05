//! 子命令实现：只做「拼 JSON 参数 + 调命令名 + 排版输出」。
//!
//! 每个 family 一个文件，`run` 接收已经解析好的命令行和客户端。命令名与参数 key 都直接
//! 对应 `generate_handler!` 里的 Tauri 命令（camelCase key），所以这里不需要额外的协议层。

mod apps;
mod backup;
mod commands;
mod gateway;
mod ipc;
mod mcp;
mod profile;
mod prompt;
mod provider;
mod proxy;
mod session;
mod settings;
mod skill;
mod status;
mod usage;

use super::{Context, Parsed, Payload};
use serde_json::Value;

pub fn run(ctx: &Context, parsed: &Parsed) -> Result<Payload, String> {
    match parsed.group() {
        "status" => status::run(ctx, parsed),
        "apps" => apps::run(ctx, parsed),
        "provider" => provider::run(ctx, parsed),
        "ipc" => ipc::run(ctx, parsed),
        "commands" => commands::run(ctx, parsed),
        "mcp" => mcp::run(ctx, parsed),
        "skill" => skill::run(ctx, parsed),
        "settings" => settings::run(ctx, parsed),
        "proxy" => proxy::run(ctx, parsed),
        "gateway" => gateway::run(ctx, parsed),
        "profile" => profile::run(ctx, parsed),
        "prompt" => prompt::run(ctx, parsed),
        "usage" => usage::run(ctx, parsed),
        "session" => session::run(ctx, parsed),
        "backup" => backup::run(ctx, parsed),
        other => Err(format!(
            "未知命令 `{other}`（ccswitch --help 看全部；任意命令可用 `ccswitch ipc <command>` 调用）"
        )),
    }
}

/// 各命令组的一句话说明（`ccswitch --help` 用）。
pub fn group_help() -> Vec<(&'static str, &'static str)> {
    vec![
        ("status", "当前供应商、本地路由、连接状态的概览"),
        ("apps", "列出支持的应用标识"),
        (
            "provider",
            "供应商：列表 / 当前 / 切换 / 增删改 / 应用到 live 配置",
        ),
        ("mcp", "MCP 服务器：列表 / 增删 / 启停 / 同步"),
        ("skill", "技能：列表 / 安装 / 卸载 / 更新 / 同步"),
        ("settings", "应用设置：读取 / 按文件写入"),
        ("proxy", "本地路由：状态 / 启停 / 配置 / 接管"),
        ("gateway", "网关聚合：目录 / 模型覆盖 / 接管与释放"),
        ("profile", "配置档案：列表 / 创建 / 应用 / 清除"),
        ("prompt", "提示词：列表 / 启用 / 删除"),
        ("usage", "用量：汇总 / 趋势 / 请求日志"),
        ("session", "会话：列表 / 消息 / 删除"),
        ("backup", "数据库备份：列表 / 创建 / 恢复"),
        ("ipc", "直接调用任意命令：ccswitch ipc <command> [json]"),
        ("commands", "列出可调用的命令名"),
    ]
}

/// 某个命令组的详细用法。
pub fn group_usage(group: &str) -> Option<&'static str> {
    Some(match group {
        "provider" => {
            "  ccswitch provider list --app <app> [--json]\n\
             \x20 ccswitch provider current --app <app>\n\
             \x20 ccswitch provider use <app> <id|名称>\n\
             \x20 ccswitch provider show <app> <id>\n\
             \x20 ccswitch provider add --app <app> --file <provider.json> [--add-live false]\n\
             \x20 ccswitch provider update --app <app> --file <provider.json> [--id <原 id>]\n\
             \x20 ccswitch provider rm <app> <id>\n\
             \x20 ccswitch provider apply-live <app> <id>\n\
             \x20 ccswitch provider unapply-live <app> <id>\n"
        }
        "status" => "  ccswitch status [--json]\n",
        "apps" => "  ccswitch apps [--json]\n",
        "ipc" => "  ccswitch ipc <command> [<json 参数>]\n  例: ccswitch ipc get_providers '{\"app\":\"claude\"}'\n",
        "commands" => "  ccswitch commands [--json]\n",
        "mcp" => {
            "  ccswitch mcp list [--app <app>] [--json]\n\
             \x20 ccswitch mcp show <id>\n\
             \x20 ccswitch mcp upsert --file <server.json> [--app <app> --id <id>]\n\
             \x20 ccswitch mcp rm <id>\n\
             \x20 ccswitch mcp enable|disable <id> --app <app>\n\
             \x20 ccswitch mcp import [--app <app>]\n\
             \x20 ccswitch mcp resync [--app <app>]\n"
        }
        "skill" => {
            "  ccswitch skill list [--json]\n\
             \x20 ccswitch skill backups\n\
             \x20 ccswitch skill install <目录名> [--app <app>]\n\
             \x20 ccswitch skill install --repo <owner/name>\n\
             \x20 ccswitch skill uninstall <id>\n\
             \x20 ccswitch skill restore <backupId> [--app <app>]\n\
             \x20 ccswitch skill enable|disable <id> --app <app>\n\
             \x20 ccswitch skill scan\n\
             \x20 ccswitch skill resync\n"
        }
        "settings" => {
            "  ccswitch settings get [--json]\n\
             \x20 ccswitch settings set --file <settings.json>\n\
             \x20 ccswitch settings path\n"
        }
        "proxy" => {
            "  ccswitch proxy status [--json]\n\
             \x20 ccswitch proxy start\n\
             \x20 ccswitch proxy stop\n\
             \x20 ccswitch proxy takeover <on|off> [--app <app>]\n\
             \x20 ccswitch proxy config get\n\
             \x20 ccswitch proxy config set --file <proxy-config.json>\n"
        }
        "gateway" => {
            "  ccswitch gateway catalog [--json]\n\
             \x20 ccswitch gateway app-model <app>\n\
             \x20 ccswitch gateway adopt <app> [--file <model-config.json>]\n\
             \x20 ccswitch gateway release <app>\n"
        }
        "profile" => {
            "  ccswitch profile list [--json]\n\
             \x20 ccswitch profile create --name <名称> [--scope <claude|claude-desktop|codex>] [--file <参数.json>]\n\
             \x20 ccswitch profile update <id> [--name <名称>] [--resnapshot] [--file <参数.json>]\n\
             \x20 ccswitch profile rm <id>\n\
             \x20 ccswitch profile apply <id> [--scope <分组>]\n\
             \x20 ccswitch profile clear [--scope <分组>]\n"
        }
        "prompt" => {
            "  ccswitch prompt list [--app <app>] [--json]\n\
             \x20 ccswitch prompt enable <app> <id>\n\
             \x20 ccswitch prompt rm <app> <id>\n\
             \x20 ccswitch prompt upsert --file <prompt.json> [--app <app> --id <id>]\n"
        }
        "usage" => {
            "  ccswitch usage summary [--app <app>] [--since 7d] [--json]\n\
             \x20 ccswitch usage trends [--app <app>] [--since 30d]\n\
             \x20 ccswitch usage logs [--app <app>] [--since 7d] [--limit 50]\n\
             \x20 --since/--until 支持 7d、12h、30m、10/13 位时间戳、YYYY-MM-DD\n"
        }
        "session" => {
            "  ccswitch session list [--json]\n\
             \x20 ccswitch session messages <id> [--provider <providerId>]\n\
             \x20 ccswitch session rm <id> [--provider <providerId>]\n"
        }
        "backup" => {
            "  ccswitch backup list [--json]\n\
             \x20 ccswitch backup create\n\
             \x20 ccswitch backup restore <filename>\n\
             \x20 ccswitch backup rename <old> <new>\n\
             \x20 ccswitch backup rm <filename>\n"
        }
        _ => return None,
    })
}

/// 调一条命令的快捷方式。
pub(super) fn call(ctx: &Context, command: &str, args: Value) -> Result<Value, String> {
    ctx.client.call(command, args)
}

/// `provider use <app> <id>` / `provider rm <app> <id>` 这类两个位置参数的取值。
pub(super) fn two_words(
    parsed: &Parsed,
    first: &str,
    second: &str,
) -> Result<(String, String), String> {
    let app = parsed.app(2).map_err(|_| format!("缺少参数 <{first}>"))?;
    let second = parsed.require_word(3, second).map(str::to_string)?;
    Ok((app, second))
}
