# 浏览器开发模式（前后台分离）

把 cc-switch 的 React UI 跑在**普通浏览器**里，Rust 侧以 `--headless` 起同一个 core（不开窗口），
两者通过 loopback HTTP + WebSocket 通信。前端不需要改任何 `invoke()` 调用点。

## 架构

```
┌────────────────────────────┐        /ipc/<command>  (POST, JSON)       ┌──────────────────────────┐
│ 浏览器 UI                   │ ──►  vite dev server :3000  ──代理──►     │ headless core             │
│ React + 343 处 invoke()     │        /ipc/events      (WS)              │ 127.0.0.1:3999（仅 loopback）│
└────────────────────────────┘                                          └──────────────────────────┘
```

- 只有 `src/dev/webBridge.ts`（+ `src/dev/tauriStubs.ts`）是新增的传输层：dev 且非 Tauri 时
  polyfill `window.__TAURI_INTERNALS__`，让 `@tauri-apps/api` 原样工作：
  - `invoke(cmd, args)` → `POST /ipc/<cmd>`，body 是参数对象 JSON；
  - core 的 `emit` 事件 → `WS /ipc/events` 文本帧 `{"event","payload"}`；
  - `plugin:event|*` 和窗口/路径/对话框等 `plugin:*` 在前端本地实现，不打后端。
- 后端响应恒为 HTTP 200 + envelope：成功 `{"ok":true,"data":...}`，失败 `{"ok":false,"error":"..."}`。
- `GET /ipc/health` 返回 core 自报信息（version / platform / home / appConfigDir / commands）。
- 生产构建里 `import.meta.env.DEV === false`，桥不会安装，行为与现在完全一致。

## 桥接了哪些命令

`generate_handler!` 里注册的命令**全部**桥接（当前 413 条，含 `commands::*`、`tray::*`、
`tray_panel::*` 和注册在 `lib.rs` 里的命令），一个不漏：漏了的话 UI 一启动就会在 console 里
报「未知命令」。其中 4 条受 headless 限制：

| 命令                            | 原因                                        | 行为                             |
| ------------------------------- | ------------------------------------------- | -------------------------------- |
| `stream_session_messages`       | 参数是 `tauri::ipc::Channel`（流式回调）    | 返回可读错误，会话转录流降级     |
| `start_mcp_connectivity_checks` | 参数是 `tauri::ipc::Channel`                | 返回可读错误，MCP 连通性检测降级 |
| `get_session_image`             | 返回 `tauri::ipc::Response`（原始字节）     | 返回可读错误，会话图片预览降级   |
| `set_window_theme`              | 参数是 `tauri::Window`（headless 没有窗口） | 静默 no-op，返回 `null`          |

适配层是**生成**的，不要手写：

```bash
python3 scripts/gen_dev_ipc_adapters.py     # 改了 generate_handler! 或命令签名后重跑
```

它会读 `src/lib.rs` 的命令表 + 每个命令的真实签名，生成 `src-tauri/src/dev_ipc/adapters/*.rs`
和 `src-tauri/src/commands/dev_ipc_table.rs`，并在命令所在源文件末尾挂一个
`mod dev_ipc_adapters { include!(...) }`。因为 include 进的是命令函数所在的模块，参数/返回
类型不用做任何路径解析。

`commands::dev_ipc_table` 里有一个漂移守卫测试：`generate_handler!` 增删命令但忘了重跑生成器时，
`cargo test --lib --features dev-ipc dev_ipc` 会直接失败。

## 启动

```bash
pnpm dev:web      # 一条命令：没 core 就后台起，然后前台跑 vite
```

或者分开两个终端：

```bash
pnpm dev:core     # 终端 1：headless core（首次 cargo 编译会比较久）
pnpm dev:ui       # 终端 2：vite，浏览器打开 http://localhost:3000
```

- `pnpm dev:core` = `cargo run --features dev-ipc -- --headless`，日志默认写 `/tmp/cc-switch-dev-core.log`。
- `pnpm dev:web` 发现 3999 已有 core 会直接复用，退出时只杀掉自己起的那个。
- `pnpm dev:ui` 与 `pnpm dev:renderer` 等价（都是 `vite`）。
- 两个脚本是 bash 脚本（macOS / Linux）；Windows 请在 Git Bash 或 WSL 里执行。

## 端口与数据隔离

| 环境变量                     | 默认值                        | 说明                                                                                                            |
| ---------------------------- | ----------------------------- | --------------------------------------------------------------------------------------------------------------- |
| `CC_DEV_IPC_PORT`            | `3999`                        | vite 代理的目标端口；`dev:web` 会把它同步给 core                                                                |
| `CC_SWITCH_DEV_IPC_PORT`     | `3999`                        | core 监听端口（只绑 127.0.0.1）                                                                                 |
| `CC_SWITCH_DEV_HOME`         | `/tmp/cc-switch-dev-home`     | 隔离的 home；core 的 appConfigDir 是 `$CC_SWITCH_DEV_HOME/.cc-switch`，不会碰真实的 `~/.cc-switch`              |
| `CC_SWITCH_DEV_SEED_DB`      | 未设置                        | 设为 `1` 时启动前用 `sqlite3 .backup` 把 `~/.cc-switch/cc-switch.db` 拷到隔离 home（没有 sqlite3 会警告并跳过） |
| `CC_SWITCH_DEV_CORE_LOG`     | `/tmp/cc-switch-dev-core.log` | `dev:web` 里 core 的日志路径                                                                                    |
| `CC_SWITCH_DEV_CORE_TIMEOUT` | `600`                         | `dev:web` 等 core 就绪的秒数                                                                                    |
| `VITE_DEV_IPC_BASE`          | 未设置                        | 直连模式：前端直接请求该地址（默认 `http://127.0.0.1:3999`），经 vite 代理时不要设置                            |

换端口时注意两边要一致：

- 一条命令：`CC_DEV_IPC_PORT=4000 pnpm dev:web`；
- 分开跑：`CC_SWITCH_DEV_IPC_PORT=4000 pnpm dev:core` + `CC_DEV_IPC_PORT=4000 pnpm dev:ui`。

直连模式（`VITE_DEV_IPC_BASE=...`）下浏览器会跨端口请求 core；core 对 loopback 开了 permissive
CORS，一般也能用，但正常开发还是建议走默认的同源代理（零 CORS、且 `/ipc/events` 的 WS 也一起代理）。

## 手动自测

浏览器 console 里有 `window.__CC_DEV_BRIDGE__`：

```js
window.__CC_DEV_BRIDGE__.baseUrl; // 当前 IPC 基地址（代理模式是 http://localhost:3000）
await window.__CC_DEV_BRIDGE__.getHealth(); // core 的 version / home / appConfigDir
window.__CC_DEV_BRIDGE__.version; // health 里报告的版本
```

也可以直接 `curl http://127.0.0.1:3999/ipc/health` 确认 core 状态，`curl
http://127.0.0.1:3999/ipc/commands` 列出当前桥接的所有命令名。

## 浏览器模式下降级的能力

| 能力               | 表现                                                                                        |
| ------------------ | ------------------------------------------------------------------------------------------- |
| 原生文件对话框     | `open`/`save` 直接按「用户取消」返回 `null`；`message`/`ask`/`confirm` 自动关闭并打 console |
| 更新器             | `check` 返回 `null`（永远没有更新），下载/安装抛「更新器在浏览器开发模式下不可用」          |
| `Channel` 流式接口 | HTTP 单次请求没有 channel 通道，MCP 连通性检测、会话转录流等会失败或降级，属预期            |
| 托盘面板           | `src/tray-panel/**` 不加载，托盘导航事件不会到来                                            |
| 原生窗口控制       | minimize/maximize/close/setDecorations 等是空实现；窗口尺寸、缩放、最大化状态返回固定值     |
| deep-link          | 不会触发，`DeepLinkImportDialog` 监听的事件永远不来                                         |
| 打开外部链接       | opener 用 `window.open`；reveal 本地路径只能打日志                                          |
| 前端日志           | `plugin-log` 改写 `console.*`，不再落到 Rust 日志文件                                       |
| 事件 emit          | 只在本窗口内回环，不会再广播给 Rust 侧/托盘                                                 |

## 常见问题

1. **core 没起 / 页面一直转圈**：`curl -sf http://127.0.0.1:3999/ipc/health`；看
   `/tmp/cc-switch-dev-core.log` 末尾。首次 `cargo run` 编译可能要几分钟，`dev:web` 会等它。
2. **端口被占用**：`lsof -nP -iTCP:3999 -sTCP:LISTEN` 找到占用进程；或用上面的方式换端口。
   vite 的 3000 端口是 `strictPort`，被占会直接退出。
3. **gateway 端口 15721 和正在跑的生产版冲突**：dev core 用隔离 home，不会主动去绑生产实例的
   gateway 端口；如果你在 dev 环境里手动开了 gateway/全局代理，两边就会抢 15721。处理方式：先退出
   生产版 cc-switch，或在 dev 设置里换成别的 gateway 端口，或清掉隔离 home 里的设置后重启 core。
4. **页面报 `IPC xxx 请求失败（headless core 没起？...）`**：core 挂了，或 vite 代理目标端口不对。
   重启 `pnpm dev:web`；桥会自己重连 `/ipc/events`，但 HTTP 请求不会自动重试。
5. **想用真实数据调试**：`CC_SWITCH_DEV_SEED_DB=1 pnpm dev:web`（需要 `sqlite3`）。注意只拷数据库，
   若设置里存的是绝对路径，仍会指向你真实目录。
6. **改了 core 端口但 UI 连不上**：`CC_DEV_IPC_PORT` 只影响 vite proxy，分开跑时两个变量都要改。
