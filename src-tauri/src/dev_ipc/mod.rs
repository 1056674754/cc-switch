//! 浏览器开发模式的 IPC 桥（`--headless` + `feature = "dev-ipc"`）。
//!
//! 目的：让 UI 在普通浏览器里跑 Vite 热更新，同时 core（DB / gateway / 转发）留在一个
//! 长生命周期的 Rust 进程里不重启。做法是把 Tauri 的 IPC 命令面暴露成 HTTP：
//!
//! - `POST /ipc/<command>`：body 是参数对象（camelCase，和 `invoke` 的 args 一致），
//!   返回 `{"ok":true,"data":...}` / `{"ok":false,"error":"..."}`；
//! - `GET  /ipc/health`：进程信息，前端拿版本号、家目录等；
//! - `GET  /ipc/commands`：当前桥接的命令名列表；
//! - `WS   /ipc/events`：`app.emit` 的事件，帧格式 `{"event":"...","payload":...}`。
//!
//! 命令本身走的是和桌面端完全相同的实现：headless 进程仍然是一个真的 Tauri app
//! （真 `AppHandle`、真 managed state、真 AppState/ProxyService），只是没有窗口。
//! 分发表 `crate::commands::dev_ipc_table` 由 `scripts/gen_dev_ipc_adapters.py` 从
//! `generate_handler!` 的命令表和各命令签名生成，保证一个不漏。
//!
//! 安全边界：只在编译期打开 `dev-ipc` feature 时存在，只绑 `127.0.0.1`，且必须显式
//! `--headless` / `CC_SWITCH_DEV_IPC=1` 才会启动。正式构建里这段代码根本不编进去。

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::OnceLock;
use std::time::Instant;

use axum::body::Bytes;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::{json, Map, Value};
use tauri::{AppHandle, Listener, Manager};
use tokio::sync::broadcast;

/// 默认端口。和 `scripts/dev-core.sh` / vite 代理里的默认值保持一致。
pub const DEFAULT_PORT: u16 = 3999;

/// 事件广播缓冲：浏览器断线重连期间丢几条通知无妨，UI 会重新拉取。
const EVENT_BUFFER: usize = 256;

/// 桥接的命令数量与名单由生成的表决定。
pub(crate) use crate::commands::dev_ipc_table::{COMMANDS, COMMAND_COUNT};

static EVENT_TX: OnceLock<broadcast::Sender<String>> = OnceLock::new();
static CONFIG: OnceLock<Option<Config>> = OnceLock::new();

/// headless 桥的启动配置。
#[derive(Clone, Copy, Debug)]
pub struct Config {
    pub port: u16,
}

/// 从命令行 / 环境变量解析一次并缓存。
///
/// 开启方式（任一）：`--headless`、`--dev-ipc`、`CC_SWITCH_DEV_IPC=1`。
/// 端口：`--dev-ipc-port <n>` / `--dev-ipc-port=<n>` / `CC_SWITCH_DEV_IPC_PORT`。
pub fn config() -> Option<&'static Config> {
    CONFIG
        .get_or_init(|| {
            let mut enabled = std::env::var("CC_SWITCH_DEV_IPC")
                .map(|v| !v.is_empty() && v != "0")
                .unwrap_or(false);
            let mut port = std::env::var("CC_SWITCH_DEV_IPC_PORT")
                .ok()
                .and_then(|v| v.trim().parse::<u16>().ok());

            let args: Vec<String> = std::env::args().collect();
            let mut index = 1;
            while index < args.len() {
                let arg = args[index].as_str();
                match arg {
                    "--headless" | "--dev-ipc" => enabled = true,
                    "--dev-ipc-port" => {
                        if let Some(value) = args.get(index + 1) {
                            port = value.trim().parse::<u16>().ok().or(port);
                            index += 1;
                        }
                    }
                    _ => {
                        if let Some(value) = arg.strip_prefix("--dev-ipc-port=") {
                            port = value.trim().parse::<u16>().ok().or(port);
                        }
                    }
                }
                index += 1;
            }

            enabled.then(|| Config {
                port: port.unwrap_or(DEFAULT_PORT),
            })
        })
        .as_ref()
}

/// 是否处于 headless（无窗口）模式。生产构建里这个 feature 不编译，恒为 false。
pub fn is_headless() -> bool {
    config().is_some()
}

/// 把事件广播进 `/ipc/events`。没有订阅者时静默丢弃。
fn broadcast(event: &str, payload_json: &str) {
    let Some(tx) = EVENT_TX.get() else {
        return;
    };
    let payload_json = if payload_json.trim().is_empty() {
        "null"
    } else {
        payload_json
    };
    let event_json = serde_json::to_string(event).unwrap_or_else(|_| "\"\"".to_string());
    let _ = tx.send(format!(
        "{{\"event\":{event_json},\"payload\":{payload_json}}}"
    ));
}

/// 桥接的 HTTP 服务状态。
#[derive(Clone)]
struct Ctx {
    app: AppHandle,
    events: broadcast::Sender<String>,
    started: Instant,
}

/// 在 headless 模式下启动桥。由 `lib.rs` 的 setup 调用（窗口/托盘已按 headless 跳过）。
pub fn start(app: AppHandle) {
    let Some(config) = config().copied() else {
        return;
    };

    let (events, _receiver) = broadcast::channel::<String>(EVENT_BUFFER);
    if EVENT_TX.set(events.clone()).is_err() {
        log::warn!("[dev-ipc] 事件广播已经初始化过，忽略重复启动");
        return;
    }

    // 注册 Rust 侧监听：桌面端相当于 `webview.eval` 的 JS 监听，这里改成推给 WebSocket。
    for name in EVENT_NAMES {
        let event = (*name).to_string();
        app.listen_any(event.clone(), move |payload| {
            broadcast(&event, payload.payload());
        });
    }

    let ctx = Ctx {
        app,
        events,
        started: Instant::now(),
    };

    tauri::async_runtime::spawn(async move {
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), config.port);
        let listener = match tokio::net::TcpListener::bind(addr).await {
            Ok(listener) => listener,
            Err(error) => {
                log::error!(
                    "[dev-ipc] 绑定 {addr} 失败：{error}（端口被占用？换 CC_SWITCH_DEV_IPC_PORT）"
                );
                return;
            }
        };

        let router = Router::new()
            .route("/ipc/health", get(health))
            .route("/ipc/commands", get(list_commands))
            .route("/ipc/events", get(events_ws))
            .route("/ipc/:command", post(invoke_command))
            .layer(tower_http::cors::CorsLayer::permissive())
            .with_state(ctx);

        log::info!(
            "[dev-ipc] 浏览器 IPC 桥已就绪：http://{addr}/ipc/health（{} 条命令）",
            COMMAND_COUNT
        );
        if let Err(error) = axum::serve(listener, router).await {
            log::error!("[dev-ipc] HTTP 服务退出：{error}");
        }
    });
}

/// 前端会监听的事件名。和 `src/**` 里的 `listen(...)`、Rust 侧 `emit(...)` 的并集对齐；
/// 新增事件时两边一起加（漏了只影响实时刷新，不影响功能）。
const EVENT_NAMES: &[&str] = &[
    "configLoadError",
    "deeplink-pending",
    "profile-applied",
    "provider-switched",
    "proxy-flags-changed",
    "proxy-official-warning",
    "s3-sync-status-updated",
    "tray-navigate",
    "universal-provider-synced",
    "update-download-progress",
    "usage-cache-updated",
    "usage-log-recorded",
    "webdav-sync-status-updated",
];

async fn health(State(ctx): State<Ctx>) -> impl IntoResponse {
    let health = json!({
        "ok": true,
        "devIpc": true,
        "mode": "headless",
        "pid": std::process::id(),
        "version": ctx.app.package_info().version.to_string(),
        "platform": std::env::consts::OS,
        "home": crate::config::get_home_dir().display().to_string(),
        "appConfigDir": crate::config::get_app_config_dir().display().to_string(),
        "commands": COMMAND_COUNT,
        "uptimeMs": ctx.started.elapsed().as_millis() as u64,
    });
    (StatusCode::OK, Json(health))
}

async fn list_commands() -> impl IntoResponse {
    Json(json!({ "ok": true, "data": COMMANDS }))
}

async fn events_ws(State(ctx): State<Ctx>, ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(move |socket| pump_events(socket, ctx.events.subscribe()))
}

async fn pump_events(mut socket: WebSocket, mut events: broadcast::Receiver<String>) {
    loop {
        tokio::select! {
            message = events.recv() => match message {
                Ok(text) => {
                    if socket.send(Message::Text(text)).await.is_err() {
                        break;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => break,
            },
            incoming = socket.recv() => match incoming {
                // 浏览器侧的 ping/pong/close 都交给 axum 处理，这里只负责退出。
                Some(Ok(_)) => {}
                _ => break,
            },
        }
    }
}

async fn invoke_command(
    State(ctx): State<Ctx>,
    Path(command): Path<String>,
    body: Bytes,
) -> impl IntoResponse {
    let args = if body.is_empty() {
        Value::Null
    } else {
        match serde_json::from_slice::<Value>(&body) {
            Ok(args) => args,
            Err(error) => {
                let payload = json!({ "ok": false, "error": format!("dev-ipc: 请求体不是合法 JSON：{error}") });
                return (StatusCode::OK, Json(payload));
            }
        }
    };

    let payload = match crate::commands::dev_ipc_table::dispatch(&command, &ctx.app, args).await {
        Ok(data) => json!({ "ok": true, "data": data }),
        Err(error) => json!({ "ok": false, "error": error }),
    };
    (StatusCode::OK, Json(payload))
}

/// 命令参数的 JSON 取值器。key 是前端传的 camelCase 名字。
#[derive(Debug)]
pub struct Args {
    map: Map<String, Value>,
}

impl Args {
    pub fn new(value: Value) -> Result<Self, String> {
        match value {
            Value::Null => Ok(Self { map: Map::new() }),
            Value::Object(map) => Ok(Self { map }),
            other => Err(format!(
                "dev-ipc: 参数必须是 JSON 对象，收到 {}",
                match other {
                    Value::Array(_) => "数组",
                    Value::String(_) => "字符串",
                    Value::Number(_) => "数字",
                    Value::Bool(_) => "布尔值",
                    _ => "其它类型",
                }
            )),
        }
    }

    /// 必填参数。缺失或类型不符都返回可读错误。
    pub fn required<T: DeserializeOwned>(&mut self, key: &str) -> Result<T, String> {
        match self.map.remove(key) {
            None => Err(format!("dev-ipc: 缺少参数 `{key}`")),
            Some(value) => serde_json::from_value(value)
                .map_err(|error| format!("dev-ipc: 参数 `{key}` 解析失败：{error}")),
        }
    }

    /// 可选参数：缺失或显式 `null` 都得到 `None`。
    pub fn option<T: DeserializeOwned>(&mut self, key: &str) -> Result<Option<T>, String> {
        match self.map.remove(key) {
            None => Ok(None),
            Some(value) => serde_json::from_value::<Option<T>>(value)
                .map_err(|error| format!("dev-ipc: 参数 `{key}` 解析失败：{error}")),
        }
    }

    /// 取 managed state；没注册时给一条明确错误而不是 panic。
    pub fn state<'a, T: Send + Sync + 'static>(
        &self,
        app: &'a AppHandle,
    ) -> Result<tauri::State<'a, T>, String> {
        app.try_state::<T>().ok_or_else(|| {
            format!(
                "dev-ipc: state {} 未注册（headless 启动流程是否跳过了初始化？）",
                std::any::type_name::<T>()
            )
        })
    }
}

/// 命令的桥接返回值。
pub type Outcome = Result<Value, String>;

/// 把命令返回值转成 JSON。序列化失败说明这个命令不适合桥接，给可读错误。
pub fn ok<T: Serialize>(value: T) -> Outcome {
    serde_json::to_value(value).map_err(|error| format!("dev-ipc: 返回值序列化失败：{error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 必填与可选参数都按_camel_case_key_取值() {
        let mut args = Args::new(json!({ "app": "claude", "addToLive": true })).unwrap();
        assert_eq!(args.required::<String>("app").unwrap(), "claude");
        assert_eq!(args.option::<bool>("addToLive").unwrap(), Some(true));
        assert_eq!(args.option::<String>("missing").unwrap(), None);
        let error = args.required::<String>("missing").unwrap_err();
        assert!(error.contains("缺少参数"), "{error}");
    }

    #[test]
    fn 参数不是对象时给出可读错误() {
        let error = Args::new(json!([1, 2, 3])).unwrap_err();
        assert!(error.contains("必须是 JSON 对象"), "{error}");
        // 空 body 走 Null，等价于没有参数。
        assert!(Args::new(Value::Null).is_ok());
    }

    #[test]
    fn 事件名清单不重复() {
        let mut names = EVENT_NAMES.to_vec();
        names.sort_unstable();
        let total = names.len();
        names.dedup();
        assert_eq!(total, names.len(), "EVENT_NAMES 里有重复事件");
    }
}
