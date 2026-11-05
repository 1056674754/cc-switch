//! 独立执行模式：桌面版没在跑时，CLI 自己起一次 headless 应用、执行一条命令、退出。
//!
//! 实现方式是把「要执行什么」放进静态变量，然后调用 `crate::run()` 走完整的应用启动流程
//! （和桌面版一样的数据库迁移、`AppState`/`ProxyService` 初始化），setup 末尾把命令分派
//! 结果写回静态变量并退出事件循环。这样 CLI 与桌面版的行为完全一致，不需要另写一套服务逻辑。

use std::sync::{Mutex, OnceLock};

use serde_json::Value;

/// 一次要执行的命令。
#[derive(Debug)]
pub struct Request {
    pub command: String,
    pub args: Value,
}

static REQUEST: OnceLock<Request> = OnceLock::new();
static RESULT: Mutex<Option<Result<Value, String>>> = Mutex::new(None);

/// 当前进程是否处于「CLI 单次执行」模式。
pub fn is_one_shot() -> bool {
    REQUEST.get().is_some()
}

/// setup 里读取要执行的命令。
pub(crate) fn request() -> Option<&'static Request> {
    REQUEST.get()
}

/// 把命令结果交回 CLI 主流程。
pub(crate) fn store_result(result: Result<Value, String>) {
    if let Ok(mut slot) = RESULT.lock() {
        *slot = Some(result);
    }
}

/// 独立执行一条命令。会阻塞直到应用退出。
pub fn run(command: &str, args: Value) -> Result<Value, String> {
    let request = Request {
        command: command.to_string(),
        args,
    };
    if REQUEST.set(request).is_err() {
        return Err("CLI 一次性执行已经启动过".to_string());
    }
    if let Ok(mut slot) = RESULT.lock() {
        *slot = None;
    }

    crate::run();

    RESULT
        .lock()
        .ok()
        .and_then(|mut slot| slot.take())
        .unwrap_or_else(|| Err("命令没有返回结果（应用提前退出）".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 初始状态不是单次执行模式() {
        assert!(!is_one_shot());
        assert!(request().is_none());
    }
}
