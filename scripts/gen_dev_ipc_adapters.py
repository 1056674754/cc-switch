#!/usr/bin/env python3
"""生成 dev-ipc 的命令适配层。

浏览器开发模式（`--headless` + `feature = "dev-ipc"`）需要把 `generate_handler!` 里
注册的每条 Tauri 命令都暴露成可 JSON 调用的函数。手写不可能维护，所以从命令注册表
和各命令的真实签名生成：

1. `src-tauri/src/dev_ipc/adapters/<file>.rs`
   每个源文件一份，被该文件末尾的 `mod dev_ipc_adapters { include!(...) }` 引入。
   因为 include 进的是命令函数所在的模块，参数/返回类型不用做任何路径解析——直接用
   父模块的 import 就行。这是选 include 方案而不是生成独立模块的原因。
2. `src-tauri/src/commands/dev_ipc_table.rs`
   命令名 → 适配函数的静态分发表 + 命令名单 + 漂移守卫测试。放在 `commands` 模块里是
   必须的：它要访问 `commands::<file>` 这些私有子模块，换成别的位置就得把它们提升成
   `pub(crate)`。

覆盖的不只是 `commands::*`，也包括 `tray::*`、`tray_panel::*` 和注册在 `lib.rs` 里的
命令（早期版本只扫 `commands::` 前缀，漏了 8 条，前端一启动就报未知命令）。

脚本幂等：注册表或签名变了重跑一次即可；有命令解析失败会直接报错。
用法：`python3 scripts/gen_dev_ipc_adapters.py`（在仓库根或任意目录都可以）。
"""

from __future__ import annotations

import json
import os
import re
import sys
from dataclasses import dataclass, field
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
SRC_TAURI = REPO_ROOT / "src-tauri"
SRC = SRC_TAURI / "src"
LIB_RS = SRC / "lib.rs"
ADAPTERS_DIR = SRC / "dev_ipc" / "adapters"
TABLE_RS = SRC / "commands" / "dev_ipc_table.rs"

ADAPTER_MARKER_START = "// >>> dev-ipc adapters（生成物：scripts/gen_dev_ipc_adapters.py，勿手改）"
ADAPTER_MARKER_END = "// <<< dev-ipc adapters"


@dataclass
class Param:
    name: str
    raw_type: str
    kind: str  # "state" | "app" | "window" | "value" | "option"
    inner_type: str = ""  # state: managed type; option: T of Option<T>
    key: str = ""


@dataclass
class Command:
    name: str
    source: Path
    module: str  # 模块路径，crate root 的命令为空串
    is_async: bool
    returns_result: bool
    params: list[Param] = field(default_factory=list)
    error_type: str = ""
    unsupported: str = ""
    unsupported_kind: str = ""  # "window"（降级为 no-op）| "channel"/"response"（报错）


def strip_comments(text: str) -> str:
    """去掉 // 行注释和 /* */ 块注释——参数列表里常有人在参数后面写说明。"""
    text = re.sub(r"/\*.*?\*/", "", text, flags=re.S)
    text = re.sub(r"//[^\n]*", "", text)
    return text


def split_top_level(text: str, separator: str = ",") -> list[str]:
    """按顶层分隔符切分，忽略 <> () [] 内部的分隔符。"""
    parts: list[str] = []
    depth = 0
    current: list[str] = []
    for char in text:
        if char in "<([":
            depth += 1
        elif char in ">)]":
            depth -= 1
        if char == separator and depth == 0:
            parts.append("".join(current))
            current = []
        else:
            current.append(char)
    tail = "".join(current)
    if tail.strip() or parts:
        parts.append(tail)
    return [part.strip() for part in parts if part.strip()]


def find_matching(text: str, start: int, opening: str, closing: str) -> int:
    """返回从 start（指向 opening）开始匹配到的 closing 下标；找不到返回 -1。"""
    depth = 0
    index = start
    while index < len(text):
        char = text[index]
        if char == opening:
            depth += 1
        elif char == closing:
            depth -= 1
            if depth == 0:
                return index
        index += 1
    return -1


def camel_case(name: str) -> str:
    """Rust 参数名 → 前端传的 camelCase key（Tauri 默认 argument_case = camel）。"""
    if "_" not in name:
        return name
    head, *tail = name.split("_")
    return head + "".join(part[:1].upper() + part[1:] for part in tail)


def state_inner(state_type: str) -> str:
    """`State<'_, AppState>` → `AppState`。"""
    inner = state_type[state_type.index("<") + 1 : state_type.rindex(">")]
    parts = split_top_level(inner)
    return parts[1] if len(parts) > 1 else inner


def parse_param(text: str, rename_all_snake: bool) -> Param:
    # 去掉参数上的属性，如 #[allow(non_snake_case)]；属性里没有顶层冒号，先整段剥掉
    while True:
        stripped = text.lstrip()
        if stripped.startswith("#["):
            end = find_matching(stripped, 1, "[", "]")
            if end < 0:
                break
            text = stripped[end + 1 :]
        else:
            break
    text = text.strip()
    if ":" not in text:
        raise ValueError(f"无法解析参数：{text!r}")
    name, raw_type = text.split(":", 1)
    name = name.strip()
    if name.startswith("mut "):
        name = name[4:].strip()
    raw_type = raw_type.strip()

    if "State<" in raw_type:
        return Param(
            name=name, raw_type=raw_type, kind="state", inner_type=state_inner(raw_type)
        )
    if "AppHandle" in raw_type:
        return Param(name=name, raw_type=raw_type, kind="app")
    if raw_type.endswith("Window") and "WebviewWindow" not in raw_type:
        return Param(name=name, raw_type=raw_type, kind="window")
    key = name if rename_all_snake else camel_case(name)
    if raw_type.startswith("Option<") and raw_type.endswith(">"):
        return Param(
            name=name,
            raw_type=raw_type,
            kind="option",
            inner_type=raw_type[len("Option<") : -1].strip(),
            key=key,
        )
    return Param(name=name, raw_type=raw_type, kind="value", key=key)


def parse_return_type(return_type: str) -> tuple[bool, str]:
    """`Result<T, E>` → (True, "E")；T/E 可能带泛型，用顶层切分。"""
    compact = return_type.replace(" ", "")
    if not compact.startswith("Result<") and not compact.startswith(
        "std::result::Result<"
    ):
        return False, ""
    inner = compact[compact.index("<") + 1 : -1]
    parts = split_top_level(inner)
    return True, (parts[1] if len(parts) > 1 else "")


def parse_file(path: Path, module: str) -> dict[str, Command]:
    """解析一个源文件里所有 #[tauri::command] 函数的签名。"""
    text = path.read_text(encoding="utf-8")
    commands: dict[str, Command] = {}
    index = 0
    while True:
        attr_at = text.find("#[tauri::command", index)
        if attr_at < 0:
            break
        attr_end = find_matching(text, attr_at + 1, "[", "]")
        if attr_end < 0:
            raise ValueError(f"{path}: 无法定位 #[tauri::command] 的结尾")
        rename_all_snake = "snake_case" in text[attr_at : attr_end + 1]
        cursor = attr_end + 1

        # 跳过命令属性后的其它属性（#[allow(...)] 等）
        while True:
            match = re.match(r"\s*#\[", text[cursor:])
            if not match:
                break
            other_start = cursor + match.start() + len(match.group(0)) - 2
            other_end = find_matching(text, other_start + 1, "[", "]")
            if other_end < 0:
                raise ValueError(f"{path}: 无法定位命令后续属性的结尾")
            cursor = other_end + 1

        signature = re.match(
            r"(?:(?:\s*//[^\n]*\n)|\s)*"
            r"(?:pub(?:\([^)]*\))?\s+)?(?P<is_async>async\s+)?fn\s+(?P<name>\w+)"
            r"\s*(?:<[^<>]*>)?\s*",
            text[cursor:],
        )
        if not signature:
            raise ValueError(f"{path}: #[tauri::command] 后不是函数定义（offset={cursor}）")
        name = signature.group("name")
        is_async = bool(signature.group("is_async"))
        paren_at = cursor + signature.end()
        if text[paren_at] != "(":
            raise ValueError(f"{path}: 命令 {name} 缺少参数列表括号")
        paren_end = find_matching(text, paren_at, "(", ")")
        if paren_end < 0:
            raise ValueError(f"{path}: 命令 {name} 的参数列表没有闭合")
        params = [
            parse_param(chunk, rename_all_snake)
            for chunk in split_top_level(strip_comments(text[paren_at + 1 : paren_end]))
        ]

        returns_result = False
        error_type = ""
        unsupported = ""
        unsupported_kind = ""
        if text[paren_end + 1 :].lstrip().startswith("->"):
            arrow = text.index("->", paren_end) + 2
            return_type = text[arrow : text.index("{", arrow)].strip()
            returns_result, error_type = parse_return_type(return_type)
            if re.search(r"\bResponse\b", return_type):
                unsupported = "返回值是 tauri::ipc::Response（原始字节流），浏览器桥只支持 JSON"
                unsupported_kind = "response"
        for param in params:
            if "Channel<" in param.raw_type:
                unsupported = "参数里有 tauri::ipc::Channel（流式回调通道），浏览器桥不支持"
                unsupported_kind = "channel"
            if param.kind == "window":
                unsupported = "参数里有 tauri::Window，headless 模式没有原生窗口，降级为 no-op"
                unsupported_kind = "window"

        if name in commands:
            raise ValueError(f"{path}: 命令名重复：{name}")
        commands[name] = Command(
            name=name,
            source=path,
            module=module,
            is_async=is_async,
            returns_result=returns_result,
            params=params,
            error_type=error_type,
            unsupported=unsupported,
            unsupported_kind=unsupported_kind,
        )
        index = paren_end
    return commands


def registered_entries() -> list[str]:
    """从 lib.rs 的 generate_handler! 里按顺序取注册项（可能是 `commands::x` 也可能是裸名）。"""
    text = LIB_RS.read_text(encoding="utf-8")
    start = text.index("generate_handler![")
    end = text.index("]);", start)
    entries: list[str] = []
    for line in text[start:end].splitlines()[1:]:
        line = line.strip()
        if not line or line.startswith("//"):
            continue
        entries.append(line.rstrip(",").strip())
    if not entries:
        raise ValueError("lib.rs 的 generate_handler! 里没有解析到命令")
    return entries


def parse_all() -> dict[str, Command]:
    """解析所有可能定义命令的源文件：crate root、src/*.rs、src/commands/*.rs。

    命令名在注册表里必须唯一（`generate_handler!` 也依赖这一点），所以按名字索引；
    模块路径同时记在 Command 里，生成 adapter 路径时用。
    """
    by_name: dict[str, Command] = {}
    seen: set[Path] = set()
    candidates = [LIB_RS, *sorted(SRC.glob("*.rs")), *sorted(SRC.glob("commands/*.rs"))]
    for path in candidates:
        if path in seen:
            continue
        seen.add(path)
        text = path.read_text(encoding="utf-8")
        if "#[tauri::command" not in text:
            continue
        module = "commands" if path.parent.name == "commands" else path.stem
        if path.parent.name == "commands":
            module = f"commands::{path.stem}"
        if path == LIB_RS:
            module = ""
        for name, command in parse_file(path, module).items():
            if name in by_name:
                raise SystemExit(
                    f"命令名重复：{name}（{by_name[name].source} 与 {path}）；注册表要求名字唯一"
                )
            by_name[name] = command
    return by_name


def resolve_command(entry: str, by_name: dict[str, Command]) -> Command:
    """注册项 → 命令定义，顺便校验注册写法里的模块和实际定义位置一致。"""
    path = entry[6:] if entry.startswith("crate::") else entry
    parts = path.split("::")
    name = parts[-1]
    module = "::".join(parts[:-1])
    command = by_name.get(name)
    if command is None:
        raise SystemExit(f"注册了 `{entry}`，但 src/ 下没解析到它的 #[tauri::command] 定义")
    if module and module != command.module:
        if not (module == "commands" and command.module.startswith("commands")):
            raise SystemExit(
                f"`{entry}` 的模块路径和定义位置不一致：定义在 {command.module or 'crate root'}"
            )
    return command


def main() -> int:
    entries = registered_entries()
    if len(set(entries)) != len(entries):
        duplicates = sorted({entry for entry in entries if entries.count(entry) > 1})
        raise SystemExit(f"generate_handler! 里有重复命令：{duplicates}")

    by_name = parse_all()

    # 注册项 → 命令定义，按源文件分组（一个文件一个 include 的适配层）
    grouped: dict[Path, list[Command]] = {}
    for entry in entries:
        command = resolve_command(entry, by_name)
        grouped.setdefault(command.source, []).append(command)

    ADAPTERS_DIR.mkdir(parents=True, exist_ok=True)
    written: list[Path] = []
    for source, file_commands in sorted(grouped.items()):
        target = ADAPTERS_DIR / f"{source.stem}.rs"
        target.write_text(render_adapters_file(file_commands, source), encoding="utf-8")
        written.append(target)
        insert_marker_block(source, os.path.relpath(target, source.parent))

    TABLE_RS.write_text(render_table(entries, grouped), encoding="utf-8")
    written.append(TABLE_RS)

    remove_stale_adapters()
    print(f"✓ 已桥接 {len(entries)} 条命令（{len(grouped)} 个源文件），写 {len(written)} 个生成文件")
    return 0


def error_expr(error_type: str) -> str:
    """把命令的 Err 变成字符串：String 原样，AppError 有 Display，其它用 Debug 兜底。"""
    if error_type in ("String", "std::string::String"):
        return "Err(error)"
    if error_type.endswith("AppError"):
        return "Err(error.to_string())"
    return 'Err(format!("{error:?}"))'


def render_adapter(command: Command) -> str:
    lines = ["#[allow(non_snake_case)]"]
    lines.append(
        f"pub(crate) async fn __ipc_{command.name}("
        "__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {"
    )
    if command.unsupported:
        lines.append("    let _ = (__app, __args);")
        if command.unsupported_kind == "window":
            # 没有原生窗口时这类命令本来就没有事可做；报错只会让 UI 反复弹错，no-op 更贴近语义。
            lines.append("    crate::dev_ipc::ok(serde_json::Value::Null)")
        else:
            lines.append(
                f'    Err(format!("dev-ipc: 命令 `{command.name}` 无法在浏览器开发模式下调用：'
                f'{command.unsupported}"))'
            )
        lines.append("}")
        return "\n".join(lines)

    call_args: list[str] = []
    for param in command.params:
        local = f"__p_{param.name}"
        if param.kind == "state":
            lines.append(f"    let {local} = __args.state::<{param.inner_type}>(__app)?;")
        elif param.kind == "app":
            lines.append(f"    let {local}: tauri::AppHandle = __app.clone();")
        elif param.kind == "option":
            lines.append(
                f"    let {local}: {param.raw_type} = "
                f"__args.option::<{param.inner_type}>({json.dumps(param.key)})?;"
            )
        else:
            lines.append(
                f"    let {local}: {param.raw_type} = "
                f"__args.required::<{param.raw_type}>({json.dumps(param.key)})?;"
            )
        call_args.append(local)

    call = f"{command.name}({', '.join(call_args)})"
    await_suffix = ".await" if command.is_async else ""
    lines.append(f"    let __result = {call}{await_suffix};")
    if command.returns_result:
        lines.append(
            "    match __result { Ok(value) => crate::dev_ipc::ok(value), "
            f"Err(error) => {error_expr(command.error_type)} }}"
        )
    else:
        lines.append("    crate::dev_ipc::ok(__result)")
    lines.append("}")
    return "\n".join(lines)


def render_adapters_file(commands: list[Command], source: Path) -> str:
    header = [
        "// 由 scripts/gen_dev_ipc_adapters.py 生成，请勿手改。",
        f"// 源文件：{source.relative_to(REPO_ROOT)}（共 {len(commands)} 条命令）",
        "",
        "use super::*;",
        "",
    ]
    body = "\n\n".join(render_adapter(command) for command in commands)
    return "\n".join(header) + body + "\n"


def render_table(entries: list[str], grouped: dict[Path, list[Command]]) -> str:
    by_module_name = {
        (command.module, command.name): command
        for file_commands in grouped.values()
        for command in file_commands
    }

    def adapter_path(entry: str) -> str:
        path = entry[6:] if entry.startswith("crate::") else entry
        parts = path.split("::")
        name = parts[-1]
        module = "::".join(parts[:-1])
        command = by_module_name.get((module, name))
        if command is None:
            # 注册写法是 `commands::<fn>` 这种省略子模块的形式，按名字反查
            command = next(
                item for (_, item_name), item in by_module_name.items() if item_name == name
            )
        base = "crate" if not command.module else f"crate::{command.module}"
        return f"{base}::dev_ipc_adapters::__ipc_{name}"

    arms = [
        f'        "{entry.rsplit("::", 1)[-1]}" => {adapter_path(entry)}(__app, &mut __args).await,'
        for entry in entries
    ]
    names = ",\n    ".join(json.dumps(entry.rsplit("::", 1)[-1]) for entry in entries)
    return f'''// 由 scripts/gen_dev_ipc_adapters.py 生成，请勿手改。
// 命令清单来自 src-tauri/src/lib.rs 的 generate_handler!，与桌面端注册表一一对应。

use tauri::AppHandle;

/// 已桥接的命令名（顺序与 generate_handler! 一致）。
pub(crate) const COMMANDS: &[&str] = &[
    {names},
];

/// 已桥接的命令数量。
pub(crate) const COMMAND_COUNT: usize = {len(entries)};

/// 命令名 → 适配函数。参数走和 Tauri IPC 相同的 camelCase key。
pub(crate) async fn dispatch(
    command: &str,
    __app: &AppHandle,
    __args: serde_json::Value,
) -> crate::dev_ipc::Outcome {{
    let mut __args = crate::dev_ipc::Args::new(__args)?;
    match command {{
{chr(10).join(arms)}
        other => Err(format!(
            "dev-ipc: 未知命令 `{{other}}`（已桥接 {{COMMAND_COUNT}} 条）"
        )),
    }}
}}

#[cfg(test)]
mod tests {{
    use super::*;

    #[test]
    fn 命令清单不重复() {{
        let mut names = COMMANDS.to_vec();
        names.sort_unstable();
        let original = names.len();
        names.dedup();
        assert_eq!(original, names.len(), "COMMANDS 里有重复命令");
        assert_eq!(COMMANDS.len(), COMMAND_COUNT);
    }}

    /// 漂移守卫：generate_handler! 里增删命令后忘了重新生成时，这里会挂。
    #[test]
    fn 与_generate_handler_注册表一致() {{
        let lib_rs = include_str!("../lib.rs");
        let start = lib_rs
            .find("generate_handler![")
            .expect("lib.rs 里应该有 generate_handler!");
        let end = lib_rs[start..]
            .find("]);")
            .map(|offset| offset + start)
            .expect("generate_handler! 应该有结尾");
        let registered: Vec<&str> = lib_rs[start..end]
            .lines()
            .skip(1)
            .map(|line| line.trim())
            .filter(|line| !line.is_empty() && !line.starts_with("//"))
            .map(|line| line.trim_end_matches(','))
            .map(|entry| entry.rsplit("::").next().unwrap_or(entry))
            .collect();
        assert_eq!(
            registered, COMMANDS,
            "命令清单和 generate_handler! 不一致：请重跑 scripts/gen_dev_ipc_adapters.py"
        );
        assert_eq!(registered.len(), COMMAND_COUNT);
    }}
}}
'''


def strip_marker_block(text: str) -> str:
    pattern = re.compile(
        re.escape(ADAPTER_MARKER_START) + r".*?" + re.escape(ADAPTER_MARKER_END) + r"\n?",
        re.S,
    )
    return pattern.sub("", text).rstrip() + "\n"


def insert_marker_block(path: Path, adapter_relpath: str) -> None:
    """在命令所在源文件末尾挂上 include 适配层的模块（幂等）。"""
    text = strip_marker_block(path.read_text(encoding="utf-8"))
    block = f'''
{ADAPTER_MARKER_START}
#[cfg(feature = "dev-ipc")]
#[allow(clippy::all, dead_code, non_snake_case, unused_imports)]
pub(crate) mod dev_ipc_adapters {{
    include!("{adapter_relpath}");
}}
{ADAPTER_MARKER_END}
'''
    path.write_text(text + block, encoding="utf-8")


def remove_stale_adapters() -> None:
    """清掉旧方案遗留在 src/commands/dev_ipc/ 下的生成物，避免两份适配层并存。"""
    stale = SRC / "commands" / "dev_ipc"
    if not stale.exists():
        return
    for path in sorted(stale.rglob("*"), reverse=True):
        if path.is_file():
            path.unlink()
        else:
            path.rmdir()
    stale.rmdir()


if __name__ == "__main__":
    sys.exit(main())
