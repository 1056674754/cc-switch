# 上游 origin/main（cc-switch v4.0.25）Claude Desktop 配置实现笔记

范围：只读分析 `origin/main` 的 Claude Desktop（`AppType::ClaudeDesktop`, id `claude-desktop`）。行号均取自 `git show origin/main:<path> | nl -ba`。
主文件：`src-tauri/src/claude_desktop_config.rs`（3185 行）。

---

## 1. 配置载体与路径

### 关键常量（claude_desktop_config.rs:21-49）

```rust
21  pub const PROFILE_ID: &str = "00000000-0000-4000-8000-000000157210";
22  pub const PROFILE_NAME: &str = "CC Switch";
25  const CONFIG_FILE: &str = "claude_desktop_config.json";
27  const CONFIG_LIBRARY_DIR: &str = "configLibrary";
28  const GATEWAY_TOKEN_SETTING_KEY: &str = "claude_desktop_gateway_token";
29  const CLAUDE_DESKTOP_PROXY_PREFIX: &str = "/claude-desktop";
32  const MAX_INFERENCE_MODELS: usize = 200;   // Desktop schema maxItems: 200
37  pub const CLAUDE_ROUTE_PREFIX: &str = "claude-";
39  pub const ANTHROPIC_CLAUDE_ROUTE_PREFIX: &str = "anthropic/claude-";
42  pub const ONE_M_CONTEXT_MARKER: &str = "[1m]";
44  const CURRENT_OPUS_ROUTE_ID: &str = "claude-opus-5";
45  const LEGACY_OPUS_ROUTE_ID: &str = "claude-opus-4-8";
48  const LEGACY_HAIKU_ROUTE_ID: &str = "claude-haiku-4-5";
49  const CURRENT_HAIKU_ROUTE_ID: &str = "claude-haiku-5-5";
```

### 真实路径（三个平台）

`paths_from_dirs`（claude_desktop_config.rs:1672-1686）定义全部 5 个受管文件：

```rust
1674  let config_library_path = threep_dir.join(CONFIG_LIBRARY_DIR);
1675  let profile_path = config_library_path.join(format!("{PROFILE_ID}.json"));
1676  let meta_path = config_library_path.join("_meta.json");
1679  normal_config_path: normal_dir.join(CONFIG_FILE),      // “Claude” 目录
1680  threep_config_path: threep_dir.join(CONFIG_FILE),      // “Claude-3p” 目录
```

- macOS（:1624-1628）：`~/Library/Application Support/Claude/claude_desktop_config.json`、`~/Library/Application Support/Claude-3p/claude_desktop_config.json`、`.../Claude-3p/configLibrary/_meta.json`、`.../Claude-3p/configLibrary/00000000-0000-4000-8000-000000157210.json`。
- Windows（:1631-1669）：`%LOCALAPPDATA%\Claude\claude_desktop_config.json`、`%LOCALAPPDATA%\Claude-3p\...`；`pick_windows_claude_dir` 在精确名不存在时模糊匹配 `Claude*`（`-3p` 归属区分，:1647-1670）。
- Linux（:1592-1621）：`$XDG_CONFIG_HOME`（绝对路径才算）/ `~/.config` → `Claude`、`Claude-3p`；Flatpak 内固定用宿主 `~/.config`（:1604-1606），不跟随沙箱被替换的 `XDG_CONFIG_HOME`。
- 平台白名单：仅 macOS/Windows/Linux，其它平台 `get_status` 返回 `supported:false`（:220-236, :1555-1557, :1688-1695）。
- 用户手册也列了同一组路径：`docs/user-manual/zh/2-providers/2.6-claude-desktop.md:220-251`。

### configLibrary 是什么

就是 **Claude Desktop 3P 的多 profile 目录**：`<Claude-3p>/configLibrary/` 下每个 profile 一个 `<uuid>.json`，`_meta.json` 是索引（`entries: [{id,name}]` + `appliedId` 指向当前生效项）。上游 CC Switch 固定占用
`00000000-0000-4000-8000-000000157210.json`，显示名 `CC Switch`，并通过 `_meta.json` 的 `appliedId` 让它生效（MetaPatch，:1417-1478）：

```rust
1444  _ => entries.push(json!({ "id": PROFILE_ID, "name": PROFILE_NAME })),
1447  Some(slot) => *slot = json!(PROFILE_ID),
```

profile 文件长什么样（`build_gateway_profile`，:1480-1500 + 测试 :1963-1994 断言）：

```json
{
  "coworkEgressAllowedHosts": ["*"],
  "disableDeploymentModeChooser": true,
  "inferenceGatewayApiKey": "...",
  "inferenceGatewayAuthScheme": "bearer",
  "inferenceGatewayBaseUrl": "https://gateway.example.com",
  "inferenceProvider": "gateway"
  // 可选 inferenceModels: [...]
}
```

另有两个 `claude_desktop_config.json` 只写 `deploymentMode`：第三方 `"3p"`（:1359-1364），官方 `"1p"`（:1303-1309）。

### 原子写 / 备份 / 模式落定

- 4 个文件（两个 config 的 `deploymentMode`、profile、`_meta`）作为**一次操作**写入；`target`（直连指针、模式状态）在文件全写完后落定，任一解析失败都不写，半途崩溃下次补完（`write_desktop_files`，:1342-1357；`operation::run` + `lock_app`）。
- 首次写前备份：`<device>/backups/live-first-write/`，每文件只备份一次（`live/engine.rs:52-53, 161-181`）。
- 含 Key 的 profile 用 `LiveFile::private`（0600 权限）；config/_meta 用 `shared`（`live/engine.rs:57-78`；`claude_desktop_config.rs:1279, 1335`）。
- 每次写 profile 前按 floor 清关键字段再 set（`gateway_profile_patch`，:1395-1415；floor 定义 `live/floor.rs:187-205`）：

```rust
188  pub const DESKTOP_PROFILE_FLOOR: &[&str] = &[
189    "inferenceProvider", "inferenceGatewayBaseUrl", "inferenceGatewayApiKey",
190    "inferenceGatewayAuthScheme", "inferenceModels",
195    "modelDiscoveryEnabled",
196  ];
204  pub const DESKTOP_PROFILE_SEED: &[&str] = &["disableDeploymentModeChooser", "coworkEgressAllowedHosts"];
```

策略键（`disableDeploymentModeChooser`、`coworkEgressAllowedHosts`）只在缺失时播种，用户可自行收紧（floor.rs:202-205 注释）。

### 是否需要重启

是。Desktop 只在启动时读配置；切换成功提示文案即「切换成功，重启 Claude Desktop 后生效」（`src/hooks/useProviderActions.ts:264-267`），用户手册同（`docs/user-manual/zh/2-providers/2.6-claude-desktop.md:22, 89`；`README_ZH.md:322, 441`）。模型映射模式还要求 CC Switch 保持运行（README_ZH.md:441）。

- 安装检测：没有可调命令，靠数据目录存在或 macOS app bundle（`detect_installed`，:1536-1553）。
- 编辑器/预览：Desktop 没有专用 JSON 编辑器；`get_config_dir` / `open_config_folder` 对 Desktop 打开的就是 `configLibrary` 目录（`src-tauri/src/commands/config.rs:200-233`）。健康检查走 `get_claude_desktop_status`（:219-303）+ 前端 5s 轮询（`src/components/providers/mode/useDesktopConfigMessages.ts:10-40`），检查项：平台不支持、`staleRawModels`、`missingRouteMappings`、`gatewayTokenConfigured`、`expectedBaseUrl != actualBaseUrl`。状态字段见 `ClaudeDesktopStatus`（:105-122）。

---

## 2. Provider 数据模型（DB / JSON 字段）

### 模式与路由结构（`src-tauri/src/provider.rs`）

```rust
399  /// Claude Desktop 3P 写入模式。
402  pub enum ClaudeDesktopMode { Direct, Proxy }        // serde rename_all = "lowercase"
407  /// Claude Desktop 本地路由模式下暴露给 Desktop 的安全模型路由。
410  pub struct ClaudeDesktopModelRoute {
411    /// 真实上游模型名，只保存在 CC Switch 内部，不写入 Claude Desktop profile。
412    pub model: String,
414    #[serde(rename = "labelOverride")] pub label_override: Option<String>,
417    #[serde(rename = "supports1m")]    pub supports_1m: Option<bool>,
418  }
```

meta 字段（provider.rs:497-506）：

```rust
498  #[serde(rename = "claudeDesktopMode")]        pub claude_desktop_mode: Option<ClaudeDesktopMode>,
503  #[serde(rename = "claudeDesktopModelRoutes")] pub claude_desktop_model_routes: HashMap<String, ClaudeDesktopModelRoute>,
```

- 缺省模式 = Direct（`provider_mode`，claude_desktop_config.rs:346-352）。
- 官方供应商：`CLAUDE_DESKTOP_OFFICIAL_PROVIDER_ID = "claude-desktop-official"`（`src-tauri/src/database/dao/providers_seed.rs:14`），种子名 `Claude Desktop Official`，`settings_config = {"env":{}}`，官网 `https://claude.ai/download`（providers_seed.rs:44-53）；启动时幂等种入（providers.rs:920-940），`ensure_claude_desktop_official_provider` 命令兜底（commands/provider.rs:376-385）。
- 凭据来自与 Claude Code 同形的 `settings_config.env`：`ANTHROPIC_BASE_URL` + `ANTHROPIC_AUTH_TOKEN`（`direct_gateway_credentials`，:412-456）。直连校验还要求 `apiFormat` 为空或 `anthropic`、不能是 proxy 卡、不能是 `github_copilot/codex_oauth/xai_oauth`、不能 `isFullUrl`（`validate_direct_provider`，:458-516）。
- 需路由的判定：卡片 `claudeDesktopMode=proxy` **或** 托管 OAuth（`needs_routing`，:325-329）；卡片上的 mode 只描述卡片，不再决定 Desktop 全局模式（模式在 `live-state.json`，见 `mode::controller`）。
- 从 Claude Code 一键导入：可直连的进 direct，否则给建议映射进 proxy（`commands/provider.rs:323-374`；映射生成 `suggested_claude_desktop_routes` :428-540，`[1M]` 后缀翻译成 `supports1m`，:462-482）。

---

## 3. direct（直连）模式写什么

入口：`apply_provider` → `write_direct`（:141-180）→ `direct_target`（:159-164）→ `direct_profile`（:1181-1191）。

写入 profile 的字段清单（`build_gateway_profile`，:1480-1500）：

| 字段 | 值 | 行 |
|---|---|---|
| `inferenceProvider` | `"gateway"` | :1491 |
| `inferenceGatewayBaseUrl` | provider 的 `env.ANTHROPIC_BASE_URL`（原样） | :1184, :1490 |
| `inferenceGatewayApiKey` | provider 的 `env.ANTHROPIC_AUTH_TOKEN` | :1184, :1488 |
| `inferenceGatewayAuthScheme` | `"bearer"` | :1489 |
| `disableDeploymentModeChooser` | `true` | :1487 |
| `coworkEgressAllowedHosts` | `["*"]` | :1486 |
| `inferenceModels` | 可选；仅当卡片配了 `claudeDesktopModelRoutes` 时写 | :1194-1197, :1494-1497 |

- **没有** `mcpServers`、`skills`、`prompts`、`project` 等任何键；Desktop 3P 不走 CC Switch 的 MCP/Skills 同步（`docs/user-manual/zh/2-providers/2.6-claude-desktop.md:25`；`src-tauri/src/app_config.rs:52,1020,202`）。
- 直连模式**不能把 route 映射到不同上游模型**：`route_id` 必须 claude-safe，且 `route.model` 为空或等于 route_id，否则报错让用户改用路由（direct_inference_model_specs，:619-680，错误文案 :648-656）。
- `inferenceModels` 条目形态（`inference_model_json`，:381-397）：无 1M/无 label 时是纯字符串；否则对象，且 1M 会额外写 `prefer1m:true`：

```rust
383  let mut item = json!({ "name": spec.name });
385  item["labelOverride"] = json!(label_override);
388  item["supports1m"] = json!(true);
391  item["prefer1m"] = json!(true);   // 只列原名，内嵌 CLI 自动补 [1m]
```

- 档位排序 sonnet→opus→fable→haiku（`claude_role_rank`，:1040-1052；排序 :670-679），保证菜单顺序稳定。
- 测试断言实际落盘内容：:1963-1994（无模型时不写 `inferenceModels`）、:1997-2014（`[{"name":"claude-sonnet-4-6","supports1m":true,"prefer1m":true}]`）。

---

## 4. proxy（模型映射/本地路由）模式写什么

### 入口与地址

- 路由 profile：`route_profile(db, route, proxy_url, stack)`（:1202-1253）。
- 网关地址：`gateway_base_url(proxy_url)` = `proxy_url` + `/claude-desktop`（:1194-1200）；实际从 DB 取监听地址：`proxy_gateway_base_url_from_db`（:1167-1179），端口 0 时报错「请先启动本地代理或使用固定端口」。
- 测试实际值：`http://127.0.0.1:15721/claude-desktop`（:2251-2254）。
- 网关 token：`get_or_create_gateway_token`（:399-410），存在 setting `claude_desktop_gateway_token`，首次生成 `ccs-<uuid-simple>`，写进 `inferenceGatewayApiKey`（:1248）；测试断言以 `ccs-` 开头且不是上游 Key（:2257-2261）。
- 路由 profile 额外写 `modelDiscoveryEnabled: true`（:1251），只为让本地 `/v1/models` 的说明生效（注释 :1209-1211）。

### 模型列表（inferenceModels）

来自 `proxy_model_routes(route)`（:691-781）的 `route_id`，再链上聚合模型的 `DesktopStackModel`（:1233-1237）；超过 200 截断并 warn（:1239-1245）：

```rust
1228  .map(|route| InferenceModelSpec { name: route.route_id, label_override: route.label_override, supports_1m: route.supports_1m })
1234  .chain(stack.iter().map(|model| InferenceModelSpec { name: model.id.clone(), label_override: Some(model.label.clone()), ... }))
```

### route id 与「槽位」

- 默认 4 个角色 route（`DEFAULT_PROXY_ROUTES`，:60-86）：`claude-sonnet-5`(1M) / `claude-opus-5`(1M) / `claude-haiku-4-5`(1M) / `claude-fable-5`(1M)，各带 `env_key`（`ANTHROPIC_DEFAULT_*_MODEL`）。fable 放最后是为了 `next_catalog_safe_route_id` 借用角色名时按 sonnet→opus→haiku 顺序（:76-80 注释）。
- 前端角色表 `CLAUDE_DESKTOP_ROLE_ROUTE_IDS`（`src/config/claudeDesktopProviderPresets.ts:35-40`）：sonnet=`claude-sonnet-5`、opus=`claude-opus-5`、fable=`claude-fable-5`、haiku=`claude-haiku-4-5`；UI 行序 `ROLE_ORDER = ["sonnet","opus","fable","haiku"]`（`ClaudeDesktopProviderForm.tsx:126`），即**固定 4 行角色槽**。
- **「1+4 槽位」不是上游概念**：上游只有 4 个固定角色 route（第一个即 Desktop 的默认模型，见 :1206 注释「第一项是默认模型」），没有独立的 default 槽、也没有 `slots` 结构（DB 里是 `claudeDesktopModelRoutes` map）。`1+4 槽位` 出现在本地 diy 分支（HEAD）：`src-tauri/src/proxy/handlers.rs:218`、`src-tauri/src/claude_desktop_config.rs:102`、`src-tauri/src/services/gateway_route.rs:163`（应用配置 `slots` + 角色名），属本地网关改造新增，非上游。
- 发布名修正（`published_route_id`，:794-808）：Opus/Sonnet 键若目标是 5.x 会换成目录里的 point-release 名（复用 `live/project/claude::point_release_client_id`，:799）；Haiku 键（`claude-haiku-4-5`）在目标不是 4.x/3.x 旧 Haiku 时发布成 `claude-haiku-5-5`（:801-805，`is_pre_5_claude_haiku` :810-816）。
- 非法 route id 修复：借用未被占用的默认 route id，再退化为 `claude-sonnet-5-r2/r3...`（`next_catalog_safe_route_id`，:818-842）。
- 直连卡留空映射时走默认角色路由（`uses_default_proxy_routes`，:682-689；默认表 upstream=route_id、supports_1m=false，:694-707）。
- 发布安全名校验：`is_claude_safe_model_id`（:354-379）——必须 `claude-` 或 `anthropic/claude-` 前缀、含 `[1m]` 直接拒、角色白名单 `sonnet-/opus-/haiku-/fable-` 且其后非空；注释说明 Desktop 1.12603.1+ 的 fail-all validator 已收 fable（app.asar 白名单 `["sonnet","opus","haiku","fable","mythos"]`），mythos 未公开不暴露。
- 模型列表 `/v1/models`：`model_list_response`（:862-901），每条 `{type:"model", id, created_at:"2024-01-01T00:00:00Z", supports1m?, description}`，description = `<供应商名> · <上游模型> · <窗口>` 截断 100 字（:844-860）。

### 请求侧映射

- 路由挂载：`src-tauri/src/proxy/server.rs:309-317` → `GET /claude-desktop/v1/models`、`POST /claude-desktop/v1/messages`。
- handler：`handle_claude_desktop_messages` / `handle_claude_desktop_models`（`proxy/handlers.rs:253-312`），先校验网关 token，再以 `AppType::ClaudeDesktop` + `strip_prefix Some("/claude-desktop")` 进入通用处理。
- token 校验：`validate_claude_desktop_gateway_auth`（handlers.rs:451-482），常量时间比较 `ct_eq`；`extract_gateway_token` 接受 `Authorization: Bearer <token>` 或 `x-api-key`（:484-500+）。`/claude-desktop/` 前缀被 browser_guard 视为 token 保护、豁免浏览器头检查（`proxy/browser_guard.rs:25-27`）。
- 请求内 model 映射（`map_proxy_request_model`，:903-973）：
  1) 取 `body.model`，剥掉本地 `[1m]` 标记（:917, :989-999）；
  2) 精确匹配 route id（:925-927）；Opus 5/4-8 互为别名（:1014-1020）；
  3) 老 profile 的原键经 `legacy_raw_route_upstream_model` 查原始映射表（:1001-1012）；
  4) 角色关键词回落（opus/haiku/fable/sonnet，:936-962, :1025-1038），fable 无独立档时降级到 opus 档（:949-960）；
  5) 都不中 → 400 `Claude Desktop 模型路由未配置: <model>`（:963-969）。
- 命中后 `body["model"] = upstream_model`（:971），再做上游怪癖整理 `normalize_upstream_request`（:977-987；目前只有 MiMo Anthropic thinking 历史补偿，:1054-1165）。
- 转发层调用点：`proxy/forwarder.rs:1466-1476`——ClaudeDesktop 必须映射，未知 route 直接报错不兜底；聚合解析出来的模型只做 normalize（`keeps_resolved_model`）。

---

## 5. Stack（聚合）对 Desktop 的支持

- 能力开关包含 Desktop：`supports_stack` 匹配 `Claude | Codex | ClaudeDesktop`（`src-tauri/src/mode/stack.rs:53-59`）；路由应用清单也含 Desktop（`mode/controller.rs:47-55`，注释：Desktop 经同一路由服务转发，但没有故障转移）。
- Desktop 的 Stack 模型 id：`claude-fable-ccs-<6 位数字>`（stack.rs:40-45）：

```rust
44  const DESKTOP_PREFIX: &str = "claude-fable-ccs-";
45  const DESKTOP_CODE_DIGITS: usize = 6;
```

  理由（注释 :40-43）：Desktop 选择器只收含 `claude` 等字样、不含厂商黑名单的 id，且末尾 8 位数字会被当日期剥掉，所以只放 6 位数字；`fable` 前缀让 Desktop 给 effort 选择器。
- 编码：`encode` 对 Desktop 忽略 `one_m`（1M 写在条目里，id 不带标记，:116-127）；`desktop_code` 用 FNV-1a 散列 key+model 取 6 位（:129-142）。
- `DesktopStackModel` 结构（:381-394）：`id`（发布名）/`upstream`/`label`（`<供应商名> / <模型显示名>`）/`provider_name`/`supports_1m`。
- 发布集合：`desktop_models`（:396-423，按 upstream 去重，任一标 1M 即 1M，映射表不完整则不发布）、`desktop_published`（:425-440，路由那家不发布、id 撞号后一个丢弃并 warn）；`desktop_published_now` 从 DB 现算（:622-631）；`resolve_desktop` 反解回上游（:744-790）。
- 写入：`route_profile` 把聚合模型排在路由那家后面（:1206-1207 注释、:1233-1237），截断 200；`/v1/models` 也会把聚合模型补进响应（带 `display_name`，`proxy/handlers.rs:282-310`）。
- 注意：`README_ZH.md:346-349` 的功能矩阵把 Desktop 的「聚合」列为 `–`（只有「模型映射时」本地路由、托盘/用量），与后端代码支持 Stack 不一致，属文档滞后；写报告时不要照抄矩阵。

---

## 6. Desktop 与 Claude Code 的差异（不支持/不同）

| 维度 | Claude Desktop | 依据 |
|---|---|---|
| 配置文件 | `<Claude-3p>/configLibrary/<uuid>.json` + `_meta.json` + 两个 `claude_desktop_config.json` 的 `deploymentMode` | claude_desktop_config.rs:1672-1686 |
| 生效方式 | **必须完全退出并重启 Desktop**；模型映射时 CC Switch 需保持运行 | README_ZH.md:441；manual:22,89；useProviderActions.ts:264-267 |
| MCP | 不支持（3P profile 不走 MCP 同步；Desktop 页打开的 MCP 面板作用于 Claude Code） | app_config.rs:52, 1020；README_ZH.md:366 |
| Skills | 不支持同步 | app_config.rs:202；README_ZH.md:349,366；manual:25 |
| 提示词 | 不适用（README 矩阵 `–`） | README_ZH.md:349 |
| 会话 | 不支持（矩阵 `–`） | README_ZH.md:349 |
| 用量统计 | 仅模型映射（经本地路由）时有；日志/计价继承 `claude`，`app_type="claude-desktop"` | README_ZH.md:349；proxy/usage/logger.rs:366-367, 659-664；response_processor.rs:1378-1383 |
| 项目保存 | 有自己的项目快照槽位，但**只保存供应商**（不存 MCP/Skills/提示词） | README_ZH.md:372；docs/release-notes/v3.17.0-zh.md:74 |
| 本地路由 | 有，但**无故障转移、无 `proxy_config` 行**；`has_route_mode()` 单独放行 | app_config.rs:534-539；PROXY_APPS 注释 controller.rs:47-48 |
| 托盘切换 | 支持 | README_ZH.md:321,346-349 |
| 直连模式限制 | 只能写原生 Anthropic Messages API，不能做模型映射、不支持托管 OAuth 类供应商（它们必须走路由） | validate_direct_provider :458-516；needs_routing :321-329 |
| 入口/形态 | 与 Claude Code 是两个应用，凭据形态沿用 Anthropic `env`（便于从 CC 导入），但落盘是 3P profile | provider.rs:1787-1798 测试注释；manual:14 |

---

## 7. presets 与切换生效机制

### 预设（`src/config/claudeDesktopProviderPresets.ts`）

- 形态差异（文件头注释 :1-10）：`baseUrl` 是顶级字段（不是 `settingsConfig.env.ANTHROPIC_BASE_URL`）；模型信息表达为「Desktop 可见 routeId → upstreamModel」。
- 结构（:15-69）：

```ts
21  export interface ClaudeDesktopRoutePreset { routeId; upstreamModel; labelOverride?; supports1m: boolean }
44  export interface ClaudeDesktopProviderPreset {
54    baseUrl: string;  apiKeyField?: "ANTHROPIC_AUTH_TOKEN" | "ANTHROPIC_API_KEY";
57    mode: "direct" | "proxy";  apiFormat?: "anthropic"|"openai_chat"|"openai_responses"|"gemini_native";
59    modelRoutes?: ClaudeDesktopRoutePreset[];  providerType?: "github_copilot"|"codex_oauth"|"xai_oauth";
61    requiresOAuth?: boolean;
```

- 角色 route 表 `CLAUDE_DESKTOP_ROLE_ROUTE_IDS`（:35-40，见 §4）是所有工厂/表单/后端借名的唯一来源（注释 :28-34 记录了 fail-all 白名单与时间点）。
- 路由工厂：`passthroughRoutes`（:77-94，直连同名）、`mappedRoutes`（:96-117，非 Claude 上游）、`brandedRoutes`（:119-140，品牌名只进 `labelOverride`/`upstreamModel`）。
- 1M 规则：Haiku 4.x 只有 200K，声明 1M 的预设里 Haiku 档不能跟标 1M（:71-75, :88-93）。
- 实测预设数组从 :150 起；部分非 Claude 品牌用 `CLAUDE_DESKTOP_ROLE_ROUTE_IDS.fable` + `labelOverride`（如 :1753-1756 MiniMax-M3）。
- 表单（`src/components/providers/forms/ClaudeDesktopProviderForm.tsx`）：`初始 mode` :267、`apiFormat` :268-269、`effectiveMode` 托管 OAuth 强制 proxy :420；模型映射 4 行（角色只读 + 菜单显示名 + 实际请求模型 + 1M），行渲染 :1276-1358；直连模式是「模型列表」可加行（:1363+）。

### 切换生效机制

- 直连切换：`services/provider/live.rs:439-447` → `claude_desktop_config::apply_provider` → 写 3p profile + `_meta`（`apply_provider` :141-148）。
- 路由接入：`mode/controller.rs:493-513` → `route_profile` + `run(DesktopTarget::Profile)`；退出/写回直连：:609-624（写不出直连 profile 时退回 `Official`，即恢复 1P）。
- 官方恢复：两个 config 改 `deploymentMode:"1p"`、清旧版 `enterpriseConfig` 网关键（:1366-1373, :1291-1340）、profile 保留文件但按 floor 清字段、`_meta` 移除条目（:1417-1478）。
- 启动迁移：旧版把模型映射卡当直连指针时，`legacy_mapping_card` 检出并在启动时转入路由模式（:331-344；controller.rs:1729）；启动接代理失败则退回直连并记录 `StartupAttachFailure`（controller.rs:1749-1774）。
- 路由服务是共享的：`/claude-desktop` 只是同一 HTTP 服务的独立 namespace + 独立 token 鉴权，`select_providers("claude-desktop")` 选供应商（handlers.rs:274-279）。

---

## 未确认 / 边界

- 未在代码中找到 Desktop 专属的 JSON 配置编辑器预览（只有「打开配置目录」）：`commands/config.rs:200-233` 是唯一入口；若前端另有 preview 组件，属前端同事范围。
- `README_ZH.md` 功能矩阵的「聚合 = –」与后端 Stack 支持矛盾（见 §5），未查证是否有意限制 UI 入口。
- Desktop profile 的完整 Desktop 官方 schema 未在仓库内（仅注释引用 `maxItems: 200`、fail-all validator 白名单）；其余字段靠测试与 patch 语义推断，未逐一验证。
- 未跑编译/测试，均为静态阅读；行号以 origin/main 当前提交为准。
