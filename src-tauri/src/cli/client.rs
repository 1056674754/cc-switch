//! CLI 的执行通道：优先附加到运行中的桌面版，否则自己 headless 执行一次。

use std::time::Duration;

use serde_json::{json, Value};

use crate::control::{read_descriptor, PROTOCOL};

/// 单条命令的超时：同步/备份类命令可能很慢，给足时间。
const CALL_TIMEOUT: Duration = Duration::from_secs(600);
/// 发现桌面版时的探活超时：桌面版刚退出时不要卡太久。
const HEALTH_TIMEOUT: Duration = Duration::from_millis(800);

/// 命令执行通道。
pub struct Client {
    target: Target,
    runtime: tokio::runtime::Runtime,
}

enum Target {
    /// 桌面版在跑：命令发给它执行，界面会实时刷新。
    Attached {
        port: u16,
        token: String,
        version: String,
        mode: String,
    },
    /// 桌面版没在跑：CLI 自己起 headless 应用执行一条命令。
    Standalone,
}

impl Client {
    /// 建立通道。`force_standalone` 对应 `--standalone`。
    pub fn connect(force_standalone: bool, quiet: bool) -> Result<Self, String> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|error| format!("创建异步运行时失败：{error}"))?;

        if force_standalone {
            return Ok(Self::standalone(runtime));
        }

        let path = descriptor_path();
        match read_descriptor(&path) {
            Ok(descriptor) => {
                if descriptor.protocol != PROTOCOL {
                    return Err(format!(
                        "桌面版协议版本是 {}，CLI 要求 {}。请重启桌面版（或升级 CLI）后再试。",
                        descriptor.protocol, PROTOCOL
                    ));
                }
                if !probe(&runtime, descriptor.port) {
                    if !quiet {
                        eprintln!(
                            "ccswitch: 发现 {} 但连不上（桌面版可能正在退出），改用独立模式",
                            path.display()
                        );
                    }
                    return Ok(Self::standalone(runtime));
                }
                Ok(Self {
                    target: Target::Attached {
                        port: descriptor.port,
                        token: descriptor.token,
                        version: descriptor.version,
                        mode: descriptor.mode,
                    },
                    runtime,
                })
            }
            Err(_) => {
                if !quiet {
                    if desktop_socket_exists() {
                        eprintln!(
                            "ccswitch: 独立模式。注意：桌面版看起来正在运行但没有控制端口（旧版本或设置了 CC_SWITCH_NO_CONTROL），本次修改可能在界面上看不到。"
                        );
                    } else {
                        eprintln!("ccswitch: 独立模式（桌面版未运行）");
                    }
                }
                Ok(Self::standalone(runtime))
            }
        }
    }

    fn standalone(runtime: tokio::runtime::Runtime) -> Self {
        Self {
            target: Target::Standalone,
            runtime,
        }
    }

    pub fn is_attached(&self) -> bool {
        matches!(self.target, Target::Attached { .. })
    }

    /// 人类可读的通道说明（`ccswitch status` 用）。
    pub fn describe(&self) -> String {
        match &self.target {
            Target::Attached { version, mode, .. } => {
                format!("已连接桌面版 v{version}（{mode}）")
            }
            Target::Standalone => "独立模式（桌面版未运行）".to_string(),
        }
    }

    /// 执行一条命令。
    pub fn call(&self, command: &str, args: Value) -> Result<Value, String> {
        match &self.target {
            Target::Standalone => crate::cli::once::run(command, args),
            Target::Attached { port, token, .. } => {
                let url = format!("http://127.0.0.1:{port}/rpc");
                let body = json!({ "command": command, "args": args });
                let response = self
                    .runtime
                    .block_on(async {
                        reqwest::Client::builder()
                            .timeout(CALL_TIMEOUT)
                            .build()?
                            .post(&url)
                            .header("x-ccswitch-token", token)
                            .json(&body)
                            .send()
                            .await
                    })
                    .map_err(|error| format!("请求桌面版失败（{url}）：{error}"))?;

                match response.status().as_u16() {
                    401 => Err(format!(
                        "桌面版拒绝了请求（token 无效，可能重启过）。重试一次即可：如果仍失败，删掉 {} 后再试",
                        descriptor_path().display()
                    )),
                    status if status != 200 => {
                        Err(format!("桌面版返回 HTTP {status}（{url}）"))
                    }
                    _ => {
                        let value: Value = self
                            .runtime
                            .block_on(response.json())
                            .map_err(|error| format!("桌面版返回的不是 JSON：{error}"))?;
                        match value.get("ok").and_then(Value::as_bool) {
                            Some(true) => Ok(value.get("data").cloned().unwrap_or(Value::Null)),
                            Some(false) => Err(value
                                .get("error")
                                .and_then(Value::as_str)
                                .unwrap_or("桌面版返回了未知错误")
                                .to_string()),
                            None => Err(format!("桌面版响应格式不对：{value}")),
                        }
                    }
                }
            }
        }
    }

    /// 读桌面版自报信息；独立模式返回 `None`。
    pub fn health(&self) -> Option<Value> {
        let Target::Attached { port, .. } = &self.target else {
            return None;
        };
        let url = format!("http://127.0.0.1:{port}/health");
        self.runtime.block_on(async {
            reqwest::Client::builder()
                .timeout(HEALTH_TIMEOUT)
                .build()
                .ok()?
                .get(&url)
                .send()
                .await
                .ok()?
                .json::<Value>()
                .await
                .ok()
        })
    }

    /// 命令名列表：附加模式问桌面版（版本可能不同），独立模式用本地生成的表。
    pub fn commands(&self) -> Result<Vec<String>, String> {
        match &self.target {
            Target::Standalone => Ok(crate::commands::command_table::COMMANDS
                .iter()
                .map(|name| (*name).to_string())
                .collect()),
            Target::Attached { port, token, .. } => {
                let url = format!("http://127.0.0.1:{port}/commands");
                self.runtime
                    .block_on(async {
                        reqwest::Client::builder()
                            .timeout(HEALTH_TIMEOUT)
                            .build()?
                            .get(&url)
                            .header("x-ccswitch-token", token)
                            .send()
                            .await
                    })
                    .map_err(|error| format!("请求桌面版命令表失败：{error}"))
                    .and_then(|response| {
                        if response.status() != 200 {
                            return Err(format!("桌面版返回 HTTP {}", response.status()));
                        }
                        let value: Value = self
                            .runtime
                            .block_on(response.json())
                            .map_err(|error| format!("命令表不是 JSON：{error}"))?;
                        value
                            .get("data")
                            .and_then(Value::as_array)
                            .map(|items| {
                                items
                                    .iter()
                                    .filter_map(|item| item.as_str().map(str::to_string))
                                    .collect()
                            })
                            .ok_or_else(|| "命令表格式不对".to_string())
                    })
            }
        }
    }
}

fn descriptor_path() -> std::path::PathBuf {
    if let Some(path) = std::env::var_os("CC_SWITCH_CONTROL_DESCRIPTOR") {
        if !path.is_empty() {
            return std::path::PathBuf::from(path);
        }
    }
    crate::control::descriptor_path()
}

/// 探活：桌面版刚退出时可能连不上，快速失败。
fn probe(runtime: &tokio::runtime::Runtime, port: u16) -> bool {
    let url = format!("http://127.0.0.1:{port}/health");
    runtime
        .block_on(async {
            reqwest::Client::builder()
                .timeout(HEALTH_TIMEOUT)
                .build()
                .ok()?
                .get(&url)
                .send()
                .await
                .ok()
                .map(|response| response.status().is_success())
        })
        .unwrap_or(false)
}

/// 桌面版在跑时会持有单实例 socket（`/tmp/<identifier>_si.sock`）。
///
/// 这个检测只用来在「没找到控制端口」时给一句更准的提示：桌面版可能在跑但版本旧，
/// 这时独立执行会和界面状态不一致，值得提醒。
fn desktop_socket_exists() -> bool {
    #[cfg(unix)]
    {
        let path = crate::control::desktop_socket_path();
        path.exists()
    }
    #[cfg(not(unix))]
    {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::path::{Path, PathBuf};

    /// 每个用例都改 `CC_SWITCH_CONTROL_DESCRIPTOR`，串行跑，避免互相串环境变量。
    struct DescriptorEnv {
        path: PathBuf,
        previous: Option<std::ffi::OsString>,
    }

    impl DescriptorEnv {
        fn new(dir: &Path, descriptor: Option<&crate::control::Descriptor>) -> Self {
            let path = dir.join("control-server.json");
            if let Some(descriptor) = descriptor {
                // 测试里自己落盘，避免把 control 的写函数提成 pub。
                std::fs::write(&path, serde_json::to_vec(descriptor).unwrap()).unwrap();
            }
            let previous = std::env::var_os("CC_SWITCH_CONTROL_DESCRIPTOR");
            std::env::set_var("CC_SWITCH_CONTROL_DESCRIPTOR", &path);
            Self { path, previous }
        }
    }

    impl Drop for DescriptorEnv {
        fn drop(&mut self) {
            match &self.previous {
                Some(value) => std::env::set_var("CC_SWITCH_CONTROL_DESCRIPTOR", value),
                None => std::env::remove_var("CC_SWITCH_CONTROL_DESCRIPTOR"),
            }
            let _ = &self.path;
        }
    }

    fn descriptor(port: u16, protocol: u32) -> crate::control::Descriptor {
        crate::control::Descriptor {
            protocol,
            pid: std::process::id(),
            port,
            token: "a".repeat(64),
            version: "9.9.9".to_string(),
            mode: "gui".to_string(),
            started_at: 1_700_000_000_000,
        }
    }

    /// 起一个只会回 `/health` 的本机 HTTP 服务，验证附加逻辑。
    fn spawn_health_server() -> (u16, std::thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let handle = std::thread::spawn(move || {
            for stream in listener.incoming().take(8) {
                let Ok(mut stream) = stream else { return };
                let mut buffer = [0u8; 2048];
                let _ = stream.read(&mut buffer);
                let body = r#"{"ok":true,"pid":1,"version":"9.9.9","commands":413}"#;
                let response = format!(
                    "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = stream.write_all(response.as_bytes());
            }
        });
        (port, handle)
    }

    #[serial_test::serial]
    #[test]
    fn 没有控制端口时退回独立模式() {
        let dir = tempfile::tempdir().unwrap();
        let _env = DescriptorEnv::new(dir.path(), None);
        let client = Client::connect(false, true).unwrap();
        assert!(!client.is_attached());
        assert!(client.describe().contains("独立模式"));
    }

    #[serial_test::serial]
    #[test]
    fn 探活成功后进入附加模式() {
        let (port, handle) = spawn_health_server();
        let dir = tempfile::tempdir().unwrap();
        let _env = DescriptorEnv::new(
            dir.path(),
            Some(&descriptor(port, crate::control::PROTOCOL)),
        );

        let client = Client::connect(false, true).unwrap();
        assert!(client.is_attached(), "{}", client.describe());
        assert!(client.describe().contains("9.9.9"));
        let health = client.health().expect("附加模式应该能读到 health");
        assert_eq!(health["commands"], 413);
        drop(handle);
    }

    #[serial_test::serial]
    #[test]
    fn 控制端口连不上时退回独立模式() {
        // 先占一个端口再释放，保证这个端口没人监听。
        let port = {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            listener.local_addr().unwrap().port()
        };
        let dir = tempfile::tempdir().unwrap();
        let _env = DescriptorEnv::new(
            dir.path(),
            Some(&descriptor(port, crate::control::PROTOCOL)),
        );

        let client = Client::connect(false, true).unwrap();
        assert!(!client.is_attached(), "端口已关闭，不应附加成功");
    }

    #[serial_test::serial]
    #[test]
    fn 协议版本不匹配时直接报错() {
        let dir = tempfile::tempdir().unwrap();
        let _env = DescriptorEnv::new(
            dir.path(),
            Some(&descriptor(1, crate::control::PROTOCOL + 1)),
        );
        match Client::connect(false, true) {
            Ok(_) => panic!("协议版本不匹配时应该直接报错"),
            Err(error) => assert!(error.contains("协议版本"), "{error}"),
        }
    }
}
