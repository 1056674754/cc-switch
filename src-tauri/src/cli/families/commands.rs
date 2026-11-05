//! `ccswitch commands`：列出可调用的命令名。

use super::super::{Context, Parsed, Payload, Table};

pub fn run(ctx: &Context, _parsed: &Parsed) -> Result<Payload, String> {
    let commands = ctx.client.commands()?;
    let mut table = Table::new(&["command"]);
    for command in commands {
        table.push(vec![command]);
    }
    Ok(Payload::Table(table))
}
