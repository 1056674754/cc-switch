# Claude Code（cc） vs Claude Desktop：配置差异全览（上游 + 本地 diy）

> **口径**
> - **上游** = `origin/main`（cc-switch **v4.0.25**，`7afceec1`）。
> - **本地** = `diy` 分支 **HEAD `7538d3d7`**（比上游多 **59 个提交**，即本地网关改造）。
> - 行号：未特别标注时均为本地 HEAD / 当前工作区行号；上游专属结论标注 `origin/main:路径:行`。
> - 备注：整理期间另一会话正在工作区改「供应商启停 / 删除供应商 + v28 迁移」（17 个未提交文件），只影响 `commands/suppliers.rs`、`database/schema.rs`、`suppliers` 前端等，**不涉及两客户端配置写入文件**；本文相关文件行号不受影响。

---

## 0. 结论速览

1. **两者在数据库里是同一套 provider 模型**：同一张 `providers` 表、同一个 `(id, app_type)` 主键、同样的 `settings_config = {"env": {...}}` 形状；差异全部落在 `meta` JSON、前端能力集合和"写到磁盘的载体"上。
2. **写到磁盘上的东西完全不同**：
   - **cc**：一份 `~/.claude/settings.json`（进程环境配置），切换 = 改 `env`；写入是字段级 patch（floor + exclusive），热生效，不需要重启桌面应用。
   - **Claude Desktop**：一套 **4 个文件的 3P Profile 配置**——`Claude/claude_desktop_config.json`、`Claude-3p/claude_desktop_config.json`（只写 `deploymentMode`）、`Claude-3p/configLibrary/00000000-0000-4000-8000-000000157210.json`（网关 profile）、`_meta.json`（profiles 索引 + `appliedId`）；Desktop **只在启动时读**，切换后必须完全退出并重启。
3. **模型表达方式不同**：cc 把模型名/别名**直接写进 env**（本地 diy 写的是模型引用 `<供应商名>/<模型 id>`，1M 加 `[1M]`）；Desktop 的模型菜单只认 **claude-safe 的角色 route id**（`claude-sonnet-5`/`claude-opus-5`/`claude-haiku-4-5`/`claude-fable-5`），真实模型由 profile 的 `inferenceModels`/`labelOverride` 或**请求到达网关后**再映射。
4. **上游 v4.0.25 已经完整支持 Claude Desktop**（直连 / 模型映射 / 聚合 Stack、configLibrary、`/claude-desktop` 路由前缀、`ccs-` 网关 token 全部是上游代码，`claude_desktop_config.rs` 上游就有 3185 行）。本地 diy 的改动是：加一层「供应商 / 上游 / 绑定 / 目录 / 每 app 模型配置 / 接管」体系，把 Desktop 的 4 个角色档**抽象成和 cc 一样的「1+4 槽位」**，并让 Desktop 的槽位在**请求时**映射到模型引用。
5. **能力面差距很大**：cc 有 MCP / Skills / 提示词 / 会话 / 用量 / 故障转移 / 项目整套；Desktop 只有"供应商切换 + 路由/映射 + 聚合 + 托盘"，项目只保存供应商，在 Desktop 页面打开的 MCP/Skills/提示词/会话面板**实际作用于 cc**。

---

## 1. 配置载体对照（最核心的差异）

| 维度 | Claude Code（cc） | Claude Desktop |
|---|---|---|
| 主配置文件 | `~/.claude/settings.json`（旧安装可能是 `~/.claude/claude.json`；可被「Claude 配置目录覆盖」改根）<br>`config.rs:297-309` | 4 个受管文件：<br>`~/Library/Application Support/Claude/claude_desktop_config.json`<br>`~/Library/Application Support/Claude-3p/claude_desktop_config.json`<br>`.../Claude-3p/configLibrary/00000000-0000-4000-8000-000000157210.json`<br>`.../Claude-3p/configLibrary/_meta.json`<br>`claude_desktop_config.rs:1708-1730`、`:1290-1329` |
| 文件里写什么 | 顶层关键字段（`model`/`modelPicker`/`apiKeyHelper` 等 12 键）+ `env`（`ANTHROPIC_*` 等）+ 供应商 exclusive 兼容开关 | profile：`inferenceProvider`/`inferenceGatewayBaseUrl`/`inferenceGatewayApiKey`/`inferenceGatewayAuthScheme`/`inferenceModels`/`modelDiscoveryEnabled`；两个 config 只写 `deploymentMode: "3p"`（官方为 `"1p"`）；`_meta` 写 `entries` + `appliedId` |
| 谁读它 | 每个新启动的 `claude` 进程读 env；CC Switch 侧不需要重启 | Claude Desktop 启动时读；**必须完全退出并重开**（README_ZH.md:441） |
| 切换粒度 | 字段级 patch：floor 键先清再写，exclusive 键按"上一家写过的值"精确回退；`live/project/claude.rs` 的 `direct_patch` | 整份 profile 重建 + `deploymentMode` 补丁；profile 用 `floor` 先清再写（`gateway_profile_patch`，`claude_desktop_config.rs:1431+`） |
| 凭据写哪 | 直连：provider 的 `env.ANTHROPIC_AUTH_TOKEN/API_KEY`；路由：占位符 `PROXY_MANAGED`（`live/project/claude.rs:199`） | 直连：provider `env.ANTHROPIC_AUTH_TOKEN` → `inferenceGatewayApiKey`；路由：真实网关 token `ccs-<32 位 hex>`（存 `settings` 表 `claude_desktop_gateway_token`） |
| 模型名形态 | 直连：provider 原值；路由（上游）：`claude-sonnet-5[1M]` 等稳定别名；本地 diy 新路径：模型引用原文 | 上游只发 claude-safe 角色 route id（`claude-*-5`）+ 聚合 id `claude-fable-ccs-<6 位数字>`；本地 diy 在有每 app 模型配置时，还会把 `default + allowed` 的模型引用作为额外条目发布（原始 `<供应商>/<模型>` id，见 §8.5）；真实上游模型只存 CC Switch 内（`labelOverride` 只用来显示） |
| 菜单/模型发现 | `/model` + 四档 `ANTHROPIC_DEFAULT_*_MODEL`；本地接管路径可选写 `CLAUDE_CODE_ENABLE_GATEWAY_MODEL_DISCOVERY`（须 provider 行里带） | Desktop 模型菜单直接由 profile 的 `inferenceModels` 决定（最多 200 条），`modelDiscoveryEnabled: true` 时本地 `/v1/models` 只补名字/说明 |
| 生效/契约 | 代理"契约"（env 指纹）没变就不碰文件；CC Switch 不需要重启 | 契约（profile hash）没变就不写文件；写了就等用户重启 Desktop |
| 官方回退 | 切官方卡 = 清空所有 floor 键，回落客户端自带登录（`providers_seed.rs`） | `deploymentMode` 改回 `1p`、清 profile 的 6 个 floor 键、`_meta` 摘条目；profile 文件保留、Key 从磁盘清掉（`claude_desktop_config.rs:1332-1379`） |
| 备份/原子性 | 每个文件**首次被改写前**备份到 `~/.cc-switch/backups/live-first-write/`；临时文件 + rename 原子写、0600、`live-state.json` pending 崩溃前滚 | 4 个文件在**同一次 operation** 里提交，任一失败不落指针；同样有首写备份、profile 0600 |
| 硬约束 | 基本没有；Claude Code 自己解析 env | Desktop fail-all 校验：`inferenceModels` 名字必须 claude-safe（含 `sonnet/opus/haiku/fable/mythos` 角色词）、拒绝 `[1m]` 后缀、最多 200 条；直连只接受原生 Anthropic Messages、不接受托管 OAuth/完整 URL |

---

## 2. provider 数据模型的差异（上游已有，两边同构）

- **相同**：`providers` 表（`id, app_type, name, settings_config, meta, category, ...`，`database/schema.rs:311` 起）；`settings_config` 都是 `{"env": {"ANTHROPIC_BASE_URL": "...", "ANTHROPIC_AUTH_TOKEN": "..."}}`；两个 app 的官方卡 seed 都是空 env（`providers_seed.rs`：`claude-official` / `claude-desktop-official`）。
- **差异全在 `meta` JSON**（`provider.rs:497-506`）：
  - `claudeDesktopMode: direct | proxy`（`provider.rs:399-405`）——Desktop 供应商卡自己的"接入方式"；
  - `claudeDesktopModelRoutes: { routeId -> { model, labelOverride?, supports1m? } }`（`provider.rs:407-419`）——Desktop 特有的"菜单名 → 真实模型"映射表；
  - `apiFormat`、`providerType`（`github_copilot`/`codex_oauth`/`xai_oauth`）在 Desktop 直连时直接被拒（`claude_desktop_config.rs:493-551`）。
- **预设/表单形态不同**：`claudeProviderPresets.ts` 是整块 `settingsConfig` env；`claudeDesktopProviderPresets.ts` 是顶层 `baseUrl` + `mode` + `modelRoutes[]`（真实模型名只保存在 CC Switch 内，不写进 Desktop profile）。
- **本地新增的网关表**（上游 v24 基线）：v25 `upstreams / upstream_endpoints / upstream_credentials / provider_bindings`（同一 host+protocol 算一个上游，每个 app 一条 binding）、v26 `model_map / secret_kind`、v27 `gateway_model_overrides / gateway_app_profiles`；工作区另有未提交的 v28（供应商启停）。每 app 的「默认 + 槽位 + 允许列表」存在 `gateway_app_profiles.payload`（`gateway_config.rs:45-80`）。

---

## 3. 上游 origin/main：cc 的三种模式与写入字段

cc 的主配置是"DB 行 → 投影"：floor 键先全清再写目标，exclusive 键（兼容开关/窗口值）按上一家的值精确回退。

### 3.1 直连（direct）
- 写回 `settings.json`：把该 provider 的 `settings_config` 里 **floor 键**（顶层 12 键 + `env.ANTHROPIC_*`/`AWS_*`/`VERTEX_REGION_*` 前缀 + 协议选择器 + 具名键）整体替换；`exclusive` 键追加；指针和文件同事务提交（`services/provider/claude_direct.rs:30-95`、`live/project/claude.rs:89-131`）。
- 典型字段：`env.ANTHROPIC_BASE_URL`、`env.ANTHROPIC_AUTH_TOKEN`（或 `ANTHROPIC_API_KEY`）、`env.ANTHROPIC_MODEL`、`env.ANTHROPIC_SMALL_FAST_MODEL`、`env.ANTHROPIC_DEFAULT_{OPUS,SONNET,HAIKU,FABLE}_MODEL`、`CLAUDE_CODE_USE_BEDROCK/VERTEX` 等，取决于 provider 行里有什么。
- 官方卡（空 env）切换后 = 所有 `ANTHROPIC_*` 被删，客户端回落到 `~/.claude/.credentials.json` / Keychain 登录。

### 3.2 路由 / 本地代理（proxy）
`proxy_projection`（`live/project/claude.rs:249-289`，上游同款）写：
- `env.ANTHROPIC_BASE_URL` = `http://127.0.0.1:<port>`（**不带路径前缀**）；
- `env.ANTHROPIC_AUTH_TOKEN` = `PROXY_MANAGED` 占位（或托管账号时 `ANTHROPIC_API_KEY`，二选一避免 Claude Code 的"Both set"告警）；
- 四档稳定别名：`ANTHROPIC_DEFAULT_HAIKU_MODEL=claude-haiku-5-5`、`SONNET=claude-sonnet-5`、`OPUS=claude-opus-5`、`FABLE=claude-fable-5`；上游是 1M 模型时别名加 `[1M]`；
- 每档再写 `ANTHROPIC_DEFAULT_*_MODEL_NAME`（显示名）和 `_MODEL_DESCRIPTION`（`<供应商> · <上游模型> · <窗口>`）；
- 若 provider 行里有 `CLAUDE_CODE_SUBAGENT_MODEL`，也写（5.5 代模型会换成目录 id）；
- 其余 floor 键全部清空（协议选择器、云凭据、用户自定 `/model` 选择等），防止客户端绕过代理；路由那家的 `exclusive` 键保留。

### 3.3 聚合 / Stack
- 仍是代理模式 + `StackState.enabled`：四档都写默认那家的 Stack id `ccs-claude-<key>--<model>`；写顶层 `modelPicker` 和 `CLAUDE_CODE_MAX_CONTEXT_TOKENS`；请求按 `ccs-claude-` 前缀直达成员那家，不做故障转移（`mode/stack.rs`、`mode/controller.rs:187-331`）。

### 3.4 写入引擎（两种 app 共用）
- 原子写 + 0600；每个文件首次改写前备份；多文件操作记 pending，崩溃后前滚/丢弃；写前 hash 比对，最多重算 3 次（`live/engine.rs`、`mode/operation.rs`）。
- 编辑器"切过去之后配置文件长什么样"= 真切换同一个补丁在内存里跑一遍（`claude_editor.rs`）；**Desktop 没有这个预览**（前端只给"打开配置目录"）。

---

## 4. 上游 origin/main：Desktop 的三种模式与写入字段

Desktop 的 provider 卡有两种模式（`meta.claudeDesktopMode`），落到磁盘的都是"网关 profile"。

### 4.1 直连（direct）
`direct_profile`（`claude_desktop_config.rs:1217-1228`）把 Desktop 当成"直接连第三方 Anthropic 兼容端点"：
```json
{
  "inferenceProvider": "gateway",
  "inferenceGatewayBaseUrl": "<provider.env.ANTHROPIC_BASE_URL>",
  "inferenceGatewayApiKey": "<provider.env.ANTHROPIC_AUTH_TOKEN>",
  "inferenceGatewayAuthScheme": "bearer",
  "disableDeploymentModeChooser": true,
  "coworkEgressAllowedHosts": ["*"],
  "inferenceModels": ["..."]   // 可选：来自 provider.meta.claudeDesktopModelRoutes
}
```
- `inferenceModels` 条目：无 1M/显示名时是纯字符串；否则 `{name, labelOverride?, supports1m: true, prefer1m: true}`（`inference_model_json`，`:416-432`）。
- 直连**不能**做模型映射（route.model 必须为空或等于 route_id）、不支持托管 OAuth、不支持完整 URL（`validate_direct_provider`，`:493-551`）；也不写 `mcpServers` 等任何其他键。

### 4.2 模型映射 / 本地路由（proxy）
`route_profile`（`:1247-1289`）：
```json
{
  "inferenceProvider": "gateway",
  "inferenceGatewayBaseUrl": "http://127.0.0.1:<port>/claude-desktop",
  "inferenceGatewayApiKey": "ccs-<32 位 hex>",
  "inferenceGatewayAuthScheme": "bearer",
  "disableDeploymentModeChooser": true,
  "coworkEgressAllowedHosts": ["*"],
  "inferenceModels": [
    {"name":"claude-sonnet-5","labelOverride":"...","supports1m":true,"prefer1m":true},
    {"name":"claude-opus-5", ...},
    {"name":"claude-haiku-4-5", ...},
    {"name":"claude-fable-5", ...}
  ],
  "modelDiscoveryEnabled": true
}
```
- 4 个固定角色 route id 来自 `DEFAULT_PROXY_ROUTES`（`:60-88`），可被 provider 的 `claudeDesktopModelRoutes` 覆盖；发布名会做修正（5.5 目标换成 `claude-opus-5-5` 等；haiku 4.5 目标不是旧 Haiku 时发 `claude-haiku-5-5`；非法 route 借名 `claude-sonnet-5`/`-r2`…，`:794-842`）。
- `inferenceModels` 上限 **200**，多出的截断；聚合（Stack）模型排在路由那家后面，id 为 `claude-fable-ccs-<6 位 FNV 散列>`，`labelOverride` 是 `<供应商> / <模型>`。
- 请求路径：Desktop 发 `claude-opus-5` 这类名字 → `map_proxy_request_model`（`:938-1012`）做"精确 route → opus 别名 → 旧映射原键 → 角色关键词回落（fable 无档降 opus）"四级映射，未知直接 400；随后换成真实上游模型转发。
- 路由入口是**同一个本地 HTTP 服务的 `/claude-desktop` namespace**，独立 token 鉴权（`handlers.rs`、`proxy/server.rs`）。

### 4.3 聚合 / Stack（上游已支持）
- `supports_stack` 含 `ClaudeDesktop`（`mode/stack.rs:54-59`）；Desktop 的 Stack id 是 `claude-fable-ccs-<6 位数字>`（因为 Desktop 的 fail-all 校验只接受 claude 角色词，且末尾 8 位数字会被当日期剥掉，`mode/stack.rs:40-45,129-142`）。
- 注意：README_ZH.md:349 功能矩阵把 Desktop「聚合」标成 `–`，**与代码支持不符**（文档滞后）。

### 4.4 受管文件与操作
- `write_profile_at_paths`（`:1290-1329`）一次提交 4 个文件：两个 `deploymentMode: "3p"` + profile + `_meta`（entries 登记 `{id: PROFILE_ID, name: "CC Switch"}`，`appliedId` 指向它）。profile 里的 `disableDeploymentModeChooser`/`coworkEgressAllowedHosts` 是"策略键"：缺失时写入，用户改过就不动（`live/floor.rs:202-205`）。
- 官方恢复 `restore_official_at_paths`（`:1332-1379`）：两个 `deploymentMode` 改 `"1p"`；清旧版写在 `enterpriseConfig` 里的网关键；清 profile 的 6 个 floor 键；`_meta` 摘条目。profile 文件本身保留（用户在 Desktop 里对它做的其它设置不丢，Key 从磁盘删掉）。
- 旧版迁移：直连指针是"模型映射卡"的旧状态会在启动时转入路由模式（`claude_desktop_config.rs:368-381`、`mode/controller.rs` 老卡迁移）。

---

## 5. 本地 diy 网关改造后：两边的写入路径

### 5.1 新增的层（上游没有）
- **供应商 / 上游**：`upstreams`、`upstream_endpoints`、`upstream_credentials`、`provider_bindings`（v25/v26）；一个"供应商"= 一个确切的上游 API（服务根 + 上游协议），CC / Desktop 可以各留一条 binding。
- **模型目录**：`services/gateway_catalog.rs`，把 provider 行派生成 `<供应商名>/<模型 id>` 引用（重名加 `（2）`），页面可选、可改名/隐藏（`gateway_model_overrides`）。
- **每 app 模型配置**：`gateway_app_profiles.payload`（`AppModelConfig { default, slots, allowed, routing }`，`gateway_config.rs:17-35`）：默认 + 槽位（claude 系 4 个：haiku/sonnet/opus/fable）+ 允许列表；允许列表非空时 default 必填且必须在列表内；前端把默认+槽位当 pinned（永远勾上、禁用，`AppConfigPage.tsx:105-141`）。
- **接管 / 释放**：`adopt_app_to_gateway` / `release_app_from_gateway`（`commands/proxy.rs:101-119`），底层就是上游的 `mode::controller::enter/exit`；保存模型配置后若已接管会立刻 `resync_route` 重写客户端（`commands/gateway.rs:319-339`）。
- **UI**：删掉上游整套 `providers/mode/*`（ModeDialog/SwitchModePanel/StackSettingsSheet）与 `useModeActions`，改为应用页「编辑配置」+ 供应商页 + 网关接管按钮。

### 5.2 cc 接管：`~/.claude/settings.json`
实现：`live/project/claude.rs:290-367` 的 `ModelConfigSlots` / `proxy_projection_from_model_config`，仅当 Stack 名单为空时启用（`mode/controller.rs:187-228`）。

| 键 | 值 |
|---|---|
| `env.ANTHROPIC_BASE_URL` | 网关地址（**不带** `/claude` 前缀） |
| `env.ANTHROPIC_MODEL` | `default` 引用的原文（没配不写） |
| `env.ANTHROPIC_MODEL_NAME` | default 的显示名（★本分支新增键，上游从未写过） |
| `env.ANTHROPIC_DEFAULT_{HAIKU,SONNET,OPUS,FABLE}_MODEL` | 该槽位引用；槽位没配则 **回落 default**；仍没有就不写 |
| `env.ANTHROPIC_DEFAULT_*_MODEL_NAME` | 槽位显示名（若有） |
| `env.ANTHROPIC_AUTH_TOKEN` 或 `env.ANTHROPIC_API_KEY` | `PROXY_MANAGED`（二选一，按路由那家的凭据策略） |
| `env` 其余 | 路由那家 provider 的 `exclusive` 兼容开关原样保留 |
| 顶层 | 不写（`top: Map::new()`）；1M 引用追加 `[1M]` |

- 与上游 proxy 写法的差别：本地新路径**不写**四档稳定别名（`claude-sonnet-5` 等）、`_MODEL_DESCRIPTION`、`CLAUDE_CODE_SUBAGENT_MODEL`、Stack 的 `modelPicker`/`MAX_CONTEXT_TOKENS`；模型名直接是引用。
- 释放：`controller::exit` → `write_direct` 的 Claude 分支先清 floor 键（`ANTHROPIC_*` 全清、含 base url/token/model、四档与 `_NAME`、subagent、顶层 `model`/`modelPicker`），再写回直连供应商；官方卡则清空后回落登录。

### 5.3 Desktop 接管：4 个文件
实现：`mode/controller.rs:639-659` → `claude_desktop_config::route_profile`（`:1247`）→ `write_profile_at_paths`（`:1290`）。

| 文件 | 写入 |
|---|---|
| `~/Library/Application Support/Claude/claude_desktop_config.json` | `deploymentMode: "3p"` |
| `~/Library/Application Support/Claude-3p/claude_desktop_config.json` | `deploymentMode: "3p"` |
| `.../Claude-3p/configLibrary/00000000-0000-4000-8000-000000157210.json` | `inferenceProvider` / `inferenceGatewayBaseUrl`（`<gateway>/claude-desktop`）/ `inferenceGatewayApiKey`（`ccs-*`）/ `inferenceGatewayAuthScheme` / `inferenceModels` / `modelDiscoveryEnabled` / `disableDeploymentModeChooser` / `coworkEgressAllowedHosts` |
| `.../Claude-3p/configLibrary/_meta.json` | `entries: [{id, name:"CC Switch"}]` + `appliedId` |

`inferenceModels` 的组成（本地）：
1. 先放路由那家的 4 个角色 route id（`proxy_model_routes`，来自 provider 的 `meta.claudeDesktopModelRoutes` 或默认 4 档）；
2. Stack 开启时再追加 `claude-fable-ccs-<6 位>` 聚合条目；
3. Stack 关闭但有每 app 模型配置时，追加 `default + allowed` 里的**模型引用**（id 就是 `<供应商名>/<模型 id>`，label 是 `供应商 / 显示名`；`mode/controller.rs:262-311`）。

**1+4 槽位在 Desktop 上是"请求时映射"**（不是写 env）：
- Desktop 菜单发 `claude-opus-5` → `handlers.rs:218-240` → `gateway_route.rs:165-193`：`role_for_route_id` 认出 role → 读每 app 配置的 `slots[role]` → 换成模型引用 → 解析上游；
- 槽位没配 → 返回 `None`，**回落到 provider 的模型映射表**（老行为）；
- 发布条目（default + allowed 引用）直接按引用选上游，和 Stack 的旧 id 并存。

**释放**：`restore_official_at_paths`：`deploymentMode` 改 `"1p"`、清 profile 6 个 floor 键、`_meta` 摘条目；profile 文件保留。

### 5.4 两边「1+4」对照

| 槽位 | Claude Code 落点（本地 diy） | Claude Desktop 落点（本地 diy） |
|---|---|---|
| 默认 | `ANTHROPIC_MODEL` (+`_NAME`) | 作为额外菜单条目的模型引用发布（不是 role id，也不一定是列表第一项） |
| Fable | `ANTHROPIC_DEFAULT_FABLE_MODEL` | 菜单 `claude-fable-5` → 请求时查 `slots.fable` |
| Opus | `ANTHROPIC_DEFAULT_OPUS_MODEL` | 菜单 `claude-opus-5` → `slots.opus` |
| Sonnet | `ANTHROPIC_DEFAULT_SONNET_MODEL` | 菜单 `claude-sonnet-5` → `slots.sonnet` |
| Haiku | `ANTHROPIC_DEFAULT_HAIKU_MODEL` | 菜单 `claude-haiku-4-5` → `slots.haiku`（可被修正发布成 `claude-haiku-5-5`） |
| 槽位空 | **回落 default** | **不回落 default**，回落 provider 映射表 |
| 1M 表达 | 引用加 `[1M]` 后缀（网关剥掉） | `supports1m` + `prefer1m`（profile 不接受 `[1M]`） |
| 凭据 | `PROXY_MANAGED` 占位 | 真实 token `ccs-*` |
| 上限 | 无 | `inferenceModels` 最多 200 条 |

### 5.5 生效时机
- cc：写完 `settings.json` 后新启动的 `claude` 生效；CC Switch 托盘不提示重启；代理契约不变时连文件都不碰。
- Desktop：写文件后必须**完全退出并重新打开**；使用模型映射时 CC Switch 要保持运行；Desktop 运行中改文件不会热更新（`mode/controller.rs:1156` 附近注释明确：Desktop 不"分离"，因为它只在启动时读配置）。

---

## 6. 能力差异（与"配置"相关）

| 能力 | cc | Claude Desktop | 依据 |
|---|---|---|---|
| 供应商切换/共存 | 切换式（同一时间一家生效） | 切换式 | `app_config.rs` |
| 本地路由 / 模型映射 | ✅ | ✅ 但仅"模型映射"时，无故障转移 | README 矩阵、`appConfig.tsx:93-120` |
| 聚合 Stack | ✅ | ✅（代码支持；README 矩阵写 `–` 属滞后） | `mode/stack.rs:54-59` |
| 故障转移 / 路由设置 | ✅ | ❌（不在 `PROXY_APP_IDS`） | `appConfig.tsx:49-55` |
| MCP | ✅ `~/.claude.json` | ❌ 不由 CC Switch 管 | `appConfig.tsx:180-199` |
| Skills | ✅ `~/.claude/skills` | ❌ | `appConfig.tsx:77-91` |
| 提示词 | ✅ `CLAUDE.md` | ❌ | `prompt_files.rs:51` |
| 会话历史 | ✅ `~/.claude/projects/**/*.jsonl` | ❌ | 会话服务 |
| 项目 Profile | providers + MCP + Skills + 提示词 | **只存供应商** | README_ZH.md:372 |
| 用量统计 | 完整（会话 + 代理日志 + 订阅） | 仅"模型映射"时经代理日志 | `usage_stats.rs:413-433` |
| 托盘 | ✅ | ✅ | `tray.rs` |
| 重启要求 | 不需要 | 必须重启 Desktop | README_ZH.md:441 |
| 编辑配置预览 | ✅（切过去后的 JSON） | ❌（只有"打开配置目录"） | `claude_editor.rs`、`commands/config.rs` |
| 在 Desktop 页打开 MCP/Skills/提示词/会话 | — | **实际作用于 cc 的文件/表** | README_ZH.md:366、`appConfig.tsx` |

---

## 7. 本次 diy 分支改了什么（相对上游）

| 改动 | 文件/规模 | 对两客户端的影响 |
|---|---|---|
| 网关/供应商/上游/绑定/目录 | `services/gateway_*.rs`（新增 ~2000 行）、`commands/suppliers.rs`、DB v25/v26（工作区另有 v28） | 两端都从"手填 baseUrl+key"改成"网关上的供应商 + 模型引用" |
| 每 app 模型配置 + 允许列表 | `gateway_config.rs`、`commands/gateway.rs`、v27 `gateway_app_profiles` | 两端第一次有同样的「默认 + 槽位 + 允许列表」抽象 |
| cc 接管改为写模型引用 | `live/project/claude.rs` +176 行；`mode/controller.rs` 新增 `claude_contract` 分支 | cc 从"稳定别名"变成"引用原文写 env"（`ANTHROPIC_MODEL_NAME` 是新键） |
| Desktop 1+4 槽位 | `claude_desktop_config.rs` +56 行（role ↔ route id 双向映射）；`handlers.rs` +22 行请求侧；`gateway_route.rs` +94 行 | 菜单里的 4 个 role id 现在可被每 app 配置改写实际上游；默认+允许列表以引用条目发布 |
| UI 重做 | 删除 `providers/mode/*` 7 文件 + `useModeActions`；新增 `components/gateway/*`（10+ 文件） | 上游的「直连/路由/聚合」弹层 → 应用页「编辑配置」+「接管/释放」 |
| 默认/槽位钉进允许列表 | `AppConfigPage.tsx`（commit `7538d3d7`） | 两端相同 |

上游已有（不要误以为本地新增）：Desktop 的 configLibrary/`/claude-desktop`/`ccs-` token/`inferenceModels`/200 上限/4 个角色 route/Stack 对 Desktop 的支持；cc 的三模式投影与写入引擎。

---

## 8. 观察到的不一致 / 风险（供后续开发参考）

1. **Desktop 槽位不回落 default**：`gateway_route.rs:175-180` 只查 `config.slots[role]`，而 cc 是 `.or(self.default)`（`live/project/claude.rs:316`）。如果产品预期两端一致，这里是缺口。
2. **Desktop 的 default 不是 `inferenceModels` 第一项**：`route_profile` 先放 4 个 role id，再追加 default/allowed 引用（`claude_desktop_config.rs:1260-1277`）；注释里"第一项是默认模型"描述的是 Stack 模式。Desktop 菜单默认选中项可能不是用户配的 default。
3. **`ANTHROPIC_MODEL_NAME` 是本分支新写的键**：上游从未写过；仓库内除写入点与测试外无人读取，Claude Code 是否消费未验证（低风险但建议确认）。
4. **`provider_bindings.model_map` 只写 env**：`p1_set_model_map`（`database/schema.rs:2241-2276`）只改 `ANTHROPIC_*`，而 Desktop 的模型映射读 `meta.claudeDesktopModelRoutes`；对 Desktop binding 改 model_map 不会生效（CC 生效）。
5. **Desktop 发布 app 配置的引用时 id 是原始 `<供应商>/<模型>`**：同一文件的 `is_claude_safe_model_id` 会把它判成不合法（`stale_raw_models` 警告），而 Stack 用的 `claude-fable-ccs-*` 才是 claude-safe 的。是否需要转换值得确认（当前只用于自测/状态提示）。
6. **Haiku role 常量不一致**：cc 代理别名是 `claude-haiku-5-5`，Desktop 默认发布的 role id 是 `claude-haiku-4-5`（旧 id）——`role_for_route_id` 两个都认，但两端展示/语义不完全对齐。
7. **README 功能矩阵滞后**：Desktop "聚合"标 `–`，代码是支持的（`supports_stack`）；Desktop "本地路由"写"模型映射时"，实际与 cc 共用同一路由服务、但无故障转移。

---

## 9. 本机实际配置证据（脱敏）

- `~/.claude/settings.json`（当前 cc 处于**路由模式**，非本地新模型配置路径）：`env.ANTHROPIC_BASE_URL=http://127.0.0.1:15721`、`ANTHROPIC_AUTH_TOKEN=PROXY_MANAGED`、`ANTHROPIC_DEFAULT_SONNET_MODEL=claude-sonnet-5[1M]`、`OPUS=claude-opus-5`、`HAIKU=claude-haiku-5-5`、`FABLE=claude-fable-5`，另有各档 `_NAME`/`_DESCRIPTION`、`CLAUDE_CODE_SUBAGENT_MODEL`、以及 provider 行带进来的 `CLAUDE_CODE_ENABLE_GATEWAY_MODEL_DISCOVERY=1` 等 exclusive 键。
- `~/Library/Application Support/Claude{,-3p}/claude_desktop_config.json`：`deploymentMode="3p"`。
- `.../Claude-3p/configLibrary/_meta.json`：`appliedId` = CC Switch profile（另有一条 "Claude Code Router" 的 profile）。
- 同目录 profile：`inferenceGatewayBaseUrl=http://127.0.0.1:15721/claude-desktop`、`inferenceGatewayApiKey=ccs-…`、`inferenceProvider=gateway`、`inferenceGatewayAuthScheme=bearer`、`modelDiscoveryEnabled=true`、`inferenceModels` 为 4 个 role id（带 `labelOverride`/`supports1m`/`prefer1m`）+ 5 个 `claude-fable-ccs-*` 聚合条目。
- `~/.cc-switch/live-state.json`：`claude` = proxy/attached；`claude-desktop` = proxy/attached + stack enabled（三个成员），与上面磁盘内容一致。

---

## 10. 关键代码索引

| 主题 | 位置 |
|---|---|
| cc 配置路径 | `src-tauri/src/config.rs:297-309` |
| cc 直连写回 | `src-tauri/src/services/provider/claude_direct.rs`、`live/project/claude.rs:89-131` |
| cc 路由/Stack 投影 | `src-tauri/src/live/project/claude.rs:249-289`、`mode/stack.rs`、`mode/controller.rs:187-331` |
| cc 本地 diy 模型引用路径 | `live/project/claude.rs:290-367`、`mode/controller.rs:187-228,314-347` |
| Desktop 全部写盘逻辑 | `src-tauri/src/claude_desktop_config.rs`（路径 `:1708-1730`、profile `:1515`、路由 profile `:1247`、直连 `:1217`、官方恢复 `:1332`） |
| Desktop 本地槽位/role 映射 | `claude_desktop_config.rs:92-119`、`services/gateway_route.rs:165-193`、`proxy/handlers.rs:218-240` |
| 每 app 模型配置 | `services/gateway_config.rs:17-35`、`commands/gateway.rs:304-339`、`components/gateway/AppConfigPage.tsx` |
| 接管/释放 | `commands/proxy.rs:101-119` → `mode/controller.rs:870-1087` |
| DB 迁移 | `database/schema.rs`（v25 `:2036`、v26 `:2078`、v27 `:2103` 起；工作区未提交 v28） |
| 能力矩阵 | `src/config/appConfig.tsx:20-199`、`README_ZH.md:343-380` |
