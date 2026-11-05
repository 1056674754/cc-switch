# CC vs Claude Desktop —— 前端 + 数据库层差异（上游 v4.0.25 → 本地 diy）

基线：`origin/main` = cc-switch v4.0.25；本地 `diy` = `origin/main` + 33 个 commit（`7538d3d7`）。
标注约定：**【上游】** = origin/main 已有；**【本地】** = `git diff origin/main..HEAD` 新增或修改。
范围：只做前端（src/）与数据库层（schema/dao/migrations）。rust 写入路径（live/write、mode controller、proxy）另有同事，本笔记只引用不深挖。

---

## 0. 一句话结论

两个 app 在**数据模型上是同构的**（同一张 `providers` 表、同一 `(id, app_type)` 主键、同 `{"env":{ANTHROPIC_BASE_URL, ANTHROPIC_*}}` 形状），
**差异全部落在三处**：(a) `meta` JSON 里的 Desktop 专属字段；(b) 前端能力集合（Mode/Stack/网关/模型配置有无）；(c) 预设模板与槽位语义。
本地新增的网关层（v25/v26/v27 + `src/components/gateway/*`）把「模型配置」从供应商表单里抽出来做成独立页，并**第一次让 Desktop 也拥有固定的 1+4 槽位**。

---

## 1. 能力 / 配置矩阵

| 能力维度 | Claude Code (`claude`) | Claude Desktop (`claude-desktop`) | 归属 |
|---|---|---|---|
| `AppId` / 可见应用 | ✅ 第一项 | ✅ 第二项 | 【上游】`appConfig.tsx:20-35` |
| `ProxyAppId`（完整本地网关 + 故障转移） | ✅ | ❌ 不在集合内 | 【上游】`appConfig.tsx:93-108` |
| `ModeAppId`（直连/路由） | ✅ | ✅（经同一路由服务，但无故障转移） | 【上游】`appConfig.tsx:110-120`，注释写明差异 |
| `StackAppId`（聚合/Stack 模式） | ✅ | ✅ | 【上游】`appConfig.tsx:122-136` |
| `AdditiveAppId`（累加式写入） | ❌ | ❌ | 【上游】`appConfig.tsx:138-156` |
| `EDITOR_VIEW_APP_IDS`（编辑器投影「切过去的文件长什么样」） | ✅ | ❌ | 【上游】`appConfig.tsx:165-178` |
| MCP 面板 | ✅ | ❌ 显式排除 | 【上游】`appConfig.tsx:180-199`（`McpAppId = Exclude<AppId, "claude-desktop"\|"openclaw">`） |
| Skills 面板 | ✅ | ❌ 不在 `SKILLS_APP_IDS` | 【上游】`appConfig.tsx:77-91` |
| 供应商表单组件 | `ProviderForm` → `ClaudeFormFields` | `ClaudeDesktopProviderForm`（独立组件） | 【上游】`ProviderForm.tsx:344`、`2701-2702` |
| 官方登录 / 托管账号选择 | ✅ `ClaudeAccountSelect`（managed auth `claude_official`） | ❌ 无；官方 = 空 env 种子，切回 1P 模式 | 【上游】`ProviderForm.tsx:107,2787`；`ClaudeOfficialSection.tsx:178-179`；`providers_seed.rs:44-53` |
| 供应商表单里的「接入方式 direct/proxy」 | ❌（模式在 ModeDialog/Stack 概念里） | ✅ 下拉「直连 / 模型映射」 | 【上游】`ClaudeDesktopProviderForm.tsx:1107-1145` |
| 供应商表单里的模型行 | 经 `ClaudeFormFields` + `ClaudeStackModelsField`（meta.stackModels） | 固定四档路由表：角色 / 菜单显示名 / 实际请求模型 / 声明 1M | 【上游】`ClaudeDesktopProviderForm.tsx:1255-1440`、`826-851`（proxy 强制补齐 4 档） |
| 槽位 UI（1 默认 + 4 档） | ✅ fable/opus/sonnet/haiku → `ANTHROPIC_DEFAULT_*_MODEL` | ✅ **本地新增**：同 4 档 → Desktop 菜单 role route id | 【本地】`gateway/appModelConfig.ts:13-48` |
| 模型选择器（两栏 CCR 风格） | ✅ | ✅ | 【本地】`gateway/ModelPicker.tsx:20-45` |
| 允许模型列表 | ✅ 默认+槽位自动进白名单（勾上并禁用） | ✅ 同 | 【本地】`gateway/AppConfigPage.tsx:105-117`、commit `7538d3d7` |
| 项目 Profile 分组 | ✅ `claude` 分组 | ✅ 独立 `claude-desktop` 分组 | 【上游】`lib/api/profiles.ts:9,14-18`；`ProfileSwitcher.tsx:46-50` |
| 用量统计展示口径 | `claude` 桶 | **折叠进 `claude` 桶**，不单列 | 【上游】`services/usage_stats.rs:413-433` |
| 托盘（Tray） | ✅ | ✅ 与路由类 app 同组 | 【上游】`tray.rs:4503` 测试 `desktop_uses_the_same_mode_rows_as_the_routing_apps`、`tray.rs:1105` |
| 官方供应商种子 | `claude-official` | `claude-desktop-official` | 【上游】`dao/providers_seed.rs:14,35-53` |
| 应用页独立卡片 + 状态 | — | ✅ 独立 `claude-desktop-status` 卡片 | 【上游】`components/apps/AppsPage.tsx:113,181,341-358` |
| 网关「编辑配置」页 | ✅ | ✅（本地新建的统一页） | 【本地】`App.tsx:1485,1526,1601-1603`、`gateway/AppConfigPage.tsx` |
| 旧的 ModeDialog / StackSettingsSheet / SwitchModePanel | 存在 | 存在 | 【上游】…**本地已删除**：`src/components/providers/mode/*`（7 文件）、`src/hooks/useModeActions.ts` |

---

## 2. 前端 App 集合（`src/config/appConfig.tsx`，文件与上游逐字节相同，属【上游】）

- `APP_IDS` 14 个，Desktop 第 2 位：`appConfig.tsx:20-35`；`DEFAULT_VISIBLE_APPS` 全 true：`37-52`。
- `ProxyAppId = "claude"|"codex"|"gemini"|"grokbuild"`，`PROXY_APP_IDS` 四个：`93-108`。
  → Desktop **不在**故障转移/路由设置集合里。
- `ModeAppId = ProxyAppId | "claude-desktop"`，`MODE_APP_IDS`：`110-120`。上游注释即写明「Desktop 经同一个路由服务转发，但没有故障转移和路由设置」。
- `StackAppId = "claude"|"codex"|"claude-desktop"`：`122-136`。
- `McpAppId` 排除 Desktop（注释：不由 CC Switch 管理 MCP）：`180-199`。
- `SKILLS_APP_IDS` 无 Desktop：`77-91`。`EDITOR_VIEW_APP_IDS` 无 Desktop：`165-178`。
- 图标/配色：Desktop 用 amber 系区分：`214-221`。

**结论**：前端能力边界是「Design 上 Desktop = 供应商 + Mode/Stack + 项目 + 托盘 + 网关配置；不含 MCP/Skills/编辑器投影/故障转移」。

---

## 3. 供应商表单字段差异

### 3.1 Claude Code：`ProviderForm` → `ClaudeFormFields`（【上游】）
- 分发：`ProviderForm.tsx:338-347`（`appId === "claude-desktop"` → 独立组件，`:344`）。
- CC 分支渲染：`ProviderForm.tsx:2701-2702`。
- 字段：`settingsConfig.env.ANTHROPIC_BASE_URL`（`ProviderForm.tsx:609`）、API Key 字段二选一 `ANTHROPIC_AUTH_TOKEN`（默认）/`ANTHROPIC_API_KEY`（`ClaudeFormFields.tsx:941-955`）、模型档位写 `ANTHROPIC_DEFAULT_SONNET/OPUS/FABLE/HAIKU_MODEL` + `..._MODEL_NAME`（`ClaudeFormFields.tsx:641-669`）、`ANTHROPIC_MODEL`（`:1304-1320`）、`ClaudeStackModelsField`（多行 stackModels，渲染于 `ClaudeFormFields.tsx:1044`）。
- 官方/托管账号：`ClaudeAccountSelect`（`ProviderForm.tsx:2787`）。

### 3.2 Claude Desktop：`ClaudeDesktopProviderForm`（【上游】，共 1486 行）
- 接入方式 state：`mode: "direct" | "proxy"`、`apiFormat`：`ClaudeDesktopProviderForm.tsx:264-269`。
- **提交形状**（关键，DB 层真相）：`settingsConfig.env = { ANTHROPIC_BASE_URL, [ANTHROPIC_AUTH_TOKEN|ANTHROPIC_API_KEY] }`，先删掉两个 key 再写 —— 与 CC 完全同构：`ClaudeDesktopProviderForm.tsx:867-880`。
- **meta 形状**：`claudeDesktopMode`、`apiFormat`、`claudeDesktopModelRoutes`、`providerType`、`claudeReasoningEffort`：`ClaudeDesktopProviderForm.tsx:902-926`。
- 路由表 routeMap：`{ [routeId]: { model, labelOverride, supports1m } }`：`:882-894`。
- 固定四档 + 空档继承：proxy 模式要求至少一个模型，空档用 primary 回填（保证落库四档齐全）：`:826-851`；direct 模式校验必须是 Desktop 认得的 Claude 安全模型名：`:852-865`。
- UI 区块：baseUrl/EndpointField `:1073`；接入方式下拉 `:1107-1145`；apiFormat 下拉（anthropic / openai_chat / openai_responses / gemini_native）`:1151-1200`；四行「模型角色 / 菜单显示名 / 实际请求模型 / 声明 1M」`:1255-1440`；官方说明 `:1018`。
- Preset 触发：选预设时套 `mode/apiFormat/modelRoutes`：`:451-480`。

### 3.3 预设文件差异（【上游】两个文件都在）
| | `claudeProviderPresets.ts` | `claudeDesktopProviderPresets.ts` |
|---|---|---|
| 接口 | `ProviderPreset`：`settingsConfig: object`（整块 env JSON）、`isOfficial?`、`templateValues` | `ClaudeDesktopProviderPreset`：`baseUrl: string` 顶层、`apiKeyField`、`mode: "direct"\|"proxy"`、`apiFormat`、`modelRoutes[]`、`providerType`、`requiresOAuth` |
| 行号 | `claudeProviderPresets.ts:26-70`（`settingsConfig:32`、`isOfficial:33`、`templateValues:41`） | `claudeDesktopProviderPresets.ts:44-69`（`baseUrl:54`、`mode:57`、`modelRoutes:59`、`providerType:60`） |
| 官方供应商 | `isOfficial: true` 标记 + `claude-official` 种子 | `category: "official"` 判定，`officialNotice` 提示块 |
| 安全模型 id | 无此概念（任意上游模型名） | `CLAUDE_DESKTOP_ROLE_ROUTE_IDS = { sonnet:"claude-sonnet-5", opus:"claude-opus-5", fable:"claude-fable-5", haiku:"claude-haiku-4-5" }`（`claudeDesktopProviderPresets.ts:35-40`），注释记录 Desktop 1.12603.1+ 白名单含 fable、mythos 未公开 |

**要点**：Desktop 的「安全模型 id」是核心约束 —— 写进 Desktop profile 的 route id 必须是它认得的那几个，真实上游模型只存在 CC Switch 内部（`provider.rs:411` 注释：「真实上游模型名，只保存在 CC Switch 内部，不写入 Claude Desktop profile」）。这也是 preset 必须按 Desktop 重译、不能直接复用 CC preset 的原因（`claudeDesktopProviderPresets.ts:1-10` 注释：翻译来源 claudeProviderPresets，排除 OAuth 与不兼容预设）。

---

## 4. 模型配置 / 槽位（**本地新增主体**）

本地把「模型配置」从供应商表单抽成独立页，且 Desktop 与 CC 共用同一套 UI 骨架：

- `src/components/gateway/appModelConfig.ts`
  - `CLAUDE_CODE_SLOTS`：fable/opus/sonnet/haiku → hint `ANTHROPIC_DEFAULT_*_MODEL`：`:13-18`
  - `CLAUDE_DESKTOP_SLOTS`：**同 4 档**，hint 指向 Desktop 菜单的 role route id：`:20-42`
  - `slotDefinitions(appType)`：只有 `claude` 与 `claude-desktop` 返回非空：`:44-48` → 其余 app 槽位 UI 不渲染
  - `validateDraft`：默认模型必填、引用必须在目录、允许列表非空时默认必须入列：`:83-111`
- `src/components/gateway/SlotFields.tsx:8-27`（注释：「Claude Code 四档，其余 app 不渲染」；清空 = 删键不发空串）
- `src/components/gateway/AppConfigPage.tsx`：默认模型 + 4 槽位（`:311-334`）+ 允许模型白名单（`:105-117` 默认与槽位自动 pinned）+ 接管/释放（`:240-251`）+ 作用范围/入口模式（`:260-280`）
- `src/components/gateway/AppEditorPage.tsx:198-205` 复用 `SlotFields`
- `src/components/gateway/ModelPicker.tsx:20-45`（两栏 CCR 风格，值 = `<供应商名>/<模型 id>`，目录外的旧值标警告）
- `src/components/gateway/AllowedModelsEditor.tsx`、`modelPickerOptions.ts`、`ClientGatewayBar.tsx`（累加式 app 的「压网关/释放」）
- 挂载：`App.tsx:102-103` import；`:1601-1603` `case "gatewayConfig"`；`:1485`/`:1526` 两处 inline；侧栏 `Sidebar.tsx` 新增「网关」分组（供应商 / 接入端），全局页新增 `suppliers`/`bindings`：`lib/navigation.ts:20-30,59-70`
- 本地删除：`src/components/providers/mode/ModeDialog.tsx`、`ModeTabs.tsx`、`RouteSettingsSheet.tsx`、`StackSettingsSheet.tsx`、`StackSubagentFields.tsx`、`SwitchModePanel.tsx`、`useDesktopConfigMessages.ts`、`src/hooks/useModeActions.ts`（模式/Stack 交互被网关页取代）

**回答「cc 与 Desktop 在 UI 上暴露的槽位是否一致」**：槽位**档位一致**（都是 fable/opus/sonnet/haiku 四档 + 1 个默认模型），但**落点不同** —— CC 落 `settings_config.env.ANTHROPIC_DEFAULT_*_MODEL`（`schema.rs:2217-2247` `p1_set_model_map` 只认 env 键，无 desktop 分支），Desktop 落 `meta.claudeDesktopModelRoutes`（`ClaudeDesktopProviderForm.tsx:913`；写入方为 `commands/provider.rs:348-351`，模式/Stack 场景由 `mode/controller.rs:6748` 写；`provider.rs:499-506`）。
⚠️ 由此得出一条层间不一致：v26 引入的 `provider_bindings.model_map` 经 `p1_set_model_map` 只会写 env 的 `ANTHROPIC_*`，**不会写 Desktop 的 meta 路由**；同一个网关下 CC 的映射能被这套链路覆盖，Desktop 的映射仍只能经 provider 表单/模式控制器写 meta。引用 `dao/providers.rs:901-930` 的注释（「claude 里可以映射成 kimi-k2，claude-desktop 里映射成别的」）时需注意它描述的是存储归属，不是当前网关写入链路已覆盖。

---

## 5. 项目 / 托盘 / 导入导出 / 通用供应商 / MCP-Skills-提示词-会话

- **项目 Profile**：`ProfileScope = "claude" | "claude-desktop" | "codex"`（`lib/api/profiles.ts:9`，注释「CC 与 Desktop 的供应商独立切换，因此各自有独立分组」）。payload 按 app 分槽：`providers/mcp/skills/prompts` 各含 `claude|claude-desktop|codex`（`profiles.ts:14-31`）。但语义上只有 providers 槽对 Desktop 有意义 —— MCP/Skills 两侧值会被后端忽略。前端映射 `ProfileSwitcher.tsx:46-50`（`claude-desktop → claudeDesktop`）；`CurrentProfileIds` 同名字段 `types.ts:520`。
- **托盘**：Desktop 与路由类 app 同一组 mode 行（`tray.rs:4503` 测试名直接写明 `desktop_uses_the_same_mode_rows_as_the_routing_apps`；`tray.rs:1105` 判 `claudeDesktopMode == Proxy`；`tray.rs:4941-4943` nav id `nav:app:claude-desktop:quota`）。
- **设置-路由页**：`RoutingSection.tsx:129-151` 把 `MODE_APP_IDS`（含 Desktop）全部纳入 useQueries + 「usingApps」模式卡片。
- **MCP/Skills 面板在 Desktop 页打开**：前端 `activeApp` 只是切换视图，MCP 页拿 `AppId` 但 `MCP_APP_IDS` 排除 Desktop（`appConfig.tsx:180-199`），Skills 的 `SkillApps` 虽有可选 `"claude-desktop"?: boolean`（`lib/api/skills.ts:22-24`）但 `SKILLS_APP_IDS` 不含它（`appConfig.tsx:77-91`）→ **Desktop 页面下没有可切换的 MCP/Skills 开关，实际作用的仍是 CC（claude）那一套文件/表**。DB 侧同证据：`mcp_servers` 与 `skills` 表只有 `enabled_claude/enabled_codex/...`，全代码库 **没有 `enabled_claude_desktop` 列**（`schema.rs:349-365` 与 grep 全仓 0 命中）。
- **提示词/会话**：API 层 `promptsApi` 接通用 `AppId`（`lib/api/prompts.ts:35-59`），`prompts` 表主键 `(id, app_type)`（`schema.rs:368-372`）；Desktop 在面板中无专属入口 → 打开仍操作 CC 的 `~/.claude/CLAUDE.md`（前端表现层面：属于「页面归属 CC，数据不区分」）。
- **通用供应商（universal）**：`universalProvidersApi` 仅 CRUD + `sync_universal_provider`（`lib/api/providers.ts:289-322`），dao `universal_providers.rs` 是 app 无关表（无 `claude-desktop` 字样，grep 0 命中）→ 对两 app 一视同仁，无 Desktop 专属差异。

---

## 6. 数据库层

### 6.1 表结构（`providers`），两 app 完全同一张表
- DDL：`schema.rs:311-331`（【上游】同形状，上游行号 `schema.rs:27-49`）。
  列：`id, app_type, name, settings_config, website_url, category, created_at, sort_index, notes, icon, icon_color, meta, is_current, in_failover_queue`，**PK `(id, app_type)`**。
- **没有 `claude_desktop_mode` 列**：Desktop 的 mode 存在 `meta` JSON 里（`schema.rs` 全仓无该列；`provider.rs:497-506` 的 `ProviderMeta` 含 `claudeDesktopMode` / `claudeDesktopModelRoutes`）。
- 后加列走 `add_column_if_missing`：`category / created_at / sort_index / notes / icon / icon_color / meta`：`schema.rs:989-995`；`limit_daily_usd / limit_monthly_usd / provider_type`：`:1055-1057`。
- `category` 是两边共用列（官方/第三方分类），不是 Desktop 专属；「mode」也从不在列上。

### 6.2 `settings_config` 实际形状（两 app 同构）
- CC：`{"env":{"ANTHROPIC_BASE_URL":"…","ANTHROPIC_AUTH_TOKEN":"…","ANTHROPIC_DEFAULT_SONNET_MODEL":"…"}}`（写入见 `ClaudeFormFields.tsx:641-669`；DB 测试夹具 `dao/providers.rs:1530-1532` `anthropic()` 造 `{"env":{"ANTHROPIC_BASE_URL":…}}`）。
- Desktop：**同一个 env 形状**（`ClaudeDesktopProviderForm.tsx:867-880`；DB 测试对 `claude-desktop` 行也用同一 `anthropic()` 夹具，`dao/providers.rs:1545-1551`）。
- 差异字段只在 `meta`：`claudeDesktopMode` / `claudeDesktopModelRoutes` / `apiFormat` / `providerType` / `authBinding`（`provider.rs:399-419,485-506`；前端类型 `types.ts:123-127,180-184`）。
- 官方种子：`claude-official` 与 `claude-desktop-official` 的 `settings_config` **都是 `{"env":{}}`**（`dao/providers_seed.rs:42,52`），Desktop 的注释：「空 env 只是占位；切换该 provider 时会恢复 Claude Desktop 1P 模式」。种子幂等写入走 `ensure_official_seed_by_id(..., AppType::ClaudeDesktop)`（`dao/providers.rs:1384-1434` 测试）。

### 6.3 迁移（**SCHEMA_VERSION 24 → 27，v25/v26/v27 全为本地新增**）
- `mod.rs:55` `SCHEMA_VERSION: i32 = 27`（上游 `database/mod.rs` 为 `24`）。
- **v24→v25【本地】**：把「按 agent 重复录入的 provider」归一为**网关级上游**。新表 `upstreams` / `upstream_endpoints` / `upstream_credentials` / `provider_bindings`：`schema.rs:2031-2089`。身份键是 `(host, protocol)` 而非 host（注释给出 `ai.ctaigw.cn` 三协议共存实例）：`schema.rs:2025-2029`。口径：`claude` 与 `claude-desktop` 同 host 同 URL 的行**合并到同一 upstream/endpoint**，但各自保留一条 `provider_bindings`（PK `(app_type, provider_id)`，`schema.rs:2072-2083`）→ **DB 里两 app 的接入是「同一上游、两条绑定」**。
- **v25→v26【本地】**：`provider_bindings.model_map`（按 app 各一份模型映射）+ `upstream_credentials.secret_kind`（literal/env_ref/managed/none）：`schema.rs:2091-2145`；`secret_kind` 判定 `managed` 时 `provider_id.ends_with("-official")` → `claude-desktop-official` 落 managed：`schema.rs:2158-2181`。
- **v26→v27【本地】**：`create_gateway_tables` —— `gateway_model_overrides` + `gateway_app_profiles`（payload 按 app 存模型配置）：`schema.rs:2097-2116,2118-2120`。
- 数据修正：把 `claude` 侧一行陈旧错名改成 `CTYun`（精确三元组匹配，不命中不动作）：`schema.rs:2524-2533`，注释解释该行 base_url 实为天翼云网关，与 `claude-desktop` 侧同名。
- `dao/providers.rs` 本地 +884 行：新增 `get_provider_upstream_keys`（`:1552-1553,1584`）、`rewrite_endpoint_configs`/`p1_set_base_url` 消费者改写（`:895-899`）、`update_binding_model_map`（`:901-930`，注释明确「同一个网关，claude 里可以映射成 kimi-k2，claude-desktop 里映射成别的」）。
- 连接层【本地新增】：`database/pool.rs`（连接池替代 `Mutex<Connection>`，`mod.rs:78-100`），`busy_timeout`/`foreign_keys`/WAL 逐连接配置（`mod.rs:101-115`）。
- 前端缓存【本地改】：供应商列表刷新 10s → 60s，理由写明是「后台写入/锁竞争主要来源」：`lib/query/queries.ts:84-92`。

### 6.4 用量/统计口径
`claude-desktop` 的 app_type 在**展示口径**上折叠进 `claude`：`services/usage_stats.rs:413-433`（`CASE WHEN column='claude-desktop' THEN 'claude'`），测试明确「desktop 折叠进 claude，不再单列 claude-desktop 桶」：`:3677-3684`。→ 前端用量页看到的是合并桶。

---

## 7. 上游 vs 本地 清单（本笔记相关）

| 项 | 状态 |
|---|---|
| `appConfig.tsx` 全部集合（含 Desktop 在 Mode/Stack、不在 Proxy/MCP/Skills） | 【上游】0 本地 diff |
| `ClaudeDesktopProviderForm.tsx`、`claudeDesktopProviderPresets.ts`、`ClaudeFormFields.tsx`、`ProviderForm.tsx`、`ProvidersList`、`AppsPage.tsx` | 【上游】0 本地 diff |
| `provider.rs`（`ClaudeDesktopMode`/`ClaudeDesktopModelRoute`/`ProviderMeta`）、`dao/providers_seed.rs` | 【上游】 |
| `types.ts` Desktop 字段 | 【上游】 |
| 网关四张表 + v25/v26/v27 + `SCHEMA_VERSION 24→27` + `pool.rs` | 【本地】 |
| `src/components/gateway/*`（AppConfigPage/AppEditorPage/SlotFields/ModelPicker/AllowedModelsEditor/ClientGatewayBar/appModelConfig/modelPickerOptions） | 【本地】 |
| `src/components/bindings/*`、`src/components/suppliers/*`、`types/gateway.ts`、`api/gateway.ts`、`api/suppliers.ts`、`query/suppliers.ts` | 【本地】 |
| 删除 `providers/mode/*`（7 文件）+ `hooks/useModeActions.ts` | 【本地】 |
| Desktop 的 4 档槽位（`CLAUDE_DESKTOP_SLOTS`） | 【本地】新增（上游 Desktop 只有表单里的四行 route 表，无限定「槽位」抽象） |
| 侧栏「网关」分组 + `suppliers`/`bindings` 全局页 | 【本地】`lib/navigation.ts:20-30`、`Sidebar.tsx` |

---

## 8. 留给其他成员 / 未深挖

- Desktop 实际写盘（`claude_desktop_config.rs:150-381`，本地 +56 行）、`mode/controller.rs`、`mode/stack.rs` 的 1P/3P 判定 → 归「本地网关写入逻辑」同事。
- `dao/providers.rs` 中 P1 迁移的逐字段派生（`p1_derive`/`lift_provider_row`）只做了引用，未逐行核。
- 导入导出（backup.rs +4 行）未展开：与两 app 差异无关，未见 Desktop 专属分支。
