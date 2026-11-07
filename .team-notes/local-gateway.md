# 本地 diy 分支网关改造：Claude Code vs Claude Desktop 的「接管 / 释放」写入路径

调查范围：`/Users/song/dev_ai/cc-switch-src`，分支 `diy`，基线 `origin/main`（v4.0.25，约 60 个本地提交）。
方法：`git diff origin/main..HEAD -- <file>` + `git show <commit> --stat` + 读 HEAD 源码；**只读，未跑 cargo/pnpm**。
标注约定：**【上游】** = origin/main 已有；**【本地】** = diy 分支新增/改动。

> **快照说明（重要）**：本笔记的行号取自 2026-11-05T05:00Z 左右的工作区状态（`diy` HEAD `7538d3d7` + 当时工作区）。
> 我在写笔记期间检测到**另一个会话正在并发改动 13 个未提交文件**（供应商「启用/停用」+ 删除供应商 + v28 迁移：`commands/suppliers.rs`、`database/dao/providers.rs`、`database/mod.rs`、`database/schema.rs`、`error.rs`、`lib.rs`、`proxy/{error,error_mapper,handler_context,handlers,provider_router}.rs`、`components/suppliers/SupplierPage.tsx`、`lib/api/suppliers.ts`）。
> **CC / Desktop 结论所依赖的文件全部未被这次并发改动触碰**（`mode/controller.rs`、`live/project/claude.rs`、`live/floor.rs`、`claude_desktop_config.rs`、`services/gateway_*.rs`、`gateway_route.rs`、`commands/proxy.rs`、`live/project/*`），故 §2/§3/§4/§5 的行号稳定有效。
> 受影响的是 §1 架构/供应商部分：`services/gateway_catalog.rs`（737 行）不变；`commands/suppliers.rs` 现为 1348 行、命令行号已下移（见 §1.1 括号内的「当前行号」）；`database/schema.rs` 新增 v28，`create_*` 系列行号下移约 +6（见 §1.2）。并发改动不在我的任务范围（不越界）。

---

## 0. TL;DR

| | Claude Code（cc） | Claude Desktop |
|---|---|---|
| 写哪个文件 | `~/.claude/settings.json`（`config.rs:297`，可被 override dir 改） | `Claude/claude_desktop_config.json`、`Claude-3p/claude_desktop_config.json`、`Claude-3p/configLibrary/<PROFILE_ID>.json`、`.../configLibrary/_meta.json` |
| 写什么 | `env`：`ANTHROPIC_BASE_URL` + 占位凭据 + `ANTHROPIC_MODEL` + 4×`ANTHROPIC_DEFAULT_*_MODEL`（+`_NAME`）；`exclusive` 沿用路由那家的兼容开关 | profile：`inferenceProvider` / `inferenceGatewayBaseUrl`（`/claude-desktop` 前缀）/`inferenceGatewayApiKey`（`ccs-…`）/`inferenceGatewayAuthScheme`/`inferenceModels`/`modelDiscoveryEnabled` + 两个 config 的 `deploymentMode` |
| 模型名形态 | 模型引用原样（`<供应商名>/<模型 id>`，1M 加 `[1M]`） | `inferenceModels` 里发布引用；4 个固定 role route id（`claude-*-5`）由槽位在**请求时**映射 |
| 槽位回退 | 槽位没配 → 回落到 **默认模型引用** | 槽位没配 → **不回落 default**，回落到上游 provider 的模型映射表 |
| 释放 | 清 `ANTHROPIC_*` 等「地板键」→ 写回直连供应商（或清空） | `deploymentMode` 改回 `1p`、清 profile 的 6 个 floor 键、`_meta.json` 摘条目 |
| 触发命令 | `adopt_app_to_gateway` / `release_app_from_gateway`（`commands/proxy.rs:98-119`）→ `controller::enter/exit` | 同左 |

---

## 1. 本地新增的网关架构（item 1）

### 1.1 命令层
`src-tauri/src/commands/gateway.rs`（**本地新增，340 行**）
- `get_gateway_catalog` — L13-18，返回整份目录（引用 + 展示信息 + 端点 + 来源）。
- `get/set_app_model_config` — L304-339。`set` 先按目录校验（L324-327），落库后**已接管就立刻重写客户端**：
  ```rust
  // commands/gateway.rs:332-338
  if crate::mode::controller::PROXY_APPS.contains(&app) {
      crate::mode::controller::resync_route(state.inner(), &app).await
  ```
- `migrate_gateway_from_stack` — L296-302（旧 Stack 配置一次性翻译，幂等）。
- 模型覆盖（改名/隐藏/还原）— L250-294。
- 累加式 app 的「接管/释放」六对命令：OpenCode L20-56、Hermes L58-95、OpenClaw L97-134、Pi L136-171、OMP L173-209、DSH L211-246。这些**不走** `controller`，各写自己的配置文件（见 `services/{opencode,hermes,openclaw,pi,omp,dsh}_gateway.rs`）。
- 命令注册：`commands/mod.rs`（本地 +4 行，`mod gateway; mod suppliers;` 与 `pub use`）。

`src-tauri/src/commands/suppliers.rs`（**本地新增，HEAD 1166 行；工作区并发改动后 1348 行**）：供应商（网关级上游）总览与编辑
- 模块说明 L1-27：供应商 = **一个确切的上游 API（服务根 + 上游协议）**；下游协议由 app 决定，上游协议由 `provider.meta.apiFormat` 决定。
- 命令（快照行号 → 并发改动后的当前行号）：`get_suppliers_overview` L436→L449、`update_supplier_base_url` L447→L460、`update_supplier_secret` L510→L523、`adopt_supplier` L578→L742、`create_supplier` L698→L880、`update_binding_model_map` L787→L969。
- `BindingDetail.model_map` L36-47：槽位化 JSON（haiku/sonnet/opus/fable/subagent/default，空对象=没配）。

### 1.2 服务层（`src-tauri/src/services/`，**均为本地新增**）
- `gateway_catalog.rs`（737 行）
  - `CatalogViewEntry` L246-259：`reference`（模型引用）、`upstream_id/upstream_name`、`model_id`、`display_name`、`one_m`、`window`、`endpoint_ids`、`sources`。
  - 引用构造 L429-442：`reference: format!("{upstream_name}/{}", entry.model_id)`；重名上游加 `（2）`（`unique_upstream_names` L262-278）。
  - `resolve_reference` L445-450：按引用精确查，查不到由调用方报错、**不回落**。
  - 目录派生 `derive_entries` L327-393（先对账 `provider_bindings`，逐 app 读 provider 行，来源串 `format!("{}:{}", app.as_str(), provider.name)`，即 `claude:CTYun` / `claude-desktop:MAP`）；`raw_models` L55-96（Claude 用 `stack::claude_models`，Desktop 用 `stack::desktop_models`）；`catalog_view` L394-443（套用人工覆盖/隐藏）；`catalog_view_cached` L299-325 + `invalidate_catalog_cache` L291。
  - **上游发现（`/v1/models`）—— 目录的第三个来源**（`services/gateway_model_sync.rs`，本地新增）：`fetch_target` 按协议偏好挑端点（openai_chat > openai_responses > anthropic > gemini），只认 `secret_kind='literal'` 的明文密钥（托管登录 / `env_ref` 直接报错）；baseURL 被填成整条请求地址时（`.../v1/chat/completions`）按 full URL 回推根路径 → `model_fetch::fetch_models` → `store_discovered_models` 重写 `gateway_upstream_models`（schema v29）；新出现的模型写 `hidden=true` 覆盖（= 未启用），用户点「加入」才 `hidden=false`。`derive_entries` 把发现结果并进目录，`sources` 取该上游已绑定 app 的 `app:providerName`，所以启用后与派生条目一样可路由 / 可发布。命令 `fetch_supplier_models(upstreamIds[])`（`commands/gateway.rs`）：供应商就是上游，同一家上游在库里可能被按 app 派生成多行（Codex 订阅 = `chatgpt.com` 行 + codex 官方行），`plan_refresh` 先按真实上游身份（`literal:<baseURL>` / `codex_oauth:<account>`）归并，**一家只发一次请求**，结果写回它对应的每一行；`get_gateway_catalog` 增 `includeHidden`，只给供应商弹窗左栏用，共享缓存 `["gateway","catalog"]` 仍是可见目录。**卡片内按 modelId 归并**：一张供应商卡可能含多条上游（Codex 订阅 = `chatgpt.com` + 官方托管上游），两条上游各回一份同名模型；弹窗左栏只列「未启用」且按模型名去重，启用状态取任一条，点加入/移出/改名会写该模型在所有上游条目上的覆盖（`SupplierEditDialog.tsx` ModelSection）。**托管登录另走一条**：`resolve_source` 在没有明文密钥时读 `provider_bindings → providers.meta.authBinding`，`codex_oauth` 账号用 `CodexOAuthState` 的 access_token 打 `chatgpt.com/backend-api/codex/models`（`services/codex_oauth_models.rs`，不是 OpenAI 兼容端点）；kimi/copilot 等其它托管登录给出明确错误。Codex 的网关 key 存在 `settingsConfig.auth.OPENAI_API_KEY`，`dao::providers::credential_values` 已把 `auth` 纳入取证（否则自定义网关的 Codex 行会被当成「没有密钥」）。**实测**（2026-11-07，库副本 + 真实订阅账号）：Codex 订阅返回 10 个模型，claude-desktop 的 `Codex` 供应商刷新后目录 `含未启用 10 条 / 可见 4 条`（原有 4 条来自 `claudeDesktopModelRoutes` 四个槽位，其余 6 条默认未启用）。
- `gateway_config.rs`（289 行）
  - `AppModelConfig` L18-35：`default`（引用）、`slots: BTreeMap<String,String>`、`allowed: Vec<String>`、`routing: Option<Value>`。
  - 校验 `validate_app_model_config` L140-179：引用必须在目录里；`allowed` 非空时 **default 必填且必须在 allowed 里**（L166-177）。
  - 表读写：`get_app_model_config` L45、`set_app_model_config` L63（写 `gateway_app_profiles`，并 `invalidate_catalog_cache`）。
- `gateway_route.rs`（554 行）——**请求侧路由**
  - `resolve_reference_target` L21-84：`<供应商名>/<模型 id>` → `StackTarget{provider, upstream_model, original_model}`；剥 `[1M]`（L30-33）；供应商名不认识 → `None` 交默认路由，模型名不认识 → `Err`（L42-51）。
  - `pick_configured_reference` L89-140 / `resolve_configured_model` L143-159：累加式 app 的**裸模型 id** 按「默认 + allowed + slots」挑唯一引用，歧义/不在允许列表都报错。
  - `desktop_slot_reference` L165-181 + `resolve_desktop_role_target` L185-193：Desktop 的 **route id → 槽位引用**（见 §3）。
- `gateway_migration.rs`（323 行）：`migrate_all` L156-166（只对 Claude / ClaudeDesktop / Codex 三个 app 翻译旧 Stack 配置）。
- 迁移：`database/schema.rs`
  - v26 `create_upstream_tables` L2031-2090（当前 L2036-2095）：`upstreams` / `upstream_endpoints` / `upstream_credentials`（含 `secret_kind`）/ `provider_bindings`（含 `model_map`）。
  - `migrate_v25_to_v26` L2122-2145；v27 `create_gateway_tables` L2097-2116（当前 L2103-2122；`gateway_model_overrides`、`gateway_app_profiles`）；`migrate_v26_to_v27` L2118-2120（当前 L2124-2126）；分发 L734-738（当前 L738-739）/ L952-958（当前 L956-966，另有并发新增的 v28 `migrate_v27_to_v28` L2134 附近）。
- `database/dao/providers.rs`（本地 +884 行）：`reconcile_provider_bindings`、`rewrite_endpoint_base_url`、`update_binding_model_map`、`rewrite_endpoint_secret`、`get_provider_upstream_keys`、`credential_values`/`primary_credential_value`（凭证指纹）。

### 1.3 网关 token 与 `/v1/models`
- **Claude Desktop gateway token**（【上游已有】）：`claude_desktop_config.rs:28` `GATEWAY_TOKEN_SETTING_KEY = "claude_desktop_gateway_token"`；`get_or_create_gateway_token` L434-445 生成 `ccs-<uuid_simple>` 并存 settings；校验 `proxy/handlers.rs:570-609`（`x-api-key` 或 `Authorization: Bearer`，`ct_eq` 常量时间比较）。
- **CC 侧凭据**：不写真实 token，写占位符 `PROXY_MANAGED`（`live/project/claude.rs:199` `PROXY_TOKEN_PLACEHOLDER`）。
- `/v1/models`：
  - Claude Code 发现请求识别 `proxy/handlers.rs:135-148`（带 `anthropic-version`，或 `limit` 且无 `client_version`）；响应 `claude_model_discovery` L150-214 —— **【本地新增】** 先发布目录里 `sources` 以 `claude:` 开头的条目，`id` = 模型引用（L157-181），再拼旧 Stack 模型（L182-193）。
  - Desktop 入口 `proxy/server.rs:321-322`（`/claude-desktop/v1/models`）；发布目录里 `claude-desktop:` 来源的引用 + `supports1m`/`description`（`proxy/handlers.rs:340-397`，【本地新增】）。
  - 累加式 app 路径前缀入口：`proxy/handlers.rs:1086-1135`（`/opencode`、`/openclaw`、`/hermes`、`/pi`、`/omp`、`/dsh` + `/v1/chat/completions`）。

---

## 2. Claude Code 接管 / 释放（item 2）

### 2.1 触发路径
```
前端 AppConfigPage「接管」→ gatewayApi.adopt(appType)            (src/lib/api/gateway.ts:153-156)
→ adopt_app_to_gateway(app_type)                                (commands/proxy.rs:98-108)
→ mode::controller::enter(state, app, false)                    (controller.rs:870-872)
→ enter_locked → write_proxy(..., AppType::Claude)              (controller.rs:534-571)
```
释放：`release_app_from_gateway`（`commands/proxy.rs:111-119`）→ `controller::exit`（`controller.rs:1014-1027`）→ `exit_locked`（L1052-1087）→ `write_direct`（L676-...，Claude 分支 L682-703）。

### 2.2 写哪个文件
**【上游】** 写 `~/.claude/settings.json`（旧安装回落 `claude.json`）：
- `services/provider/claude_direct.rs:23-24` `LiveFile::private(get_claude_settings_path())`，`run` L79-95 只写这一个文件。
- 路径来源 `config.rs:297-...` `get_claude_settings_path()` = `get_claude_config_dir().join("settings.json")`（默认目录 `config.rs:96` `~/.claude`）。

### 2.3 本地新增的写入逻辑（关键：`ModelConfigSlots`）
`live/project/claude.rs`（**本地 +172 行**，提交 `78213820`）
- `ModelConfigSlots` L290-301：`default: Option<(&str, Option<&str>)>`（引用, 显示名）+ `slots: [Option<...>; 4]`，注释明确顺序 = `Tier::ENV_ORDER`：**haiku、sonnet、opus、fable**（L296-297）。
- `fields()` L307-327（**字段清单的权威实现**）：
  ```rust
  // claude.rs:308-324
  if let Some((reference, name)) = self.default {
      fields.push(("ANTHROPIC_MODEL", reference.to_string()));
      ... fields.push(("ANTHROPIC_MODEL_NAME", name.to_string()));
  }
  for (index, tier) in Tier::ENV_ORDER.into_iter().enumerate() {
      let picked = self.slots[index].or(self.default);   // ← 槽位没配回落 default
      fields.push((tier.model_key(), reference.to_string()));
      ... fields.push((tier.name_key(), name.to_string()));
  }
  ```
- `proxy_projection_from_model_config` L332-367：
  ```rust
  // claude.rs:336-341
  env.insert("ANTHROPIC_BASE_URL", Value::String(proxy_url.to_string()));
  for (key, value) in model_config.fields() { env.insert(key, value); }
  // claude.rs:343-358 → 凭据占位符（AuthToken / Managed{auth_token} 二选一）
  ```
  返回 `ClaudeProjection{ top: Map::new(), env, exclusive: route.exclusive.clone() }`。

**cc 接管写入字段清单**（`settings.json`）
| 字段 | 值 | 位置 |
|---|---|---|
| `env.ANTHROPIC_BASE_URL` | 本地网关地址（`proxy_url`） | claude.rs:336-339 |
| `env.ANTHROPIC_MODEL` | 每 app 配置的 `default` 引用（目录外引用原样写） | claude.rs:309-310 |
| `env.ANTHROPIC_MODEL_NAME` | default 的显示名（`display_name` 或 model_id），空则不写。**本地新增键**：origin/main 只写各档 `ANTHROPIC_DEFAULT_*_MODEL_NAME`，从不写 `ANTHROPIC_MODEL_NAME` | claude.rs:311-313 |
| `env.ANTHROPIC_DEFAULT_HAIKU_MODEL` | slots.haiku；没配 → default 引用 | claude.rs:315-321 + `Tier::model_key` L404-411 |
| `env.ANTHROPIC_DEFAULT_SONNET_MODEL` | 同上（sonnet） | 同上 |
| `env.ANTHROPIC_DEFAULT_OPUS_MODEL` | 同上（opus） | 同上 |
| `env.ANTHROPIC_DEFAULT_FABLE_MODEL` | 同上（fable） | 同上 |
| `env.ANTHROPIC_DEFAULT_{HAIKU,SONNET,OPUS,FABLE}_MODEL_NAME` | 各档显示名，空则不写 | `Tier::name_key` L413-421 |
| `env.ANTHROPIC_AUTH_TOKEN` **或** `env.ANTHROPIC_API_KEY` | 占位符 `PROXY_MANAGED`（二选一，绝不两个都写） | claude.rs:343-358；策略见 `claude_proxy_auth` controller.rs:174-183 |
| `env.*`（供应商独有开关） | 路由那家的 `exclusive` 原样保留 | `projection.exclusive` |
| 顶层 | **不写**（`top: Map::new()`）；`modelPicker` 只在 Stack 模式下由 `with_stack_models` 写（controller.rs:359-395） | claude.rs:359-364 |

补充：
- **1M 模型**：引用尾巴加 `[1M]`，在 controller 侧拼（`controller.rs:324-326` `model.push_str(ONE_M_MARKER_FOR_CLIENT)`；常量 `claude.rs:211` `"[1M]"`）。网关注入前会剥掉（`gateway_route.rs:30-33`）。
- 引用目录外时照样写下去（`controller.rs:335-336` 注释 + 实现），请求时网关报错，不在此吞掉。
- 不写 `CLAUDE_CODE_SUBAGENT_MODEL`（测试断言 `controller.rs:3037` `assert!(live["env"].get("CLAUDE_CODE_SUBAGENT_MODEL").is_none())`）。
- `CLAUDE_CODE_MAX_CONTEXT_TOKENS` 只在 Stack 模式由 `with_stack_models` 写（controller.rs:377-386）；模型配置路径不写。
- 生效链路：`claude_contract` 仅在 **Stack 名单为空** 时采用新写法，否则回落到旧 `proxy_projection`（controller.rs:199-218）。

### 2.4 与上游 proxy 模式的差异（同一文件同一目录，不同写法）
| | 【上游】路由模式 | 【上游】Stack 模式 | 【本地】模型配置接管 |
|---|---|---|---|
| 模型名 | 稳定别名 `claude-haiku-5-5`/`claude-sonnet-5`/`claude-opus-5`/`claude-fable-5`（`claude.rs:203-206`），由代理映射 | 四档都写 Stack id | 写**模型引用**，由网关按 `<供应商名>/<模型名>` 路由 |
| 四档来源 | provider 行 env 各档 + 回落（`route_roles` L505-523） | `stack_default` 一个模型 | 每 app 配置 slots，没配回落 default |
| 显示名/说明 | 写 `_MODEL_NAME` + `_MODEL_DESCRIPTION`（`proxy_model_fields` L546-568） | 写 `_MODEL_NAME` | 只写 `_MODEL_NAME`，不写 description |
| `/model` 菜单 | 不写 `modelPicker`（用内置菜单） | 写 `modelPicker`（replaceBuiltInOptions） | 不写 |
| 子代理 | 写 `CLAUDE_CODE_SUBAGENT_MODEL`（L561-566） | 不写 | 不写 |
| `CLAUDE_CODE_MAX_CONTEXT_TOKENS` | 不写 | 按 Stack 最小窗口写 | 不写 |

### 2.5 释放（写回直连）
`write_direct` Claude 分支 `controller.rs:682-703`：
```rust
let projection = usable_direct(app, direct.as_ref())
    .map(|provider| ClaudeProjection::of(&provider.settings_config));
let mut patch = direct_patch(live_now.claude_exclusive_owner().as_ref(), &projection...);
crate::services::provider::claude_official::strip_from_patch(&mut patch);
// 退出路由（非分离）时把直连官方卡的账号换进登录
crate::services::provider::claude_official::with_login_switch(login_target, || claude_direct::run(...))
```
`direct_patch`（`claude.rs:112-160`）= 「关键字段一律清空 → 写目标供应商的值」：
- 清空 `env` 的「地板键」：前缀 `ANTHROPIC_`、`AWS_`、`VERTEX_REGION_`（`live/floor.rs:34`）；协议选择器 `CLAUDE_CODE_USE_*`（floor.rs:21-29）；具名键 `CLAUDE_CODE_SUBAGENT_MODEL(_FORCE)`、`CLAUDE_CODE_OAUTH_*`、`GOOGLE_APPLICATION_CREDENTIALS` 等（floor.rs:44-55）；以及 `CLAUDE_CODE_SKIP_*_AUTH`（floor.rs:51-58）。
  → 因此**接管期写的 `ANTHROPIC_BASE_URL`、`ANTHROPIC_AUTH_TOKEN/API_KEY`、`ANTHROPIC_MODEL`、`ANTHROPIC_DEFAULT_*_MODEL(_NAME)` 全部被删除**，再写直连供应商自己的值（无直连卡时 = 清空）。
- 清空顶层地板键：`model`、`modelPicker`、`fallbackModel`、`apiKeyHelper`、`apiKey`…（floor.rs:61-81）。
- 独有字段：删「上一家带进来且值没被改过」的（`direct_patch` 的 `remove_if`），再写目标的。

---

## 3. Claude Desktop 接管 / 释放（item 3）

### 3.1 触发路径
同 §2.1，`controller::enter/exit` 的 `AppType::ClaudeDesktop` 分支：
- 写代理：`controller.rs:639-659`
- 写回直连：`controller.rs:760-776` → `claude_desktop_config::run(..., DesktopTarget::Profile(直连 profile) 或 DesktopTarget::Official)`

### 3.2 写哪些文件（macOS 路径）
`claude_desktop_config.rs:1706-1722` `paths_from_dirs`：
- `~/Library/Application Support/Claude/claude_desktop_config.json`（normal）
- `~/Library/Application Support/Claude-3p/claude_desktop_config.json`（3p）
- `~/Library/Application Support/Claude-3p/configLibrary/00000000-0000-4000-8000-000000157210.json`（profile，`PROFILE_ID` L21，`PROFILE_NAME="CC Switch"` L22）
- `.../Claude-3p/configLibrary/_meta.json`（条目/appliedId）
- Windows/Linux 路径另见 L1655-1705。

接管写入 `write_profile_at_paths` L1290-1329：
```rust
// claude_desktop_config.rs:1296-1299
let deployment = deployment_mode_patch("3p");
let profile = gateway_profile_patch(profile);
... // normal/3p 两个 config 写 deploymentMode=3p；profile 写网关键；_meta 登记条目
```

### 3.3 `inferenceModels` 具体 JSON 形状
`build_gateway_profile` L1515-1535（【上游已有】）：
```json
{
  "coworkEgressAllowedHosts": ["*"],
  "disableDeploymentModeChooser": true,
  "inferenceGatewayApiKey": "ccs-<uuid>",
  "inferenceGatewayAuthScheme": "bearer",
  "inferenceGatewayBaseUrl": "http://127.0.0.1:<port>/claude-desktop",
  "inferenceProvider": "gateway",
  "inferenceModels": [ ... ]
}
```
`route_profile`（【上游已有】L1247-1290）额外补：
```rust
// claude_desktop_config.rs:1287-1289
profile["modelDiscoveryEnabled"] = json!(true);
```
`inferenceModels` 元素形状 `inference_model_json` L416-432：
- 无 `labelOverride` 且非 1M → **纯字符串**（就是模型名）；
- 否则对象：`{"name": "<name>", "labelOverride": "<label>"?, "supports1m": true?, "prefer1m": true?}`（`prefer1m` 避免 Desktop 每个 1M 模型列两行）。
- 上限 200 条（`MAX_INFERENCE_MODELS` L31）。

### 3.4 1+4 槽位如何生成（本地新增，提交 `2d6efba5`）
- **4 个固定 role route id**（【上游已有】常量）：`DEFAULT_PROXY_ROUTES` L66-88 —— `claude-sonnet-5` / `claude-opus-5` / `claude-haiku-4-5`（`LEGACY_HAIKU_ROUTE_ID`）/ `claude-fable-5`；由 `proxy_model_routes` L726-753 在「无显式映射表」时发布。
- **1 个「默认」**（【本地新增】）：每 app 配置的 `default` + `allowed` 引用被发布成额外条目 —— `controller.rs:262-311` `desktop_model_config_publications`：
  ```rust
  // controller.rs:268-280
  if let Some(default) = config.default.as_deref() { references.push(default.to_string()); }
  for reference in &config.allowed { if !references.contains(reference) { references.push(...) } }
  // controller.rs:288-303
  stack::DesktopStackModel {
      id: reference,                       // 引用即模型名
      upstream: entry.model_id.clone(),
      label: format!("{} / {display}", entry.upstream_name),  // 选择器显示「供应商 / 模型」
      provider_name, supports_1m: entry.one_m,
  }
  ```
- 合并顺序 `route_profile` L1260-1277：`proxy_model_routes(route)`（4 个 role id）**在前**，`stack`（= 上面发布的引用）**在后**，超过 200 截断。
- 选择时机：仅当 Stack 名单为空才用模型配置发布（`controller.rs:644-648`）；有 Stack 名单时按名单。
- **槽位（4 个 role key）不进 `inferenceModels`**：它们是固定 route id 的「后端模型」，在请求时映射（见 §3.5）。默认引用则作为独立菜单条目出现。

### 3.5 route id ↔ 真实模型映射（请求时，【本地新增】）
`proxy/handlers.rs:218-240`（`resolve_stack_target`）：
```rust
if matches!(app_type, AppType::ClaudeDesktop) {
    match crate::services::gateway_route::resolve_desktop_role_target(&state.db, model) {
        Ok(Some(target)) => { body["model"] = Value::String(target.upstream_model.clone()); return Ok(Some(target)); }
        Ok(None) => {}          // 槽位没配 → 沿用旧 provider 映射表
        Err(e) => return 400 ...
```
调用链：
- `gateway_route.rs:185-193` `resolve_desktop_role_target` → `desktop_slot_reference` L165-181：
  ```rust
  let Some(role) = crate::claude_desktop_config::role_for_route_id(model) else { return Ok(None) };
  let Some(config) = db.get_app_model_config(AppType::ClaudeDesktop.as_str())? else { return Ok(None) };
  Ok(config.slots.get(role).map(|r| r.trim()).filter(|r| !r.is_empty()).map(str::to_string))
  ```
- `claude_desktop_config.rs:88-96` `role_route_id(role)`（角色名 → route id，由 `DEFAULT_PROXY_ROUTES` 派生）；`role_for_route_id` L99-119（route id → 角色名，兼容旧 id `claude-opus-4-8`、`claude-haiku-5-5`）。
- 拿到引用后走 `resolve_reference_target`（`gateway_route.rs:21-84`），按 `<供应商名>/<模型名>` 选上游并改写 body 的 `model`。
- 若槽位没配：返回 `None` → 旧行为（provider 的 `meta.claudeDesktopModelRoutes` 映射表 / 默认角色路由）。

### 3.6 gateway token / prefix
- token：`claude_desktop_gateway_token`（L28）→ `ccs-<uuid v4 simple>`（L434-445）写进 `inferenceGatewayApiKey`；请求侧 `x-api-key` 或 `Authorization: Bearer`，常量时间比较（`proxy/handlers.rs:587-609`）。
- 前缀：`CLAUDE_DESKTOP_PROXY_PREFIX = "/claude-desktop"`（L27）；`gateway_base_url` L1229-1237 = `proxy_url.trim_end_matches('/') + "/claude-desktop"`。路由入口 `proxy/server.rs:321-325`。

### 3.7 允许列表（allowed）在 Desktop 的作用
- 语义：`default` + `allowed` 决定**发布给 Desktop 的模型菜单条目**（§3.4）；槽位不参与发布，只参与 route id 映射。
- 校验：`gateway_config.rs:166-177`——allowed 非空时 default 必填且必须在 allowed 内。
- 前端自动钉住（提交 `7538d3d7`）：默认 + 槽位永远算 allowed，列表里勾上并禁用，保存时合并写回（`AppConfigPage.tsx:107-119` pinned/effectiveAllowed、L134-141 toggleAllowed、L181 保存 payload `allowed: effectiveAllowed`）。

### 3.8 释放行为
`restore_official_at_paths`（【上游已有】L1332-1379）：
```rust
// claude_desktop_config.rs:1338-1352
let normal = deployment_mode_patch("1p");
let mut threep = deployment_mode_patch("1p");
threep.remove = LEGACY_ENTERPRISE_GATEWAY_KEYS.iter().map(|k| KeyPath::new(&["enterpriseConfig", k])).collect();
let profile = JsonPatch { clear: vec![ClearScope{ parent: root, is_floor: floor::desktop_profile_floor }], .. };
let meta = MetaPatch { applied: false };
```
具体效果：
1. 两个 `claude_desktop_config.json` 的 `deploymentMode` 改回 `"1p"`；3p 里清旧版 `enterpriseConfig` 的 5 个网关键（`LEGACY_ENTERPRISE_GATEWAY_KEYS` L1402-1408），空了就删 `enterpriseConfig`（`DropEmptyEnterpriseConfig` L1411-1430）。
2. **profile 文件里清 6 个 floor 键**（`live/floor.rs:188-196` `DESKTOP_PROFILE_FLOOR`）：`inferenceProvider`、`inferenceGatewayBaseUrl`、`inferenceGatewayApiKey`、`inferenceGatewayAuthScheme`、`inferenceModels`、`modelDiscoveryEnabled`。profile 文件本身**保留**（用户在 Desktop 里改的设置还在），Key 从磁盘清掉。
3. `_meta.json`：删掉 `id == PROFILE_ID` 的条目，`appliedId` 指向剩下的第一条（`MetaPatch` L1454-1505）。
4. 若直连卡可用则写直连 profile（`DesktopTarget::Profile`），否则 `Official`（`controller.rs:760-776`）。

---

## 4. 两边槽位对比（item 4）

| 语义 | Claude Code（env） | Claude Desktop（route id / 槽位） |
|---|---|---|
| 默认 | `ANTHROPIC_MODEL`（引用） | 每 app 配置 `default` → 发布成 `inferenceModels` 的独立条目（**不是** 4 档之一） |
| Sonnet | `ANTHROPIC_DEFAULT_SONNET_MODEL` | route id `claude-sonnet-5` ↔ 槽位 key `sonnet` |
| Opus | `ANTHROPIC_DEFAULT_OPUS_MODEL` | route id `claude-opus-5` ↔ `opus` |
| Haiku | `ANTHROPIC_DEFAULT_HAIKU_MODEL` | route id **`claude-haiku-4-5`** ↔ `haiku`（`role_for_route_id` 同时认旧/新 `claude-haiku-5-5`） |
| Fable | `ANTHROPIC_DEFAULT_FABLE_MODEL` | route id `claude-fable-5` ↔ `fable` |
| 显示名 | 四档 `ANTHROPIC_DEFAULT_*_MODEL_NAME`（上游有）+ 默认 `ANTHROPIC_MODEL_NAME`（本地新增） | `labelOverride`（发布条目）/ `label`（Stack 模式）或 provider 映射表的 label |
| 1M 表达 | 引用后缀 `[1M]`（网关剥掉） | `supports1m: true` + `prefer1m: true`（profile 不接受 `[1M]` 后缀） |
| 槽位回退 | **回落到 default**（`fields()` `.or(self.default)`） | **不回落**：槽位空 → `None` → 旧 provider 映射表（`desktop_slot_reference` 只查 `config.slots`） |
| 存储 key | 无（直接展开到 env） | `gateway_app_profiles.payload.slots`，key = `haiku/sonnet/opus/fable` |
| 槽位顺序 | 写 env 顺序 haiku→sonnet→opus→fable（`Tier::ENV_ORDER`），菜单顺序 opus→fable→sonnet→haiku | 前端 `slotDefinitions` 顺序 fable→opus→sonnet→haiku（`appModelConfig.ts:21-42`）；发布顺序 sonnet→opus→fable→haiku |
| 差异点（需注意） | haiku 档别名 `claude-haiku-5-5`（上游常量） | 发布的 haiku role id 是 `claude-haiku-4-5`（`LEGACY_HAIKU_ROUTE_ID`），两套 id 不同但都映射到同一槽位语义 |

---

## 5. 对上游两个文件的具体改动（item 5）

### 5.1 `src-tauri/src/claude_desktop_config.rs`（+56 行，提交 `2d6efba5`）
全部【本地新增】，无上游逻辑改动：
- `role_route_id(role)` L88-96：角色名 → route id，由 `DEFAULT_PROXY_ROUTES` 派生（与前端 `CLAUDE_DESKTOP_ROLE_ROUTE_IDS` 同源）。
- `role_for_route_id(model)` L99-119：route id → 角色名；认 4 个当前 id + 旧 `claude-opus-4-8` + `claude-haiku-5-5`；非角色 id 返回 `None`（不掺和）。
- 测试 `role_route_ids_round_trip` L1736-1760。
→ 作用：**只服务 Desktop 槽位**（把桌面菜单发来的 route id 翻成配置里的角色名）。

### 5.2 `src-tauri/src/mode/controller.rs`（+409/-21，提交 `fca0bf05`、`051c326c`、`b1e00e53`、`d8b39fec`、`1095d1da`）
新增/改动逐项：
1. import 增加 `proxy_projection_from_model_config`、`ModelConfigSlots`、`ONE_M_MARKER_FOR_CLIENT`（文件头 use，本地改动）。
2. `claude_contract` 加第 6 参 `model_config: Option<&ModelConfigSlots<'_>>`，Stack 名单为空且有模型配置时优先新写法（L187-228）。**属于 CC**。
3. `OwnedModelConfigSlots` + `borrow_slot`（L229-260）——引用/显示名的拥有权桥接，**属于 CC**。
4. `desktop_model_config_publications`（L262-311）——**属于 Desktop 槽位/发布**。
5. `claude_model_config_slots`（L314-347）——**属于 CC 槽位**（含 1M 标记拼接、目录外引用原样写）。
6. `write_proxy` 三处分支：
   - Claude L539-550：`model_config` 传入 `claude_contract`（**CC**）；
   - Codex L587-600：`model: codex_model`（另一人负责，提交 `d8b39fec`/`1095d1da`）；
   - ClaudeDesktop L639-659：`from_stack.is_empty()` 时用 `desktop_model_config_publications`（**Desktop**）。
7. `resync_route*` L1677-1712：已有（`b1e00e53` 用它做「保存模型配置后立刻重写」；调用点 `commands/gateway.rs:332-338`）。
8. 新增 3 个测试：`resync_rewrites_the_client_when_the_model_config_changes`（L2923 起）、`claude_enters_with_model_references_from_app_model_config`（L2984 起）、`claude_desktop_publishes_model_references_from_app_model_config`（L6635 起）。
→ 结论：`+409` 中 **Desktop 专属**的是 `desktop_model_config_publications` + `write_proxy` 的 Desktop 分支 + Desktop 测试；其余是 CC 侧与 Codex 侧。

---

## 6. 前端 UI 层模式变化（item 6，仅概述）

- **删除**整套「直连 / 路由 / 聚合」模式 UI（提交 `f2327180`、`57d47959`）：`ModeDialog.tsx`(-568)、`ModeTabs.tsx`(-193)、`SwitchModePanel.tsx`(-703)、`StackSettingsSheet.tsx`(-165)、`StackSubagentFields.tsx`(-222)、`RouteSettingsSheet.tsx`(-58)、`useDesktopConfigMessages.ts`、`useModeActions.ts`(-120) 及其测试（约 -2800 行）。
- **新增应用页「编辑配置」**：`src/components/gateway/AppConfigPage.tsx` —— 默认模型必填（L303-309）+ 4 槽位可选（L311-338）+ 允许列表（L387-455 附近）+ 顶部「接管/释放」按钮与「已接管/未接管」徽标（L226-245，调用 `gatewayApi.adopt/release`）。
- **新增组件**：`ModelPicker.tsx`(233)、`SlotFields.tsx`、`ClientGatewayBar.tsx`(126，累加式 app 的「指向网关/释放」)、`AppEditorPage.tsx`、`appModelConfig.ts`（槽位定义 L13-48、校验 L83-110）。
- **供应商页**：`src/components/suppliers/SupplierPage.tsx`(HEAD 865 行；并发改动后 1065 行)、`CreateSupplierDialog.tsx`(371)、`SupplierModelsPanel.tsx`(236) + `src/lib/api/suppliers.ts`。
- **导航**：`Sidebar.tsx`(+49/-)、`src/lib/navigation.ts` 调整（应用页/供应商页入口倒置）。
- **API 绑定**：`src/lib/api/gateway.ts`（catalog/appConfig L12-26、累加式六对 adopt/release L73-150、通用 `adopt`/`release` L153-160）。
- 具体 UI/DB 细节由前端+DB 同事负责，此处不深挖。

---

## 7. 边界与存疑（供 Lead 决策）

1. **Desktop 默认引用不是 `inferenceModels` 第一项**：`route_profile` 先 chain `proxy_model_routes`（4 个 role id），再 chain 发布引用（L1260-1277）。注释里「第一项是默认模型」（L1241）描述的是 **Stack 模式**，不是本分支的模型配置路径。
2. **Desktop 槽位不回落默认**：代码事实明确（`gateway_route.rs:175-180` 只查 `config.slots`），与 CC 的 `.or(self.default)`（`claude.rs:316`）不同。若产品预期是"槽位空就用默认"，这里是缺口。
3. **CC 不写 `/model` 菜单**：模型配置接管路径 `top: Map::new()`，不比 Stack 模式写 `modelPicker`；`/model` 里显示的是内置四项 + 各档别名解析到 `ANTHROPIC_DEFAULT_*_MODEL`。模型发现（`/v1/models`）需 `CLAUDE_CODE_ENABLE_GATEWAY_MODEL_DISCOVERY`，本路径**不写**该键（只有 provider 行 exclusive 里有才会落盘）。
4. `ANTHROPIC_MODEL_NAME` 是**本分支新写入的键**：`git show origin/main:src-tauri/src/live/project/claude.rs` 里没有任何 `ANTHROPIC_MODEL_NAME`，只有四档 `ANTHROPIC_DEFAULT_*_MODEL_NAME`（上游 L334-337/L548-563）。仓库里除写入点（claude.rs:312）与断言（claude.rs:745）外无人读取，Claude Code 是否消费该键**未在本仓库验证**，属低风险但需确认。上游路由模式还写 `_MODEL_DESCRIPTION`，本路径不写。
5. 本次未跑测试（按要求），以上均为源码 + 注释 + 现有测试断言阅读结果；CC/Desktop 之外的 app（Codex/Gemini/Grok/6 个累加式 app）只做到与结论相关的交叉引用。
