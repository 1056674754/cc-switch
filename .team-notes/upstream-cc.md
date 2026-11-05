# 上游 origin/main（cc-switch v4.0.25）Claude Code（`AppType::Claude` / `"claude"`）配置实现

调查人：upstream-cc（task-1）。基线：`origin/main` = `7afceec1`（v4.0.25）。
读法：所有行号均为 `origin/main` 中的行号（用 `git archive origin/main` 展开到 /tmp 后 `grep -n` 得到，未 checkout 工作区）。
范围：**只覆盖 Claude Code（CC，`claude`）**，不涉及 Claude Desktop（`claude-desktop`，另有同事负责）。
本文不含本地 `diy` 分支改造差异；只描述上游现状。

---

## 0. 一句话结论

上游 CC 的模型是「**SSOT：DB 行 → 只投影关键字段（floor）+ 独有字段（exclusive）到一份 `~/.claude/settings.json`**」，
三层模式（直连 / 路由（本地代理接管）/ 聚合 Stack）都只是「同一份 settings.json 的不同投影」；
MCP 单独投影到 `~/.claude.json`，提示词写 `CLAUDE.md`，Skills 写 `~/.claude/skills`；
切换是「换一家生效」的切换式（非累加），只有 Stack 通过代理让多家模型共存。
CC 的「脏数据」判定全靠 `live/floor.rs` 的白/黑名单，没有快照回填。

---

## 1. 配置载体表（CC 读写哪些文件）

| 载体 | 路径 | 格式/内容 | 写入方 | 依据 |
|---|---|---|---|---|
| 主配置 | `~/.claude/settings.json`（旧安装兼容 `~/.claude/claude.json`；可被「Claude 配置目录覆盖」改根） | JSON 对象：顶层关键字段 + `env` 映射 | 写入引擎（`LiveFile::private` → 0600） | `src-tauri/src/config.rs:91`（`get_claude_config_dir`）、`config.rs:297-309`（settings/legacy）、`services/provider/claude_direct.rs:21-25` |
| MCP | `~/.claude.json` → `mcpServers`（覆盖目录时为 `<override>/.claude.json`） | JSON，只整体替换 `mcpServers` 键，其他键（含 projects）原样保留 | `claude_mcp::set_mcp_servers_map` | `config.rs:100-102`（`get_default_claude_mcp_path`）、`config.rs:281-292`（`derive_mcp_path_from_override`）、`claude_mcp.rs:321-380` |
| 提示词 | `~/.claude/CLAUDE.md` | Markdown 纯文本 | `PromptService`（启用项投影，互斥单激活） | `prompt_files.rs:21-52`（`AppType::Claude => "CLAUDE.md"` 在 :51）、`services/prompt.rs:1-45` |
| Skills | `~/.claude/skills/`（覆盖目录优先） | 目录（symlink/copy 由设置决定） | `SkillService` | `services/skill.rs:673-700`（Claude 分支 :675）、`services/skill.rs:734` |
| 插件/引导开关 | `~/.claude/config.json`（`primaryApiKey:"any"`，跳过第三方 Key 确认弹窗）、`~/.claude.json`（根上 `hasCompletedOnboarding:true`，跳过首次引导） | JSON，只增量改一个键 | `claude_plugin` / `claude_mcp`（**不**经写入引擎：`fs::write` / `atomic_write`） | `claude_plugin.rs:40-120`、`commands/plugin.rs:24-48`、`claude_mcp.rs:124-167` |
| 会话历史 | `~/.claude/projects/**/*.jsonl`（含 `subagents/`、`workflows/`） | JSONL | CC 自己写；CC Switch 只读/删/统计 | `session_manager/providers/claude.rs:38-44`、`services/session_usage.rs:3-8`、`session_usage.rs:496-560` |
| 官方登录凭据 | `~/.claude/.credentials.json` 或 macOS Keychain | 只读，用于订阅额度 | `subscription` 服务 | `services/subscription.rs:136-151`、`subscription.rs:230-248` |
| 多账号目录 | `<app_config_dir>/claude-accounts/<account>/config/`（软链 skills/commands/agents/output-styles/plugins/rules/workflows/projects/file-history/todos/plans；镜像 settings.json / .claude.json / CLAUDE.md） | 账号隔离用 | `claude_account::shared` | `services/claude_account/shared.rs:1-13`、`shared.rs:25-38` |
| 设备状态 | `~/.cc-switch/live-state.json`（**不跟随配置目录覆盖**） | 每 app 的 mode/attached/proxy_route/contract/stack/pending | 模式控制器 | `live/engine.rs:28-53`、`mode/state.rs:1-20` |
| 首写备份 | `~/.cc-switch/backups/live-first-write/<hash12>-<name>` | 每个文件**首次被 CC Switch 改写前**的字节，只备一次 | 写入引擎 | `live/engine.rs:52`、`live/engine.rs:161-181` |

### 备份与原子写（同一套机制，CC 无专属逻辑）

1. **原子写**：`write_json_file_private` → `atomic_write_private(...,0600)` → `stage_write`（写临时文件）→ `commit_staged`（rename 替换），见 `config.rs:436-470`。
   `settings.json` 里有 Key，所以走 private（Unix 0600）：`config.rs:442-446`。
2. **多文件操作 + 崩溃前滚**：一次切换 = 一个 `AppWrite`（应用写锁）里多个 `FileChange`，先把临时文件和写前/写后 hash 记进 `live-state.json` 的 `pending`，再逐个 publish；崩溃后下次操作/启动按 pending 前滚或丢弃，见 `mode/operation.rs:1-20`。
3. **乐观并发**：写前重读比对 hash，变了就以新内容为底重算（最多 3 次）；CC Switch 自己人之间用 `lock_app` 互斥，外部进程不参与 → 仍有极小窗口，见 `live/engine.rs:1-11`、`mode/operation.rs:33`（`MAX_REPLANS: usize = 3`）。
4. **没有 .bak 副本**：唯一备份是上面的「首写备份」，没有每次切换前备份 `settings.json`。
5. **注意边界**：只有 `settings.json`（和 Codex/Gemini/Grok 的 live 文件）走写入引擎；`~/.claude.json`（MCP、onboarding）用 `claude_mcp::write_json_value` → `config::atomic_write`（原子但无 0600、无 pending/首写备份），`~/.claude/config.json` 用裸 `fs::write`（非原子）：`claude_mcp.rs:89-96`、`claude_plugin.rs:76-84`。

---

## 2. Provider 数据模型

### 2.1 DB 表 `providers`（SQLite）

`src-tauri/src/database/schema.rs:27-43`：

```sql
id TEXT NOT NULL, app_type TEXT NOT NULL, name TEXT NOT NULL,
settings_config TEXT NOT NULL, website_url TEXT, category TEXT,
created_at INTEGER, sort_index INTEGER, notes TEXT, icon TEXT,
icon_color TEXT, meta TEXT NOT NULL DEFAULT '{}',
is_current BOOLEAN NOT NULL DEFAULT 0, in_failover_queue BOOLEAN NOT NULL DEFAULT 0,
PRIMARY KEY (id, app_type)
```

CC 的 `settings_config` 就是**一份 `settings.json` 的 JSON 对象**（可含 `env`、顶层键、hooks 等），
不是「只要 env」。Rust 结构：`provider.rs:10-45`（`settingsConfig` → `settings_config`，另有 `meta/icon/iconColor/inFailoverQueue`）。
`meta` = `ProviderMeta`（`provider.rs:487-643`），CC 相关字段：

| 字段 | 作用 | 依据 |
|---|---|---|
| `apiFormat` | `anthropic`（默认）/`openai_chat`/`openai_responses`/`gemini_native`：非 anthropic 时 CC 的请求走本地代理做协议转换 | `provider.rs:535-540` |
| `apiKeyField` | `ANTHROPIC_AUTH_TOKEN`（默认）或 `ANTHROPIC_API_KEY` | `provider.rs:553-555`、`provider.rs:103-108` |
| `isFullUrl` | base_url 视为完整端点，代理不再拼路径 | `provider.rs:556-558` |
| `claudeReasoningEffort` | CC→OpenAI 路径思考档位：缺省自动 / `off` / `passthrough` | `provider.rs:575-582` |
| `customUserAgent` / `localProxyRequestOverrides` | 仅本地代理生效的 UA / 请求覆盖 | `provider.rs:603-...` |
| `stackModels` | Stack 模式下这家发布哪些模型（`ClaudeStackModel{model,displayName,oneM}`） | `provider.rs:638-653`、`src/types.ts:243-255` |
| `authBinding` | 托管账号绑定（`claude_official` 账号 key 等） | `provider.rs:548-552`、`services/provider/claude_official.rs:26-47` |
| `commonConfigEnabled` | 旧版通用配置片段开关（新版基本不读） | `provider.rs:491-496` |

### 2.2 Provider 预设（前端 139 条，官方只有 1 条）

`src/config/claudeProviderPresets.ts:77`（数组起点），`ProviderPreset` 接口在 `:30-75`。统计：

| category | 条数 | 备注 |
|---|---|---|
| `aggregator` | 62 | 聚合站 |
| `cn_official` | 40 | 国产官方（Kimi/智谱/MiniMax…） |
| `third_party` | 24 | 第三方 |
| `cloud_provider` | 8 | AWS Bedrock / Vertex 等 |
| `local` | 4 | 本机推理（Ollama/LM Studio） |
| `official` | 1 | **Claude Official** |

官方预设 `claudeProviderPresets.ts:78-93`：

```ts
name: "Claude Official", settingsConfig: { env: {} }, isOfficial: true, category: "official",
```

即**官方 = 空 `env` 的行**：切过去 = 清空所有关键字段 → CC 回落自带登录（OAuth/keychain）。
后端还有幂等 seed（启动写入，id 固定）：`database/dao/providers_seed.rs:35-44`（`id: "claude-official"`，注释「空 env 让用户走 Claude CLI 默认认证流程」）。
非 anthropic 协议预设：`apiFormat` 出现 `openai_chat`×8、`openai_responses`×3、`gemini_native`×1。

### 2.3 校验规则（CC 极松）

`services/provider/mod.rs:7994-8006`：

```rust
AppType::Claude => {
    if !provider.settings_config.is_object() { return Err(... "Claude 配置必须是 JSON 对象" ...); }
}
```

即**只校验「是 JSON 对象」**；不校验 base_url / key 是否存在（对比 Codex 强制 `auth` 对象、Gemini/`validate_gemini_settings`、Grok Build 强制 TOML）。
保存前还会跑 CC 专属归一化 `normalize_provider_if_claude`（`:5377-5384`）→ `normalize_claude_models_in_value`（`:8399-8450`）：
缺 `ANTHROPIC_DEFAULT_{HAIKU,SONNET,OPUS}_MODEL` 时按 `自身 → ANTHROPIC_MODEL → ANTHROPIC_SMALL_FAST_MODEL` 回落补齐。

### 2.4 「当前供应商」有两个指针

`mode/current.rs:1-6`：

- **直连指针**（`Purpose::Direct`）：直连模式写进 live 的那家；代理模式下退出代理时写回它。落在 `~/.cc-switch` 的 settings 本地记录，DB `is_current` 只是回退值。
- **代理路由**（`Purpose::InUse`）：代理模式下真正收请求的那家。`mode/current.rs:71-79`。

CC 与 Desktop 的指针完全独立（CC 用 DB+live-state，Desktop 用 `configLibrary` profile）。

---

## 3. 切换时写入 `settings.json` 的字段（逐项 + 触发条件）

核心：`live/project/claude.rs` 的投影 + `live/floor.rs` 的名单；`direct_patch()`（`live/project/claude.rs:89-116`）是**唯一**的切换补丁，编辑器和真实切换共用。

```
direct_patch = {
  clear: [ 顶层按 claude_floor_top、env 按 claude_floor_env 全清 ],
  set:   目标行的 (top ∪ env ∪ exclusive) 原样写入,
  remove_if: [ 上一家的 exclusive（值未被改过才删）, residue 残留值 ],
}
```
（`live/project/claude.rs:89-116`，含 `ClearScope{parent: root/env, is_floor}`）

### 3.1 关键字段（floor）—— 一律「先清空、再写目标值」

**`env` 里按前缀**（`live/floor.rs:30`）：

```rust
pub const CLAUDE_FLOOR_ENV_PREFIXES: &[&str] = &["ANTHROPIC_", "AWS_", "VERTEX_REGION_"];
```

由此覆盖（**不限于**）：`ANTHROPIC_BASE_URL`、`ANTHROPIC_AUTH_TOKEN`、`ANTHROPIC_API_KEY`、
`ANTHROPIC_MODEL`、`ANTHROPIC_SMALL_FAST_MODEL`、`ANTHROPIC_DEFAULT_{OPUS,FABLE,SONNET,HAIKU}_MODEL`、
`ANTHROPIC_DEFAULT_*_MODEL_NAME`、`ANTHROPIC_DEFAULT_*_MODEL_DESCRIPTION`、`ANTHROPIC_CUSTOM_HEADERS`、
`ANTHROPIC_BETAS`、`AWS_REGION`/`AWS_PROFILE`/`AWS_BEARER_TOKEN_BEDROCK`、`VERTEX_REGION_*` 等。

**`env` 里按名字**（`live/floor.rs:33-45` + 判定 `:51-57`）：

```rust
"CLAUDE_CODE_SUBAGENT_MODEL", "CLAUDE_CODE_SUBAGENT_MODEL_FORCE", "CLOUD_ML_REGION",
"GOOGLE_APPLICATION_CREDENTIALS", "CLAUDE_CODE_OAUTH_TOKEN", "CLAUDE_CODE_OAUTH_REFRESH_TOKEN",
"CLAUDE_CODE_OAUTH_SCOPES", "CLAUDE_CODE_API_KEY_HELPER_TTL_MS",
// 另加前缀匹配：key.starts_with("CLAUDE_CODE_SKIP_") && key.ends_with("_AUTH")
```

**协议选择器**（不能按 `CLAUDE_CODE_USE_` 前缀，见注释 `floor.rs:13-17`；名单 `floor.rs:18-26`）：
`CLAUDE_CODE_USE_BEDROCK`、`_VERTEX`、`_FOUNDRY`、`_GATEWAY`、`_MANTLE`、`_ANTHROPIC_AWS`、`_ANTHROPIC_GOOGLE_CLOUD`。

**顶层关键字段**（`live/floor.rs:61-84`）：

```rust
"apiKeyHelper","apiBaseUrl","primaryModel","smallFastModel","apiKey","model",
"fallbackModel","modelOverrides","modelPicker","advisorModel",
"awsAuthRefresh","awsCredentialExport","gcpAuthRefresh",
```

触发条件与语义：

- **无条件清空再写**：目标行里没有的 floor 键就**消失**。所以「切到官方（空 env）」= 全部 `ANTHROPIC_*` 被删 → CC 用自带登录；「切到 Bedrock」= 其他家的 token/model 全清，只留 Bedrock 的。
- **`model` 也归供应商**：`/model` 里用户选过的值属于「当时那家」，切走即清（`floor.rs:68-69` 注释「属于当时所在的那一家」）。
- **Bedrock 迁移**：行顶层有旧式 `apiKey` 且 `env.CLAUDE_CODE_USE_BEDROCK` 为真时，投影成 `env.AWS_BEARER_TOKEN_BEDROCK`（行本身不改写），`live/project/claude.rs:66-79`；反向存回顶层见 `:135-198`。
- **`CLAUDE_CONFIG_DIR` 例外**：旧开发版写进 settings 的 `<数据目录>/claude-accounts/<id>/config` 值会被专门删掉（用户自己写的不动），`services/provider/claude_official.rs:46-64`。

### 3.2 独有字段（exclusive）—— 只删「上一家带进来且没被改过」的

`live/floor.rs:88-116`（12 个键，注释说明它们是上游兼容开关与窗口值）：

```rust
"CLAUDE_CODE_DISABLE_EXPERIMENTAL_BETAS","CLAUDE_CODE_DISABLE_ARTIFACT","ENABLE_TOOL_SEARCH",
"CLAUDE_CODE_DISABLE_THINKING","DISABLE_INTERLEAVED_THINKING","CLAUDE_CODE_ALWAYS_ENABLE_EFFORT",
"CLAUDE_CODE_EXTRA_BODY","CLAUDE_CODE_ENABLE_FINE_GRAINED_TOOL_STREAMING","CLAUDE_CODE_AUTO_MODE_SERVER",
"CLAUDE_CODE_MAX_CONTEXT_TOKENS","CLAUDE_CODE_AUTO_COMPACT_WINDOW","CLAUDE_CODE_MAX_OUTPUT_TOKENS",
"CLAUDE_CODE_DISABLE_1M_CONTEXT","CLAUDE_CODE_DISABLE_UNKNOWN_MODEL_WINDOW_ENFORCEMENT",
"CLAUDE_CODE_ENABLE_GATEWAY_MODEL_DISCOVERY",
```

- 切入写；切走时 `remove_if` 只在**值等于上一家写的值**时才删（用户改过就留），`live/project/claude.rs:92-101`。
- 再叠加**残留清理**：旧版整份写入/注入过的窗口值精确命中就删（`CLAUDE_CODE_MAX_CONTEXT_TOKENS`:262144/372000/983616；`AUTO_COMPACT_WINDOW`:262144/372000/1000000；`MAX_OUTPUT_TOKENS`:131072），`live/residue.rs:14-31`。
- 用户自己设的全局兼容开关（不在上一家记录里、也不命中残留）不会被误删。

### 3.3 代理模式（路由 / Stack）写入的字段（`proxy_projection`，`live/project/claude.rs:249-289`）

只写下面这些，其余 floor 键**全部清空**（否则 CC 会绕过本地代理）：

| 键 | 值 | 触发条件 | 依据 |
|---|---|---|---|
| `env.ANTHROPIC_BASE_URL` | `proxy_url`（`http://<listen_host>:<port>`） | 恒写 | `claude.rs:263-266`、`services/proxy.rs:251-273` |
| `env.ANTHROPIC_AUTH_TOKEN` | `PROXY_MANAGED` 占位符 | 非托管账号（普通第三方 Key） | `claude.rs:199`（常量）、`claude.rs:272-276` |
| `env.ANTHROPIC_API_KEY` **或** `ANTHROPIC_AUTH_TOKEN` | `PROXY_MANAGED` | 仅托管账号（Copilot/Codex/xAI）；Copilot 且表单显式选 API_KEY 才写 API_KEY，否则 AUTH_TOKEN | `claude.rs:221-236`、`mode/controller.rs:174-182` |
| `env.ANTHROPIC_DEFAULT_HAIKU_MODEL` | 稳定别名 `claude-haiku-5-5` | 路由模式 | `claude.rs:206`、`:465-480` |
| `env.ANTHROPIC_DEFAULT_SONNET_MODEL` | `claude-sonnet-5`（上游 5.5 → `claude-sonnet-5-5`） | 路由模式 | `claude.rs:207`、`:497-519` |
| `env.ANTHROPIC_DEFAULT_OPUS_MODEL` | `claude-opus-5`（5.5 → `claude-opus-5-5`） | 路由模式 | `claude.rs:208`、`:503-519` |
| `env.ANTHROPIC_DEFAULT_FABLE_MODEL` | `claude-fable-5` | 路由模式 | `claude.rs:209` |
| `_MODEL` 后缀 `[1M]` | 上游模型带 1M 标记时别名后加 `[1M]`（haiku 档不加） | 上游是 1M | `claude.rs:211`、`:469-473`、`:585-591` |
| `env.ANTHROPIC_DEFAULT_*_MODEL_NAME` | 显示名（行里 `_MODEL_NAME` 优先，否则去 `[1M]` 的上游名） | 显示名非空 | `claude.rs:473-477`、`:408-417` |
| `env.ANTHROPIC_DEFAULT_*_MODEL_DESCRIPTION` | `<供应商名> · <上游模型> · <窗口>` | 恒写（无行内覆盖时） | `claude.rs:375-390`、`:478-480` |
| `env.CLAUDE_CODE_SUBAGENT_MODEL` | 行里的值（5.5 上游换成 `claude-*-5-5` id） | 仅当行里配了 | `claude.rs:481-487`、`:529-543` |
| `exclusive` | 路由那家的兼容/窗口开关 | 恒（跟路由行） | `claude.rs:285-288` |
| 顶层 `modelPicker` | Stack 列表 `{replaceBuiltInOptions:true, options:[...]}` | **仅 Stack 且有发布模型** | `mode/controller.rs:221`、`:226-256` |
| `exclusive.CLAUDE_CODE_MAX_CONTEXT_TOKENS` | Stack 里非 1M 模型的最小窗口（=200000 时不写） | **仅 Stack** | `mode/controller.rs:244-255` |
| `exclusive` 里的成员兼容开关 | `ANY` 类取第一个「要关」的；`ALL` 类只有所有成员都开才开 | **仅 Stack 且成员>1** | `mode/controller.rs:260-331` |

四档别名取哪个上游模型（`route_roles`，`claude.rs:424-443`）：

```
haiku : 自己的 _HAIKU → ANTHROPIC_SMALL_FAST_MODEL → ANTHROPIC_MODEL
sonnet/opus : 自己的 → ANTHROPIC_MODEL → ANTHROPIC_SMALL_FAST_MODEL
fable : 只用自己的（没配就不写，映射侧会 fable→opus 降级）
```

Stack 模式下四档**全部**指向 `claude_route_default`（默认那家列表里的第一个模型），haiku 档也带 `[1M]`，
且不写 `CLAUDE_CODE_SUBAGENT_MODEL`（子代理跟随主模型），`live/project/claude.rs:544-570`。

### 3.4 editor_view「切换后配置预览」

- 命令：`get_provider_editor_view`（`src-tauri/src/commands/provider.rs:104-135`）→ `ProviderService::editor_view_with_meta`（`services/provider/mod.rs:5868-5900`），CC 直接委派 `claude_editor::view`。
- 语义（`services/provider/claude_editor.rs:1-8`）：**底部 JSON = 切到这个供应商之后 settings.json 的样子**，在内存里对当前 live 跑一次和切换**同一个补丁**：

```rust
let settings = project_onto(&file.path, &live, prev.as_ref(), &ClaudeProjection::of(settings_config))?;
```
（`claude_editor.rs:122-134`；`project_onto` 在 `live/project/claude.rs:118-131`，只 apply 不写盘）

- 编辑当前 / 非当前 / 新增供应商看到的都是「切换投影」；代理模式下也显示该行自己的关键字段值（前端 `EditProviderDialog.tsx:150-176`）。
- 新增 CC 卡片时先取一次 live 作底（`AddProviderDialog.tsx:131-167`，`appId !== "claude"` 才走表单自身投影）。
- 返回体还带 `inactive`（行里存着但不随切换生效的字段，如 hooks、`API_TIMEOUT_MS`），`claude_editor.rs:46-63`、`:136-167`。
- 保存：关键字段+独有字段写回行，其余改动（CC 全局设置）只把用户动过的键写进 live；三方比较冲突策略 `Refuse/KeepMine/KeepTheirs`（`claude_editor.rs:65-101`）。
- 前端「哪些应用有编辑器预览」：`EDITOR_VIEW_APP_IDS = ["claude","codex","gemini","grokbuild"]`（`src/config/appConfig.tsx:169-178`）。

---

## 4. 三种模式：直连 / 路由（proxy）/ Stack（聚合）

模式状态存 `~/.cc-switch/live-state.json`（设备本地、0600、不同步）：`mode/state.rs:1-20`。
枚举 `Mode::{Direct,Proxy}`（`state.rs:54-58`），`ModeState{mode,attached,proxy_route,contract}`（`state.rs:73-95`），
`StackState{enabled,members,keys}`（`state.rs:119-140`）。**Stack 不是第三种 Mode，而是 `Mode::Proxy + StackState.enabled`**（界面上和路由二选一）。

操作表（`mode/controller.rs:6-12`）：

| 操作 | 客户端文件 | 模式状态 |
|---|---|---|
| 进入 | 写代理契约（路由供应商的） | proxy, attached, 记 contract |
| 退出 | 写回直连指针的供应商 | direct；路由保留 |
| 分离（退出 CC Switch） | 同退出 | 模式/路由不变，未 attached |
| 接上（启动） | 按保存的路由写契约 | attached |
| 换路由 | 契约没变就不碰；变了先改写客户端 | 路由 + 契约 |

### 4.1 直连（direct）

- 切换 = `ProviderService::switch` → 非代理分支 → `switch_claude_direct`（`services/provider/mod.rs:7128-7148`）→ `claude_direct::switch_to`（`services/provider/claude_direct.rs:30-38`）：
  `direct_patch(prev, target)` 写 `settings.json` + 把直连指针改成 target，**同一 operation 提交**。
- 切完**幂等重投影 MCP**（`~/.claude.json`），失败只 warn（`provider/mod.rs:7143-7147`）。
- 官方卡绑了账号时，写文件前后会「换登录」（`claude_official::with_login_switch`，`claude_direct.rs:63-75`、`claude_official.rs:66-79`）。
- 用户行里与供应商无关的键（hooks、超时、插件…）**原样保留在 live**，CC Switch 不写不删。

### 4.2 路由（proxy / 本地网关接管）

- 进/退：`mode::controller::enter/exit`（`controller.rs:719`、`:863`）；启动接上：`startup`（`:1701`）。
- 写代理契约：`write_proxy` 的 `AppType::Claude` 分支（`controller.rs:401-432`）→ `claude_contract`（`:187-214`）→ `proxy_projection` → `claude_direct::run`。
- **代理模式下「切换供应商」= 只换路由，不动直连指针**（`services/provider/mod.rs:6924-6958`）；契约没变时连文件都不读不写（`contract.key` 摘要比较，`controller.rs:144-147`）。
- 代理模式下**禁止切到官方供应商**（防封号）：`provider/mod.rs:6928-6938`，错误 key `switch.official_blocked_by_proxy`；`official_provider_supports_proxy_takeover` 只对 Codex 官方为真（`provider/mod.rs:85-88`），所以 **CC 官方卡在代理模式下永远不能切**。
- 契约内容由 `contract::claude`（`mode/contract.rs:24-40`）摘要成 `key`，退出时按 `contract.exclusive` 删独有字段（`state.rs:61-71` 注释）。
- 故障转移（failover）只在代理模式生效：队列 = `providers.in_failover_queue`（`database/dao/failover.rs:1-40`），成功后切路由并刷托盘（`proxy/failover_switch.rs:1-50`）。Stack 模式**不做故障转移**（`mode/stack.rs:14-15`）。

### 4.3 Stack（聚合）

- 成员的第三方模型以保留前缀 id 发布给客户端：`ccs-claude-<key>--<model>`（`mode/stack.rs:30-37`）；id 里必须含 `claude` 才进 `/model` 选择器，且不以 `claude-` 开头 MAX 窗口才生效。
- key 一经分配永久归这家（登记簿 `StackState.keys`，移除/删除供应商也不回收）——避免旧 id 被悄悄发给别家（`stack.rs:1-16`、`state.rs:119-140`）。
- 默认那家（代理路由）也在名单里、不能移除；不带前缀的请求发往默认那家（`controller.rs:1235-1265`）。
- 官方账号**不能加入 Stack**（`controller.rs:1235-1250`）。
- 发布内容：`claude_published`（`stack.rs:564-569`）；四档别名指向 `claude_route_default`（`stack.rs:575-580`）；`modelPicker` 顶层替换内置菜单（`controller.rs:226-256`）。
- 空列表的成员「什么都不发布」；`meta.stackModels` 由用户显式指定（`provider.rs:638-653`、`src/types.ts:243-255`）。

### 4.4 生效是否需要重启

- **CC Switch 侧**：不需要重启，所有写入都是即时落盘 + 状态同事务。
- **托盘语义**：`switch_feedback`（`tray.rs:1524-1560`）对 **Claude Code 的直连 / 路由 / 故障转移换一家「立即生效、不提示重启」**；会提示「重启 {app} 后生效」的只有 Claude Desktop（直连/路由/聚合都提示）和 Codex / Gemini / Grok Build 的直连切换（`tray.rs:1524-1560`、文案 `:441-442`）。
- **客户端侧**：CC 的 `env` 是进程启动时读的（`services/provider/claude_official.rs:8-11` 的措辞「之后任何地方**新启动**的 `claude` 都是这个号」；`CLAUDE_CONFIG_DIR` 注释「Claude Code 只在**启动环境**里完整认它」）。因此已在跑的 `claude` 会话要重开才会用新地址/新 Key。**这一条是从代码注释推断的，上游 docs/ 没有明文「必须重启」**，报告里建议表述为「新启动的 claude 进程生效，无需重启 CC Switch」。

---

## 5. 支持范围（CC 视角）

| 能力 | CC 是否支持 | 说明与依据 |
|---|---|---|
| 切换 / 共存 | 切换式（非累加） | `is_additive_mode()` 不含 Claude（`app_config.rs:513-526`）；同一时刻只有一家生效。累加式是 OpenCode/OpenClaw/Hermes/Pi/Mcode/Kimi/Dsh/Omp。**Stack 是唯一的「多模型共存」途径**（经本地代理）。 |
| 项目 Profile | ✅ | `ProfileScope::Claude`（`services/profile.rs:34-60`）；payload 四槽：`providers/mcp/skills/prompts`（`profile.rs:131-140`），apply 复用 `ProviderService::switch`、`McpService::toggle_app`、`SkillService::toggle_app`、`PromptService::enable_prompt`（`profile.rs:1-17`）。 |
| MCP | ✅（仅 user scope `~/.claude.json`） | `claude_mcp.rs:321-380`；`McpApps.claude` 开关（`app_config.rs:19`、`:38-53`）；同步前检查 `~/.claude` 或 `~/.claude.json` 存在，未安装则不创建文件（`mcp/claude.rs:11-16`）；mcpPresets 见 `src/config/mcpPresets.ts`。不管理项目级 `.mcp.json`（上游未见）。 |
| Skills | ✅ | `~/.claude/skills`（`services/skill.rs:673-700`、`:734`），支持 symlink/copy、SSOT 目录、卸载备份。 |
| 提示词（CLAUDE.md） | ✅ | `prompt_files.rs:51`；多 Prompt 互斥，启用项覆盖写 `~/.claude/CLAUDE.md`（`services/prompt.rs:25-50`）。 |
| 会话管理 | ✅ | 读/列表/解析/删除 `~/.claude/projects/**/*.jsonl`（`session_manager/providers/claude.rs:38-64`），含子 agent 与 workflow 子目录。 |
| 用量统计 | ✅（四路来源） | ①官方订阅额度：keychain 或 `~/.claude/.credentials.json`（`services/subscription.rs:136-151`、`:230-248`）；②供应商级 `usage_script` 脚本（`provider.rs:507-509`、`provider.rs:135-141`（`has_usage_script_enabled`））；③会话 JSONL → `proxy_request_logs` 表（`services/session_usage.rs:3-8`）；④本地代理请求日志与费用（`proxy/usage/`）。 |
| 托盘 | ✅（可切换） | `TRAY_APPS` 内含 Claude（`tray.rs:35-44`，名字 "Claude Code"）；托盘可换供应商/看用量/看账号 hint，但**托盘里不切模式、不启停路由**（`tray.rs:1-8`）。 |
| 官方卡绑 Claude 账号 | ✅（只有 CC 分支调用） | `category=="official"` + `meta.authBinding(authProvider="claude_official")`（`claude_official.rs:26-47`）；切换时把该账号登录换进默认槽位，失败回滚（`:66-160`）。 |
| 配置目录覆盖 | ✅ | `settings.claude_config_dir`（`settings.rs:1284-1290`），影响 settings/MCP/skills/prompt/session 全部路径。 |
| 首次导入（从 live 反推供应商） | ✅ | `services/provider/live.rs:1084-1150`；若 live 处于代理接管态（含 `PROXY_MANAGED`）则拒绝导入（`:1093-1105`）。 |

---

## 6. Claude Code 独有 / 特有的配置机制

1. **四档模型别名**：`opus / sonnet / haiku / fable`（`live/project/claude.rs:291-360`），分别映射到 `ANTHROPIC_DEFAULT_{OPUS,SONNET,HAIKU,FABLE}_MODEL`；
   `/model` 里保存的选择存在顶层 `model`（是 floor，随供应商切换而清空），`modelPicker` 是自定义菜单（`floor.rs:61-84`、`controller.rs:218-256`）。
2. **`[1M]` 后缀**：模型名/别名后缀 `[1M]` 表示 1M 上下文窗口；前端常量 `CLAUDE_ONE_M_MARKER = "[1M]"`（`src/components/providers/forms/hooks/useModelState.ts:26-42`），后端 `ONE_M_MARKER_FOR_CLIENT`（`live/project/claude.rs:211`）、判定 `has_one_m_marker`（`:585-591`）。
   选模型时只在上游带标记时给别名加后缀，haiku 档不加（`claude-haiku-5-5` 本身按 1M 算），`claude.rs:466-475`。
3. **上游 5.5 版本 → 客户端 id 改写**：`claude-opus-5.5` 之类上游 id 要写成 `claude-opus-5-5`，否则 CC 模型表显示错、输出上限按 64K 截断（`claude.rs:489-519`，`POINT_RELEASE_CLIENT_IDS`）。
4. **协议选择器（Bedrock/Vertex/Foundry/网关）**：`CLAUDE_CODE_USE_*` 七个白名单（`floor.rs:13-26`），不能按前缀匹配（同前缀还有与供应商无关的 `USE_POWERSHELL_TOOL` 等功能开关）。
   旧 Bedrock API Key 行自动迁移成 `env.AWS_BEARER_TOKEN_BEDROCK`（`claude.rs:66-79`）。
5. **订阅账号 token 键**：`CLAUDE_CODE_OAUTH_TOKEN / _REFRESH_TOKEN / _SCOPES`、`CLAUDE_CODE_API_KEY_HELPER_TTL_MS`，以及与顶层 `apiKeyHelper` 配套（`floor.rs:33-45`、`:61-84`）。
6. **窗口强制上限**：`CLAUDE_CODE_MAX_CONTEXT_TOKENS` 只对非 `claude-` 前缀模型 id 生效（预设注释 `claudeProviderPresets.ts:152-160`），Stack id 前缀 `ccs-claude-` 正是为此设计（`mode/stack.rs:30-36`）。
7. **故障转移**只在代理模式：`in_failover_queue` + `proxy/failover_switch.rs`；Stack 明确不做（`mode/stack.rs:14-15`）。
8. **无 live 文件监听（无 watcher）**：上游没有 `notify`/watch 逻辑（全仓 `grep -rn "notify" live/ mode/` 只命中 `Condvar`）。
   替代机制：①切换时按关键字段重投影（漂移被「清空再写」纠正）；②写入前 hash 比对 + 重新规划（`live/engine.rs:1-11`）；③CC Switch 启动时 `controller::startup` 重新接上/重写（`controller.rs:1701-1790`）；④首次写备份 + pending 前滚。
9. **多账号快照目录**：账号切换时把登录「移」进默认槽位，账号目录软链共享 skills/projects，镜像 settings.json（去掉关键字段）与 `.claude.json`（只镜像 mcpServers）+ `CLAUDE.md` 写成 `@<默认 CLAUDE.md>`（`services/claude_account/shared.rs:1-13`、`shared.rs:157-170`）。
10. **`meta.apiFormat` 协议转换**：CC 可以指向 OpenAI Chat/Responses/Gemini 形状的上游，由本地代理转换（`provider.rs:535-540`）；代理模式下写客户端的是 anthropic 形状契约（`proxy/providers/transform*.rs`）。
11. **思考档位**：`meta.claudeReasoningEffort`（`off`/`passthrough`/缺省自动），只在 CC→OpenAI 转发路径生效（`provider.rs:575-582`）。
12. **「Claude 插件」开关**：非官方供应商时往 `~/.claude/config.json` 写 `primaryApiKey:"any"`，让 CC 跳过「检测到自定义 API key」的确认弹窗；切回官方时删掉这个键（`claude_plugin.rs:48-84`、`commands/plugin.rs:24-30`）。
13. **跳过首次引导**：往 `~/.claude.json` 根写 `hasCompletedOnboarding:true`，可恢复（删字段）（`commands/plugin.rs:38-48`、`claude_mcp.rs:124-167`）。
14. **打开终端用临时 `--settings`**：卡片「打开终端」会生成临时 settings 文件，把该卡 env 和 `modelPicker` 写进去，并挡住切换会清掉的键（`commands/misc.rs:5581-5600`、`services/provider/claude_editor.rs:143-149`）。

---

## 7. 未确认 / 边界（不要写进报告当作事实）

- 「已在运行的 `claude` 会话是否需要重启才生效」：代码只支持「新启动的进程读取」，**没有**明文规定旧会话行为；上游 docs/ 无相关说明 → 标注为推断。
- CC 是否读取项目级 `.mcp.json`：上游 CC Switch **不管理**它，是否由客户端读不在本仓库范围 → 未确认。
- 会话「恢复/继续」：`session_manager/providers/claude.rs` 只看到扫描/解析/删除（`:38-64`），是否支持在终端里 resume 由 `commands/session_manager.rs`/前端决定，未逐一核实。
- `~/.claude/plugins` 目录内容：上游只做软链共享（`claude_account/shared.rs:28-38`），插件安装本体不在 CC Switch 职责内（`claude_plugin.rs` 只读写 `~/.claude/config.json`）。
- 上游 changelog 中与 CC 配置相关的行为变更未逐条比对（时间原因）。

---

## 8. 关键文件行号索引（便于最终报告引用）

| 主题 | 位置 |
|---|---|
| CC 配置目录 / settings 路径 / MCP 路径 | `config.rs:91`、`config.rs:100-102`、`config.rs:281-309` |
| 原子写 / 0600 / 首写备份 | `config.rs:436-470`、`live/engine.rs:52`、`live/engine.rs:161-181` |
| 关键字段/独有字段名单 | `live/floor.rs:18-116` |
| 切换补丁 / 预览投影 / 行回存 | `live/project/claude.rs:89-116`、`:118-131`、`:135-198` |
| 代理契约投影 | `live/project/claude.rs:249-289`、`:424-480`、`:544-570` |
| 模式状态 / 契约 / Stack 状态 | `mode/state.rs:54-140` |
| 进入/退出/启动/换路由 | `mode/controller.rs:719`、`:863`、`:1701`、`:1046` |
| 代理契约装配（CC） | `mode/controller.rs:187-331`、`:401-432` |
| 直连写回 | `mode/controller.rs:531-560` |
| 切换入口 | `services/provider/mod.rs:6825`、`:6853`、`:6919-6958`、`:7128-7148` |
| 编辑器预览 | `services/provider/claude_editor.rs:1-8`、`:122-134`、`commands/provider.rs:104-135` |
| 官方卡 / 账号切换 | `services/provider/claude_official.rs:26-70` |
| 校验 / 归一化 | `services/provider/mod.rs:7994-8006`、`:8399-8450` |
| 预设 / 官方预设 | `src/config/claudeProviderPresets.ts:77-93` |
| 前端 meta / 分类 | `src/types.ts:1-10`、`:176-255` |
| 托盘 | `tray.rs:35-44`、`tray.rs:1524-1560` |
| 多账号共享层 | `services/claude_account/shared.rs:1-38` |
| Profile 四槽 | `services/profile.rs:34-60`、`:131-140` |
