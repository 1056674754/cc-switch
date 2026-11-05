//! 命令桥接层的公共类型（命令行 + 浏览器开发模式共用）。
//!
//! 这里只放「怎么把一条 Tauri 命令当成一次 JSON 调用」的通用零件：
//!
//! - [`Args`]：把前端/命令行传来的 camelCase JSON 对象解成命令参数；
//! - [`Outcome`] / [`ok`]：命令返回值的统一形状。
//!
//! 真正的分发表在 [`crate::commands::command_table`]：它由
//! `scripts/gen_command_adapters.py` 从 `generate_handler!` 注册表和各命令签名生成，
//! 保证一个命令都不漏。适配函数按源文件被 `include!` 进命令所在模块（见每个命令文件
//! 末尾的 `mod ipc_adapters`），所以参数和返回类型可以直接用父模块的 import。
//!
//! 为什么这一层不按 feature 门禁：桌面版内置的本地控制服务（`crate::control`）和
//! `ccswitch` 命令行都依赖它，正式构建里必须有。浏览器开发模式的 HTTP/WS 桥
//! （`crate::dev_ipc`）仍然只在 `dev-ipc` feature 下编译。

use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::{Map, Value};
use tauri::Manager;

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
                "ipc: 参数必须是 JSON 对象，收到 {}",
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
            None => Err(format!("ipc: 缺少参数 `{key}`")),
            Some(value) => serde_json::from_value(value)
                .map_err(|error| format!("ipc: 参数 `{key}` 解析失败：{error}")),
        }
    }

    /// 可选参数：缺失或显式 `null` 都得到 `None`。
    pub fn option<T: DeserializeOwned>(&mut self, key: &str) -> Result<Option<T>, String> {
        match self.map.remove(key) {
            None => Ok(None),
            Some(value) => serde_json::from_value::<Option<T>>(value)
                .map_err(|error| format!("ipc: 参数 `{key}` 解析失败：{error}")),
        }
    }

    /// 取 managed state；没注册时给一条明确错误而不是 panic。
    pub fn state<'a, T: Send + Sync + 'static>(
        &self,
        app: &'a tauri::AppHandle,
    ) -> Result<tauri::State<'a, T>, String> {
        app.try_state::<T>().ok_or_else(|| {
            format!(
                "ipc: state {} 未注册（启动流程是否跳过了初始化？）",
                std::any::type_name::<T>()
            )
        })
    }
}

/// 命令的桥接返回值。
pub type Outcome = Result<Value, String>;

/// 把命令返回值转成 JSON。序列化失败说明这个命令不适合桥接，给可读错误。
pub fn ok<T: Serialize>(value: T) -> Outcome {
    serde_json::to_value(value).map_err(|error| format!("ipc: 返回值序列化失败：{error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

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
}
