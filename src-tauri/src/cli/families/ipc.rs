//! `ccswitch ipc <command> [json]`：直接调用任意桥接命令。
//!
//! 这是「命令表里 413 条命令都能用」的逃生口，也是 CLI 新子命令没接上时的临时手段。

use serde_json::Value;

use super::super::{Context, Parsed, Payload};
use super::call;

pub fn run(ctx: &Context, parsed: &Parsed) -> Result<Payload, String> {
    let command = parsed
        .word(1)
        .ok_or_else(|| "缺少命令名：ccswitch ipc <command> [json]".to_string())?;
    let raw_args = parsed.rest(2).join(" ");
    let args = if raw_args.trim().is_empty() {
        Value::Null
    } else {
        serde_json::from_str::<Value>(&raw_args)
            .map_err(|error| format!("参数不是合法 JSON：{error}"))?
    };
    let data = call(ctx, command, args)?;
    Ok(Payload::Value(data))
}
