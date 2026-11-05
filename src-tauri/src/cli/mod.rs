//! `ccswitch` 命令行的核心：参数解析、输出格式、子命令分派。
//!
//! 执行方式有两种，见 [`client::Client`]：
//!
//! 1. **附加**（默认）：桌面版在跑时，读它的 `control-server.json`，把命令 POST 给桌面
//!    进程执行。这样数据库/内存状态/live 配置只有一个写者，界面还能实时刷新。
//! 2. **独立**：桌面版没在跑时，CLI 自己以 headless 方式起一次应用、执行一条命令、退出，
//!    和桌面版下次启动读到的是同一份数据库。
//!
//! 子命令的实现都在 [`families`]，它们只做「拼参数 + 调 `dispatch` 命令名 + 排版输出」。

pub mod client;
pub mod families;
pub mod once;

pub use client::Client;

use std::collections::HashMap;

use serde_json::Value;

/// 正常结束。
pub const EXIT_OK: i32 = 0;
/// 命令执行失败（业务错误）。
pub const EXIT_FAIL: i32 = 1;
/// 用法错误（参数不对、未知子命令）。
pub const EXIT_USAGE: i32 = 2;

/// 带 `--value` 的选项；其余已知选项按布尔处理。
const VALUE_FLAGS: &[&str] = &[
    "app", "id", "name", "file", "key", "value", "source", "repo", "url", "token", "dir", "path",
    "type", "kind", "mode", "model", "profile", "session", "skill", "limit", "since", "until",
    "port", "format", "out", "group", "scope", "base-url", "gateway", "command", "app-type",
    "provider", "status", "add-live",
];

/// 布尔选项。
const BOOL_FLAGS: &[&str] = &[
    "json",
    "standalone",
    "help",
    "quiet",
    "verbose",
    "all",
    "force",
    "dry-run",
    "current",
    "resnapshot",
    "version",
    "no-color",
    "yes",
];

/// 解析后的命令行。
#[derive(Debug, Default)]
pub struct Parsed {
    /// 非选项参数，原样保留：words[0] 是子命令组，后面由各 family 解释。
    pub words: Vec<String>,
    pub flags: HashMap<String, String>,
}

impl Parsed {
    /// 子命令组，例如 `provider`。
    pub fn group(&self) -> &str {
        self.words.first().map(String::as_str).unwrap_or("")
    }

    /// 第二个词（通常是动词），例如 `provider list` 里的 `list`。
    pub fn verb(&self) -> Option<&str> {
        self.words.get(1).map(String::as_str)
    }

    pub fn word(&self, index: usize) -> Option<&str> {
        self.words.get(index).map(String::as_str)
    }

    /// 从第 `index` 个词开始剩下的所有参数。
    pub fn rest(&self, index: usize) -> &[String] {
        self.words.get(index..).unwrap_or(&[])
    }

    pub fn has_flag(&self, name: &str) -> bool {
        self.flags.contains_key(name)
    }

    pub fn flag(&self, name: &str) -> Option<&str> {
        self.flags.get(name).map(String::as_str)
    }

    /// 必填选项。
    pub fn require_flag(&self, name: &str) -> Result<&str, String> {
        self.flag(name)
            .ok_or_else(|| format!("缺少选项 --{name}（{})", usage_hint(self.group())))
    }

    /// 必填位置参数。
    pub fn require_word(&self, index: usize, label: &str) -> Result<&str, String> {
        self.word(index)
            .ok_or_else(|| format!("缺少参数 <{label}>（{}）", usage_hint(self.group())))
    }

    /// `--app` 或第一个位置参数（用于 `provider use claude xxx` 这种写法）。
    pub fn app(&self, fallback_index: usize) -> Result<String, String> {
        if let Some(app) = self.flag("app") {
            return normalize_app(app);
        }
        let word = self.require_word(fallback_index, "app")?;
        normalize_app(word)
    }
}

fn normalize_app(value: &str) -> Result<String, String> {
    use std::str::FromStr;
    crate::app_config::AppType::from_str(value)
        .map(|app| app.as_str().to_string())
        .map_err(|error| error.to_string())
}

/// 输出形状。
pub enum Payload {
    /// JSON 数据；文本模式下也按 pretty JSON 打印。
    Value(Value),
    /// 表格；`--json` 时转成对象数组。
    Table(Table),
    /// 已经排好版的多行文本。
    Text(String),
    /// 一串单项列表：文本模式一行一条，`--json` 模式输出数组。
    Lines(Vec<String>),
}

/// 简单表格。
#[derive(Default)]
pub struct Table {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

impl Table {
    pub fn new(headers: &[&str]) -> Self {
        Self {
            headers: headers.iter().map(|h| h.to_string()).collect(),
            rows: Vec::new(),
        }
    }

    pub fn push(&mut self, row: Vec<String>) {
        self.rows.push(row);
    }
}

/// 子命令执行上下文。
pub struct Context {
    pub client: Client,
    pub json: bool,
    pub quiet: bool,
}

/// CLI 入口。返回进程退出码。
pub fn main() -> i32 {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let parsed = match parse(&argv) {
        Ok(parsed) => parsed,
        Err(error) => {
            eprintln!("ccswitch: {error}");
            return EXIT_USAGE;
        }
    };

    if parsed.has_flag("version") || parsed.group() == "version" {
        println!(
            "ccswitch {} ({})",
            env!("CARGO_PKG_VERSION"),
            protocol_tag()
        );
        return EXIT_OK;
    }

    if parsed.has_flag("help") || parsed.words.is_empty() || parsed.group() == "help" {
        print!("{}", help_text(parsed.group()));
        return EXIT_OK;
    }

    let json = parsed.has_flag("json");
    let quiet = parsed.has_flag("quiet");
    let standalone = parsed.has_flag("standalone");

    // apps 是纯本地信息，不需要连桌面版，也不用提示“独立模式”。
    let connect_quiet = quiet || !group_needs_server(parsed.group());
    let client = match Client::connect(standalone, connect_quiet) {
        Ok(client) => client,
        Err(error) => {
            eprintln!("ccswitch: {error}");
            return EXIT_FAIL;
        }
    };

    let context = Context {
        client,
        json,
        quiet,
    };

    match families::run(&context, &parsed) {
        Ok(payload) => {
            print_payload(&payload, context.json);
            EXIT_OK
        }
        Err(error) => {
            eprintln!("ccswitch: {error}");
            EXIT_FAIL
        }
    }
}

/// 有些命令组只读本地常量/枚举，不需要和桌面版建立通道。
fn group_needs_server(group: &str) -> bool {
    !matches!(group, "apps")
}

fn protocol_tag() -> String {
    if once::is_one_shot() {
        "one-shot".to_string()
    } else {
        format!("protocol={}", crate::control::PROTOCOL)
    }
}

/// 解析全局选项。未知选项直接报错，避免 `--dryrun` 这种拼写错误被静默忽略。
pub fn parse(argv: &[String]) -> Result<Parsed, String> {
    let mut parsed = Parsed::default();
    let mut index = 0;
    while index < argv.len() {
        let token = argv[index].as_str();
        if let Some(rest) = token.strip_prefix("--") {
            if rest.is_empty() {
                index += 1;
                continue;
            }
            let (name, inline) = match rest.split_once('=') {
                Some((name, value)) => (name, Some(value.to_string())),
                None => (rest, None),
            };

            if BOOL_FLAGS.contains(&name) {
                parsed.flags.insert(name.to_string(), "true".to_string());
                index += 1;
                continue;
            }
            if VALUE_FLAGS.contains(&name) {
                let value = match inline {
                    Some(value) => value,
                    None => {
                        index += 1;
                        argv.get(index)
                            .cloned()
                            .ok_or_else(|| format!("选项 --{name} 缺少取值"))?
                    }
                };
                parsed.flags.insert(name.to_string(), value);
                index += 1;
                continue;
            }
            return Err(format!(
                "未知选项 --{name}（{}）",
                usage_hint(parsed.group())
            ));
        }
        if let Some(rest) = token.strip_prefix("-") {
            if rest.is_empty() {
                index += 1;
                continue;
            }
            // 只支持几个常用短选项，避免和位置参数混淆。
            match rest {
                "h" => {
                    parsed.flags.insert("help".into(), "true".into());
                }
                "j" => {
                    parsed.flags.insert("json".into(), "true".into());
                }
                "q" => {
                    parsed.flags.insert("quiet".into(), "true".into());
                }
                other => {
                    return Err(format!(
                        "未知选项 -{other}（{}）",
                        usage_hint(parsed.group())
                    ))
                }
            }
            index += 1;
            continue;
        }
        parsed.words.push(token.to_string());
        index += 1;
    }
    Ok(parsed)
}

fn print_payload(payload: &Payload, json: bool) {
    match payload {
        Payload::Value(value) => {
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(value).unwrap_or_default()
                );
            } else {
                print_text_value(value);
            }
        }
        Payload::Table(table) => {
            if json {
                let rows: Vec<Value> = table
                    .rows
                    .iter()
                    .map(|row| {
                        let mut object = serde_json::Map::new();
                        for (index, header) in table.headers.iter().enumerate() {
                            object.insert(
                                header.clone(),
                                row.get(index)
                                    .cloned()
                                    .map(Value::String)
                                    .unwrap_or(Value::Null),
                            );
                        }
                        Value::Object(object)
                    })
                    .collect();
                println!(
                    "{}",
                    serde_json::to_string_pretty(&rows).unwrap_or_default()
                );
            } else {
                print_table(table);
            }
        }
        Payload::Text(text) => {
            if json {
                // 纯文本结果（如「已删除」这类确认消息）在 --json 下统一包成对象，
                // 脚本至少能拿到 message 而不是一段无法解析的文本。
                let value = serde_json::json!({ "message": text });
                println!(
                    "{}",
                    serde_json::to_string_pretty(&value).unwrap_or_default()
                );
            } else {
                print!("{text}");
            }
        }
        Payload::Lines(lines) => {
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&lines).unwrap_or_default()
                );
            } else {
                for line in lines {
                    println!("{line}");
                }
            }
        }
    }
}

fn print_text_value(value: &Value) {
    match value {
        Value::Null => println!("null"),
        Value::String(text) => println!("{text}"),
        Value::Bool(flag) => println!("{flag}"),
        Value::Number(number) => println!("{number}"),
        other => println!(
            "{}",
            serde_json::to_string_pretty(other).unwrap_or_default()
        ),
    }
}

fn print_table(table: &Table) {
    if table.rows.is_empty() {
        println!("（无数据）");
        return;
    }
    let mut widths: Vec<usize> = table
        .headers
        .iter()
        .map(|header| display_width(header))
        .collect();
    for row in &table.rows {
        for (index, cell) in row.iter().enumerate() {
            if index < widths.len() {
                widths[index] = widths[index].max(display_width(cell));
            }
        }
    }

    let mut line = String::new();
    for (index, header) in table.headers.iter().enumerate() {
        if index > 0 {
            line.push_str("  ");
        }
        line.push_str(header);
        line.push_str(&" ".repeat(widths[index].saturating_sub(display_width(header))));
    }
    println!("{}", line.trim_end());

    for row in &table.rows {
        let mut line = String::new();
        for (index, cell) in row.iter().enumerate() {
            if index > 0 {
                line.push_str("  ");
            }
            line.push_str(cell);
            if index + 1 < row.len() {
                line.push_str(&" ".repeat(widths[index].saturating_sub(display_width(cell))));
            }
        }
        println!("{}", line.trim_end());
    }
}

fn display_width(text: &str) -> usize {
    text.chars().count()
}

fn usage_hint(group: &str) -> String {
    if group.is_empty() {
        "ccswitch --help".to_string()
    } else {
        format!("ccswitch {group} --help")
    }
}

/// 帮助文本。
pub fn help_text(group: &str) -> String {
    let groups = families::group_help();
    let mut out = String::new();
    out.push_str("ccswitch - CC Switch 命令行\n\n");
    out.push_str("用法: ccswitch <命令> [子命令] [参数] [--选项]\n\n");
    out.push_str("执行方式:\n");
    out.push_str("  桌面版在运行时，命令交给桌面进程执行，界面只按域刷新被改动的数据；\n");
    out.push_str("  否则 CLI 自己以 headless 方式执行一次（DB 与桌面版共用）。\n\n");
    out.push_str("全局选项:\n");
    out.push_str("  --json              以 JSON 输出（脚本用）\n");
    out.push_str("  --standalone        强制独立执行，不连桌面版\n");
    out.push_str("  -q, --quiet         不打印多余提示\n");
    out.push_str("  -h, --help          显示帮助\n\n");
    out.push_str("命令:\n");
    for (name, summary) in groups {
        out.push_str(&format!("  {name:<12} {summary}\n"));
    }
    out.push_str("\n示例:\n");
    out.push_str("  ccswitch status\n");
    out.push_str("  ccswitch provider list --app claude\n");
    out.push_str("  ccswitch provider use claude 3f2a1b\n");
    out.push_str("  ccswitch ipc get_settings '{}'\n");
    out.push_str("  ccswitch commands\n");

    if !group.is_empty() && group != "help" {
        if let Some(detail) = families::group_usage(group) {
            out.push_str(&format!("\n{group} 用法:\n{detail}"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn parse_args(args: &[&str]) -> Parsed {
        parse(&args.iter().map(|s| s.to_string()).collect::<Vec<_>>()).unwrap()
    }

    #[test]
    fn 位置参数和选项分开解析() {
        let parsed = parse_args(&["provider", "list", "--app", "claude", "--json"]);
        assert_eq!(parsed.group(), "provider");
        assert_eq!(parsed.verb(), Some("list"));
        assert_eq!(parsed.flag("app"), Some("claude"));
        assert!(parsed.has_flag("json"));
    }

    #[test]
    fn 支持等号写法和短选项() {
        let parsed = parse_args(&["provider", "list", "--app=codex", "-j"]);
        assert_eq!(parsed.flag("app"), Some("codex"));
        assert!(parsed.has_flag("json"));
    }

    #[test]
    fn 未知选项报错而不是静默忽略() {
        let error = parse(&["status".to_string(), "--dryrun".to_string()]).unwrap_err();
        assert!(error.contains("未知选项 --dryrun"), "{error}");
    }

    #[test]
    fn 缺取值的选项报错() {
        let error = parse(&["provider".to_string(), "--app".to_string()]).unwrap_err();
        assert!(error.contains("缺少取值"), "{error}");
    }

    #[test]
    fn 表格_json_输出按表头转对象() {
        let mut table = Table::new(&["id", "name"]);
        table.push(vec!["a".into(), "b".into()]);
        let payload = Payload::Table(table);
        assert!(matches!(payload, Payload::Table(_)));
        let value = json!({"id": "a", "name": "b"});
        assert_eq!(value["id"], "a");
    }
}
