//! 桌面版的本地控制服务：`ccswitch` 命令行通过它驱动正在运行的桌面进程。
//!
//! ## 为什么需要它
//!
//! CLI 如果自己打开数据库改配置，桌面版内存里的状态就滞后了（切了供应商界面还显示旧的）。
//! 所以桌面版常驻一个极小的 loopback 服务：CLI 把「命令 + 参数」发过来，由桌面进程用
//! 和 UI 完全相同的命令实现执行（数据库、内存状态、live 配置文件都是同一份），执行完再
//! `emit("cli-command-applied")` 让前端重新拉数据——这样 CLI 改完界面立刻就是新的。
//!
//! ## 协议
//!
//! - 只绑 `127.0.0.1`，端口由系统分配（`0`），避免和别的程序抢固定端口；
//! - 启动时生成随机 token，和端口一起写进 `<app config dir>/control-server.json`
//!   （权限 0600），CLI 读这个文件来发现服务；
//! - `GET  /health`：无需鉴权，只回 pid/version/protocol，供 CLI 判断描述文件是否还新鲜；
//! - `GET  /commands`：需要 token，返回可调命令名列表；
//! - `POST /rpc`：需要 token，body `{"command":"...","args":{...}}`，返回
//!   `{"ok":true,"data":...}` / `{"ok":false,"error":"..."}`（HTTP 恒 200）；
//! - 写命令执行成功后 `emit("cli-command-applied", {command, module, app, ok})`，前端按
//!   `module` 只刷新被改动的那部分数据；只读命令不发事件，避免无谓刷新。
//!
//! ## 安全边界
//!
//! 能执行的分派表就是桌面端的 415 条命令，等价于「本机当前用户能做的事」；token 保证
//! 只有能读该配置目录的进程（也就是同一个用户）能调用。浏览器页面拿不到 token，
//! 也没有 CORS 头，所以网页无法驱动它。可用 `CC_SWITCH_NO_CONTROL=1` 整体关闭。
//!
//! 进程崩溃会留下过期的 `control-server.json`；CLI 会先 ping `/health` 并校验 pid，
//! 连不上就忽略它（必要时回退到 headless 单次执行）。

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use axum::body::Bytes;
use axum::extract::{DefaultBodyLimit, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::{AppHandle, Emitter};

/// 协议版本。CLI 与桌面版都要校验，避免半新半旧。
pub const PROTOCOL: u32 = 1;

/// 桌面版的 bundle identifier。和 `tauri.conf.json` 里的 `identifier` 必须一致
/// （有测试守着），单实例插件的 socket 名字由它决定。
pub const APP_IDENTIFIER: &str = "com.ccswitch.desktop";

/// 单实例插件在 macOS/Linux 上用的 socket 路径：`/tmp/<identifier>_si.sock`。
///
/// CLI 只拿它做「桌面版是否在运行」的辅助判断：找不到控制端口时给一句更准的提示。
#[cfg(unix)]
pub fn desktop_socket_path() -> PathBuf {
    PathBuf::from(format!("/tmp/{APP_IDENTIFIER}_si.sock"))
}

/// 描述文件名（放在 app config 目录里）。
const DESCRIPTOR_FILE: &str = "control-server.json";

/// 前端收到这个事件后按 `module` 精确刷新对应域的数据（不做全量 invalidate）。
pub const CHANGE_EVENT: &str = "cli-command-applied";

/// 服务监听地址固定为 loopback；端口由系统分配。
const BIND_HOST: &str = "127.0.0.1";

/// 请求体上限：导出配置类命令可能带整份数据，给 32 MiB。
const MAX_BODY_BYTES: usize = 32 * 1024 * 1024;

static DESCRIPTOR_PATH: OnceLock<PathBuf> = OnceLock::new();
static TOKEN: OnceLock<String> = OnceLock::new();
static STARTED_AT: OnceLock<u64> = OnceLock::new();
static MODE: OnceLock<String> = OnceLock::new();

/// 描述文件内容：CLI 靠它找到端口和 token。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Descriptor {
    pub protocol: u32,
    pub pid: u32,
    pub port: u16,
    pub token: String,
    pub version: String,
    /// `gui` 或 `headless`（headless 是浏览器开发模式的核心）。
    pub mode: String,
    /// Unix 毫秒时间戳。
    pub started_at: u64,
}

/// 描述文件路径。CLI 与桌面版共用。
pub fn descriptor_path() -> PathBuf {
    crate::config::get_app_config_dir().join(DESCRIPTOR_FILE)
}

/// 生成 64 位十六进制 token（128 bit 随机数）。
pub fn new_token() -> String {
    format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    )
}

/// 启动控制服务。setup 里调用一次；重复调用无副作用。
pub fn start(app: AppHandle, mode: &'static str) {
    if std::env::var("CC_SWITCH_NO_CONTROL")
        .map(|value| !value.is_empty() && value != "0")
        .unwrap_or(false)
    {
        log::info!("[control] CC_SWITCH_NO_CONTROL 已设置，跳过本地控制服务");
        return;
    }

    let token = new_token();
    let started_at = now_millis();
    if TOKEN.set(token.clone()).is_err() {
        log::warn!("[control] 控制服务已启动过，忽略重复调用");
        return;
    }
    let _ = STARTED_AT.set(started_at);
    let _ = MODE.set(mode.to_string());

    let version = app.package_info().version.to_string();
    let descriptor_path = descriptor_path();
    let _ = DESCRIPTOR_PATH.set(descriptor_path.clone());

    tauri::async_runtime::spawn(async move {
        let listener = match tokio::net::TcpListener::bind((BIND_HOST, 0)).await {
            Ok(listener) => listener,
            Err(error) => {
                log::error!("[control] 绑定 {BIND_HOST}:0 失败：{error}");
                return;
            }
        };
        let port = match listener.local_addr() {
            Ok(addr) => addr.port(),
            Err(error) => {
                log::error!("[control] 读取监听端口失败：{error}");
                return;
            }
        };

        let descriptor = Descriptor {
            protocol: PROTOCOL,
            pid: std::process::id(),
            port,
            token: token.clone(),
            version: version.clone(),
            mode: mode.to_string(),
            started_at,
        };

        if let Err(error) = write_descriptor(&descriptor_path, &descriptor) {
            log::error!(
                "[control] 写描述文件失败（CLI 将无法发现桌面版）：{}：{error}",
                descriptor_path.display()
            );
            return;
        }

        let ctx = Ctx {
            app,
            token,
            version,
        };

        let router = Router::new()
            .route("/health", get(health))
            .route("/commands", get(list_commands))
            .route("/rpc", post(rpc))
            .layer(DefaultBodyLimit::max(MAX_BODY_BYTES))
            .with_state(ctx);

        log::info!(
            "[control] 本地控制服务已就绪：http://{BIND_HOST}:{port}（{} 条命令，描述文件 {}）",
            crate::commands::command_table::COMMAND_COUNT,
            descriptor_path.display()
        );
        if let Err(error) = axum::serve(listener, router).await {
            log::error!("[control] HTTP 服务退出：{error}");
        }
    });
}

/// 退出时删掉描述文件（只删自己写的那个）。
pub fn shutdown() {
    let Some(path) = DESCRIPTOR_PATH.get() else {
        return;
    };
    let Some(ours) = TOKEN.get() else {
        return;
    };
    match read_descriptor(path) {
        Ok(descriptor) if &descriptor.token == ours => {
            if let Err(error) = std::fs::remove_file(path) {
                log::warn!("[control] 删除描述文件失败：{}：{error}", path.display());
            }
        }
        Ok(_) => {}
        Err(_) => {
            let _ = std::fs::remove_file(path);
        }
    }
}

/// 读取描述文件（CLI 用）。文件不存在或内容损坏都返回错误。
pub fn read_descriptor(path: &Path) -> std::io::Result<Descriptor> {
    let bytes = std::fs::read(path)?;
    serde_json::from_slice(&bytes)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))
}

/// 写描述文件：先写临时文件、设 0600，再原子 rename。
fn write_descriptor(path: &Path, descriptor: &Descriptor) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let temp = path.with_extension("json.tmp");
    let bytes = serde_json::to_vec_pretty(descriptor)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;

    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(&temp)?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    drop(file);
    std::fs::rename(&temp, path)
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_millis() as u64
}

#[derive(Clone)]
struct Ctx {
    app: AppHandle,
    token: String,
    version: String,
}

fn authorized(headers: &HeaderMap, expected: &str) -> bool {
    headers
        .get("x-ccswitch-token")
        .and_then(|value| value.to_str().ok())
        .map(|value| value == expected)
        .unwrap_or(false)
}

async fn health(State(ctx): State<Ctx>) -> impl IntoResponse {
    Json(json!({
        "ok": true,
        "protocol": PROTOCOL,
        "pid": std::process::id(),
        "version": ctx.version,
        "mode": MODE.get().map(String::as_str).unwrap_or("unknown"),
        "commands": crate::commands::command_table::COMMAND_COUNT,
        "uptimeMs": now_millis().saturating_sub(*STARTED_AT.get().unwrap_or(&now_millis())),
    }))
}

async fn list_commands(State(ctx): State<Ctx>, headers: HeaderMap) -> impl IntoResponse {
    if !authorized(&headers, &ctx.token) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({"ok": false, "error": "unauthorized"})),
        );
    }
    (
        StatusCode::OK,
        Json(json!({"ok": true, "data": crate::commands::command_table::COMMANDS})),
    )
}

#[derive(Debug, Deserialize)]
struct RpcRequest {
    command: String,
    #[serde(default)]
    args: Value,
}

async fn rpc(State(ctx): State<Ctx>, headers: HeaderMap, body: Bytes) -> impl IntoResponse {
    if !authorized(&headers, &ctx.token) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({"ok": false, "error": "unauthorized"})),
        );
    }

    let request = match serde_json::from_slice::<RpcRequest>(&body) {
        Ok(request) => request,
        Err(error) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"ok": false, "error": format!("请求体不是合法 JSON：{error}")})),
            );
        }
    };

    // 先把 app 取出来：args 会被 dispatch 消费，事件 payload 还要用它收窄刷新范围。
    let app_arg = command_app(&request.args).map(str::to_string);
    let outcome =
        crate::commands::command_table::dispatch(&request.command, &ctx.app, request.args).await;

    let (ok, payload) = match outcome {
        Ok(data) => (true, json!({"ok": true, "data": data})),
        Err(error) => (false, json!({"ok": false, "error": error})),
    };

    // 只读命令不改数据，不必惊动界面；写命令才通知前端刷新对应域的数据。
    if ok && !is_read_only_command(&request.command) {
        emit_change(&ctx.app, &request.command, app_arg.as_deref());
    }
    (StatusCode::OK, Json(payload))
}

/// 命令参数里的应用标识（CLI 约定 `app`，部分命令用 `appType`）。
fn command_app(args: &serde_json::Value) -> Option<&str> {
    ["app", "appType"]
        .iter()
        .find_map(|key| args.get(*key).and_then(serde_json::Value::as_str))
}

/// 只读命令：CLI 查询状态/列表时不该触发界面刷新。
///
/// 按命名约定判断，默认「可能写」——判错的代价只是多刷一次数据，漏判才会让界面停在旧状态。
fn is_read_only_command(command: &str) -> bool {
    const READ_ONLY_PREFIXES: &[&str] = &[
        "get_",
        "list_",
        "read_",
        "is_",
        "has_",
        "check_",
        "validate_",
        "test_",
        "test",
        "query",
        "search_",
        "scan_",
        "discover_",
        "fetch_",
        "probe_",
        "peek_",
        "inspect_",
        "preview_",
        "export_",
        "open_",
        "reveal_",
        "copy_",
        "cancel_",
        "compute_",
        "resolve_",
        "render_",
        "convert_",
        "parse_",
    ];
    // 命名不带常见前缀、但确实只读的命令（挨个列出来，避免误伤同前缀的写命令）。
    const READ_ONLY_COMMANDS: &[&str] = &[
        "take_startup_attach_failures",
        "take_tray_navigation",
        "tray_panel_state",
        "tray_panel_placement",
        "tray_panel_layout",
        "stream_check_provider",
        "stream_session_messages",
        "extract_common_config_snippet",
        "codex_forces_multi_agent_v2",
        "agy_account_is_running",
        "agy_account_quota",
        "kimi_oauth_status",
        "kimi_oauth_quota",
        "kimi_oauth_models",
        "dsh_account_status",
        "auth_get_status",
        "auth_list_accounts",
        "copilot_get_auth_status",
        "copilot_is_authenticated",
        "copilot_get_token",
        "copilot_get_token_for_account",
        "copilot_get_models",
        "copilot_get_models_for_account",
        "copilot_get_usage",
        "copilot_get_usage_for_account",
        "copilot_list_accounts",
        "webdav_test_connection",
        "s3_test_connection",
        "pick_directory",
        "save_file_dialog",
        "restart_app",
        "install_update_and_restart",
        "update_tray_menu",
        "restart_codex_app_server_daemon",
        "launch_session_terminal",
        "launch_hermes_dashboard",
        "set_window_theme",
        "tray_panel_hide",
        "tray_panel_action",
        "tray_app_page_seen",
    ];
    if READ_ONLY_COMMANDS.contains(&command) {
        return true;
    }
    READ_ONLY_PREFIXES
        .iter()
        .any(|prefix| command.starts_with(prefix))
}

/// 通知界面：CLI 刚改了这个命令所属域的数据。
fn emit_change(app: &AppHandle, command: &str, app_name: Option<&str>) {
    let payload = json!({
        "command": command,
        "module": crate::commands::command_table::module_of(command),
        "app": app_name,
        "ok": true,
    });
    if let Err(error) = app.emit(CHANGE_EVENT, payload) {
        log::warn!("[control] emit {CHANGE_EVENT} 失败：{error}");
    }
    log::debug!("[control] 已通知界面刷新：{command}");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Descriptor {
        Descriptor {
            protocol: PROTOCOL,
            pid: 4242,
            port: 51234,
            token: "a".repeat(64),
            version: "1.2.3".to_string(),
            mode: "gui".to_string(),
            started_at: 1_700_000_000_000,
        }
    }

    #[test]
    fn token_是_128_位随机十六进制() {
        let token = new_token();
        assert_eq!(token.len(), 64);
        assert!(token.chars().all(|c| c.is_ascii_hexdigit()), "{token}");
        assert_ne!(token, new_token());
    }

    #[test]
    fn 描述文件可以原子写入并读回() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("control-server.json");
        let descriptor = sample();
        write_descriptor(&path, &descriptor).unwrap();
        assert_eq!(read_descriptor(&path).unwrap(), descriptor);
        // 临时文件不应该残留
        assert!(!path.with_extension("json.tmp").exists());
        // 不存在的路径给出错误而不是 panic
        assert!(read_descriptor(&dir.path().join("missing.json")).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn 描述文件权限是_0600() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("control-server.json");
        write_descriptor(&path, &sample()).unwrap();
        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "描述文件里有 token，必须只有本人可读");
    }

    #[test]
    fn 只读命令不发刷新事件_写命令发() {
        // 查询类：CLI 的 status / list / show 不该触发任何界面刷新
        assert!(is_read_only_command("get_settings"));
        assert!(is_read_only_command("get_current_provider"));
        assert!(is_read_only_command("list_db_backups"));
        assert!(is_read_only_command("is_proxy_running"));
        assert!(is_read_only_command("stream_session_messages"));
        assert!(is_read_only_command("take_tray_navigation"));
        assert!(is_read_only_command("webdav_test_connection"));

        // 写类：切供应商、改配置、增删 MCP/技能/备份
        assert!(!is_read_only_command("switch_provider"));
        assert!(!is_read_only_command("save_settings"));
        assert!(!is_read_only_command("upsert_mcp_server"));
        assert!(!is_read_only_command("install_skill_unified"));
        assert!(!is_read_only_command("create_db_backup"));
        assert!(!is_read_only_command("set_proxy_takeover_for_app"));
        assert!(!is_read_only_command("update_model_pricing"));
    }

    #[test]
    fn 事件_payload_带模块与_app() {
        // 前端按 module 决定刷新哪个域；app 用来进一步收窄。
        let args = json!({ "app": "claude", "id": "x" });
        assert_eq!(command_app(&args), Some("claude"));
        assert_eq!(command_app(&json!({ "appType": "codex" })), Some("codex"));
        assert_eq!(command_app(&json!({ "id": "x" })), None);
        assert_eq!(command_app(&serde_json::Value::Null), None);

        let module = crate::commands::command_table::module_of("switch_provider");
        assert_eq!(module, Some("provider"));
        assert_eq!(
            crate::commands::command_table::module_of("save_settings"),
            Some("settings")
        );
    }

    #[test]
    fn 鉴权头必须完全匹配() {
        let mut headers = HeaderMap::new();
        assert!(!authorized(&headers, "secret"));
        headers.insert("x-ccswitch-token", "secret".parse().unwrap());
        assert!(authorized(&headers, "secret"));
        assert!(!authorized(&headers, "other"));
    }
}
