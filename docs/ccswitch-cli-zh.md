# ccswitch 命令行（CLI）

`ccswitch` 用来在终端里管理 CC Switch：切供应商、改设置、管 MCP / Skills / 提示词 / 档案，
以及脚本化地读状态、做备份。它和桌面版共用同一份实现、同一份数据库，**不是**第二个写者。

```bash
ccswitch status
ccswitch provider list --app claude
ccswitch provider use claude 3f2a1b
ccswitch mcp list
ccswitch ipc get_settings '{}'
```

## 为什么不是“再写一个独立 CLI”

如果 CLI 自己读写 `~/.cc-switch/config.json` 和各家 live 配置，就会有两个写者：

- CLI 和桌面版互相覆盖对方的修改；
- 桌面版界面看不到 CLI 改了什么，CLI 也可能读到桌面版还没落盘的旧状态；
- live 配置（`~/.claude/settings.json` 等）由两个进程同时改，容易写出半截状态。

所以 `ccswitch` 的定位是**驱动正在运行的桌面版**；桌面版没在跑时，CLI 自己以 headless 方式
把同一套应用启动一次、执行一条命令、退出。两条路径最终都落到同一个
`crate::commands::command_table::dispatch`。

## 两种执行模式

| 模式 | 触发条件 | 谁在写 | 界面 |
| --- | --- | --- | --- |
| **附加**（默认） | 找到运行中桌面版的控制服务（`<app config dir>/control-server.json` 能探活） | 桌面版进程 | 立即按域刷新对应数据 |
| **独立** | 桌面版没在跑，或显式 `--standalone` | CLI 自己 headless 起一次应用 | 不涉及（执行完就退出，桌面版下次启动读同一份库） |

- 附加模式单条命令约 10ms；独立模式要启动一次应用，热启动约 0.1–0.3s，冷启动（第一次跑 debug 构建）可能几秒。
- 独立执行和桌面版共用数据库与服务代码，所以迁移、设置读取、live 配置写入的行为完全一致。
- 如果桌面版在跑但控制端口不可用（旧版本，或设置了 `CC_SWITCH_NO_CONTROL=1`），CLI 会独立执行
  并在 stderr 提醒：这次修改界面可能看不到。

## 界面数据刷新：只刷新改动的数据域

CLI 的写命令在桌面版里执行完后，控制服务会 emit `cli-command-applied`；前端**不重载页面、
不重新挂载组件**，只失效这条命令影响的那个数据域：

- payload：`{ command, module, app, ok }`，`module` 是命令所属源文件（如 `provider`、`mcp`），
  `app` 是命令参数里的应用（如 `claude`）；
- Rust 侧只对**写命令**发事件（`control::is_read_only_command`），`ccswitch status` 这类只读查询
  一次事件都不发；
- 前端 `src/lib/cliCommandRefresh.ts` 维护「模块 → 查询根」的映射，
  `src/hooks/useExternalChangeRefresh.ts` 收事件、150ms 防抖合并后按域 `invalidateQueries`；
- 领域事件（`provider-switched` / `profile-applied` / `webdav-sync-status-updated` 等）仍然由
  命令自身发出，CLI 路径和界面内操作走的是同一套刷新逻辑。

实测（headless core + 浏览器 UI，CDP 记录界面实际发出的 invoke）：

| 操作 | 界面重取 |
| --- | --- |
| `ccswitch status`（只读） | 0 次 |
| `ccswitch provider use claude …` | 只重取 providers 域（`get_providers` / `get_app_mode` / `get_current_provider` 等） |
| `ccswitch ipc resync_mcp_to_apps …`（当前页不是 MCP） | 0 次（不会顺手重拉 providers/设置/技能） |

页面没有发生 reload：CDP 里 marker 仍在、`performance.getEntriesByType("navigation").length` 不变。

> 反例：早期实现是收到事件就 `queryClient.invalidateQueries()`（无 key 过滤）。那会把当前页面
> 所有活跃查询都重拉一遍，只读命令也会触发，属于“整页数据重刷”而不是“刷新被改的数据”，已废弃。

## 安装

```bash
scripts/install-cli.sh                 # release 构建并软链到 ~/.local/bin/ccswitch
scripts/install-cli.sh /usr/local/bin  # 指定安装目录
```

或者手动构建：

```bash
cargo build --release --manifest-path src-tauri/Cargo.toml --bin ccswitch
```

## 用法约定

```bash
ccswitch <命令> [子命令] [参数] [--选项]
```

| 选项 | 说明 |
| --- | --- |
| `--json` | 机器可读输出（列表类会转成对象数组） |
| `--standalone` | 强制独立执行，不连桌面版 |
| `-q`, `--quiet` | 不打印“独立模式”之类的提示 |
| `-h`, `--help` | 帮助 |

退出码：`0` 成功；`1` 命令执行失败；`2` 用法错误。

## 命令一览

| 命令 | 说明 | 示例 |
| --- | --- | --- |
| `status` | 当前供应商、本地路由、连接状态 | `ccswitch status --json` |
| `apps` | 支持的应用标识 | `ccswitch apps` |
| `provider` | 供应商：列表 / 当前 / 切换 / 增删改 / live | `ccswitch provider use claude <id或名称>` |
| `mcp` | MCP 服务器：列表 / 增删 / 启停 / 同步 | `ccswitch mcp list --app claude` |
| `skill` | 技能：列表 / 安装 / 卸载 / 备份 / 同步 | `ccswitch skill list` |
| `settings` | 设置读取 / 按文件写入 | `ccswitch settings get --json` |
| `proxy` | 本地路由：状态 / 启停 / 接管 / 配置 | `ccswitch proxy status` |
| `gateway` | 网关聚合：目录 / 模型配置 / 接管与释放 | `ccswitch gateway catalog` |
| `profile` | 配置档案：列表 / 创建 / 应用 / 清除 | `ccswitch profile create --name work` |
| `prompt` | 提示词：列表 / 启用 / 删除 / 导入 | `ccswitch prompt list --app claude` |
| `usage` | 用量：汇总 / 趋势 / 请求日志 | `ccswitch usage summary --app claude` |
| `session` | 会话：列表 / 消息 / 删除 | `ccswitch session list` |
| `backup` | 数据库备份：列表 / 创建 / 恢复 | `ccswitch backup create` |
| `ipc` | 直接调用任意命令（逃生口） | `ccswitch ipc get_providers '{"app":"claude"}'` |
| `commands` | 列出全部可调用命令名 | `ccswitch commands --json` |

`ccswitch <命令> --help` 看该组的详细用法。

### 逃生口 `ipc`

`generate_handler!` 注册的 413 条命令都可以直接调，参数就是 Tauri 命令的 camelCase key：

```bash
ccswitch ipc get_providers '{"app":"claude"}'
ccswitch ipc save_settings '{"settings":{...}}'
ccswitch commands --json | jq '. | length'
```

有 4 条命令在桥接层降级（`stream_session_messages`、`start_mcp_connectivity_checks`、
`get_session_image` 需要 Channel/二进制响应；`set_window_theme` 需要原生窗口），`ipc` 调用它们会
返回可读错误。

## 安全

- 控制服务只监听 `127.0.0.1`，端口由系统随机分配；
- 每次启动生成 128 位随机 token，写在 `<app config dir>/control-server.json`，文件权限 `0600`；
- CLI 只在同机、同用户下能读到 token；请求必须带 `x-ccswitch-token`；
- 桌面版退出时删除自己的描述文件（token 匹配才删，不会误删新实例的）；
- 不需要控制服务时用 `CC_SWITCH_NO_CONTROL=1` 关闭（CLI 会退回独立执行并提示）。

## 扩展一个子命令

1. 在 `src-tauri/src/cli/families/<组>.rs` 里加分支，用
   `call(ctx, "命令名", serde_json::json!({ ... }))` 调命令；参数 key 以
   `src-tauri/src/ipc/adapters/<模块>.rs`（生成物）为准，返回值按 `Payload::{Value,Table,Text,Lines}` 排版；
2. 在 `families/mod.rs` 里补 dispatch 和 `group_usage`；
3. `generate_handler!` 增删命令或签名变化后重跑 `python3 scripts/gen_command_adapters.py`；
   `command_table` 里的漂移守卫测试会拦住忘记重跑的情况；
4. 新的命令模块如果会改界面数据，需要在 `src/lib/cliCommandRefresh.ts` 的模块映射里登记查询根，
   否则会走兜底（只刷新数据根）；
5. 写命令不需要额外注册：控制服务默认按“可能写”处理，新增只读命令记得核对
   `control::is_read_only_command`（避免无谓刷新）。

## 验证

```bash
# Rust：控制服务、只读判定、命令表漂移守卫
cargo test --lib --manifest-path src-tauri/Cargo.toml control::
cargo test --lib --manifest-path src-tauri/Cargo.toml command_table::

# 前端：按域刷新的映射与 hook
pnpm vitest run src/lib/cliCommandRefresh.test.ts src/hooks/useExternalChangeRefresh.test.ts
```

端到端（可选）：`pnpm dev:web` 起 headless core + 浏览器 UI，另开终端跑
`ccswitch provider use claude <id>`，浏览器里当前供应商应就地变化、页面不重载。
