//! `ccswitch` 命令行入口。
//!
//! 真正的实现在 `cc_switch_lib::cli`：链接同一个库，保证 CLI 与桌面版用的是同一套
//! 命令实现、同一份数据库和服务。

fn main() {
    std::process::exit(cc_switch_lib::cli::main());
}
