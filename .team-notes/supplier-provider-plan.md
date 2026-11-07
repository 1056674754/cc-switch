# 供应商（Provider）体系合并施工计划

> 状态：P0–P5 全部完成（2026-11-07）。本文件是这次改造的唯一施工基准与施工记录。
> 已由用户拍板：① UI 术语统一叫「供应商」；② 允许创建后不接入任何 app；③ 按 P0→P5 的顺序做，不为眼前 bug 打乱顺序。

## 1. 概念模型（唯一口径）

| 层 | 名字 | 存在于哪 | 与 app 的关系 |
|---|---|---|---|
| ① 上游服务（服务根 + 协议 + 凭据/账号） | **供应商**（代码里仍叫 supplier / upstream） | 供应商页的卡片；`upstreams` + `upstream_endpoints` + 凭据/账号池 | **无关** |
| ② 接入 | **接入**（binding） | `providers` 表的一行 + `provider_bindings` | 属于某个 app |
| ③ 下游客户端 | **App** | `claude` / `claude-desktop` / `codex` / `kimi` / … | — |

**术语禁令**：UI、注释、文档里不再用 "provider" 指代①（那是 CCR 的用法，且与库表 `providers` 冲突）；
需要写代码时用 `supplier`（①）与 `binding`/`access`（②）。数据库表名 `providers` 实际存的是②，
**本次不改表名**（迁移成本远大于收益），只在文档与注释里标注。

## 2. 现状审计（未合并项摘要）

- 数据层：账号唯一真相在文件池（`codex_oauth_auth.json` 等），供应商只存 `meta.authBinding.accountId` 指针；
  `upstream_credentials` 只存明文，托管账号是合成视图；全局 `default_account_id` 仍参与路由兜底。
- 上游身份：托管接入无 URL 时按 `up-provider-<providerId>` 派生 → 同一家供应商被记成多条上游（Codex 卡 = 16 的来源）。
- 后端：`auth_*`（池）与 `add/delete/setCurrent/setEnabled_supplier_account`（复制/删除接入行）两套；
  `create_supplier` 把「建上游」与「给 app 生成接入」揉死，且只支持 claude / claude-desktop（只会写 Claude env 格式）。
- 前端：老 `AuthCenterPanel` 仍可从应用页供应商表单打开；供应商卡片内嵌老账号组件（显示整池）；
  编辑弹窗另写一套账号 UI；`新增供应商` 仍是老表单（无预设/无导入）。

## 3. 分期计划

### P0 概念与文案对账（本期）
- [x] 写本文件，作为施工基准
- [x] UI 文案：`suppliers.description` 里 "provider" → "供应商"（zh/en）
- [x] 代码注释：`SupplierPage` 顶部概念说明改为「供应商」
- 验收：全站文案一份口径

### P1 后端解耦：创建供应商 ≠ 接入 app（✅ 已完成，2026-11-07）
- 障碍：过去「供应商」是 `providers` 表派生出来的视图，**没有 provider 行的上游在库里不存在**。
  已解决：给 `upstreams` 加 `origin`（`derived` / `manual`），手动建的上游即使零接入也保留；
  `collect()` 本来就是按 endpoint 读的，因此手动上游天然出现在总览里。
- 已完成：
  1. schema v30：`upstreams.origin`（`migrate_v29_to_v30`），孤儿清理跳过 `manual`（凭证/端点/上游三处）。
  2. `Database::create_manual_upstream(name, base_url, protocol, secret, now)`：与派生路径共用
     `up-<host>` / `ep-<hash>` 同一套 id，之后接入 app 时复用同一行上游。
  3. `create_supplier(..., protocol?)`：`apps` 可为空（只建供应商）；`protocol` 默认 anthropic。
  4. 接入仍走 `adopt_supplier`（卡片「采用到…」）：新增 codex（auth + TOML，wire_api 按协议）
     与 gemini（env）的忠实渲染器；opencode / kimi 这类要 models 列表的仍明确拒绝。
  5. 前端：创建弹窗允许不勾 app；「采用到…」按上游协议过滤候选 app（anthropic→claude 系、
     openai_*→Codex、gemini_native→Gemini）。
- 剩余（顺延到 P4/P5）：opencode / kimi 的接入渲染器需要供应商的模型列表，与「模型池」一起做；
  新增向导里选择协议/预设。
- 验收：`create_supplier_without_apps_creates_standalone_supplier`（零接入跨对账存活、再接入不拆家）、
  `adopt_renders_codex_config`、`adopt_renders_gemini_config`、`migrate_v29_to_v30_adds_upstream_origin`；
  前端 `CreateSupplierDialog` / `SupplierPage` 测试全绿。

### P2 账号口径统一（文件池为唯一真相）— 进行中（2026-11-07 第一刀）
设计决定（用户已拍板）：
- 账号池文件（`codex_oauth_auth.json` 等）是**唯一真相**，数据库只存指针，不允许复制账号。
- **切换** = 供应商级：改这家供应商所有接入的 `authBinding.accountId` + 文件池的 `default_account_id`
  一起改，两个 app 同步生效，下游 app 不可见。
- **禁用** = 保留账号、不参与使用（备选池里排除它）；**不删**。
- **退出登录/删除** = 从文件池删除账号（影响所有用它的接入），两个入口（卡片 ⋯ 菜单 / 编辑弹窗垃圾桶）
  统一成同一个动作、同一个图标、同一段二次确认；**不再有「只删接入」的删除**。
- 账号 UI 只有一个组件：卡片与编辑弹窗共用（登录、额度、切换、禁用、退出登录）；
  老 `AuthCenterPanel` 退役；应用页入口保留但复用同一组件。
- 删除「添加账号 = 复制 `sup-acct-*` 接入行」语义：账号不在池里就先去登录，不再克隆接入。
实施顺序：
1. 抽出共享组件 `SupplierAccountsSection`（数据源 = 供应商 credentials ∪ 文件池账号），卡片与弹窗都渲染它。
2. 后端合并命令：`set_supplier_managed_account`（切换）保留；`delete_supplier_account` 改为
   「从池删除 + 解绑所有接入」，与 `auth_remove_account` 合流；`add_supplier_account` 退役。
3. 卡片内嵌的 `SupplierManagedAccounts`（整池视图）删除，`AuthCenterPanel` 从应用页入口摘掉。
4. i18n：删除/退出登录/禁用文案统一；补「影响所有使用该账号的接入」提示。
- 验收：外面 N 个 = 里面 N 个；点切换里外一致；删除只有一种结果；两个入口行为相同。
已完成（第一刀）：
- 编辑弹窗不再自己维护「只列已绑定账号 + 自己加账号」：托管供应商直接渲染卡片用的同一个
  `SupplierManagedAccounts`（登录 / 额度 / ⋯ 全同一套），删掉弹窗里 add/delete/toggle 三套私有逻辑。
- 换账号抽成 `useSupplierAccountSwitch`，卡片与弹窗共用：账号池存在时只切 current、
  不重绑全部接入；切完同步账号池默认账号（里外一起变）。
- 删除口径统一成「退出登录」：账号 ⋯ 菜单、确认框标题/正文/确认键、i18n（zh/en）全部改口，
  并写清「等于退出登录、影响所有使用该账号的接入」。
- 弹窗里「添加已登录账号」下拉删除（账号从登录来，不再从已有账号里"加入"）。
- 测试：`SupplierEditDialog` / `SupplierPage` / `CodexOAuthSection` / `ClaudeOfficialSection` /
  `AuthCenterGroups` / 全量 3638 条全绿。

第二刀（已完成）：
- **禁用/启用** 补回来了，并且是共享的：`ManagedAccountsGroup` 的账号行 ⋯ 里新增一项，
  仅当调用方传 `onToggleEnabled` + `enabled` 时出现 → 授权中心不受影响，供应商卡片与弹窗
  两处同时获得该动作（`SupplierManagedAccounts` 把 `accountEnabled` / `onToggleAccountEnabled`
  透传给 Codex / Copilot / xAI / agy / Claude 官方五个区块）。
- 供应商页统一用 `useSupplierAccountSwitch` 暴露的 `toggleEnabled` 调
  `set_supplier_account_enabled`，成功后刷新总览；账号「启用/停用」= 是否参与使用
  （后端语义：该账号的接入是否进故障转移队列；停用当前账号会自动切到别的账号）。
- 状态来源仍是供应商总览里的 `credential.apps[].inFailoverQueue/isCurrent`：
  正在使用的账号恒为「启用」；池里没有的账号不显示停用/启用。
- 测试：`CodexOAuthSection` 新增「不传回调就没有停用项 / 传了就按状态显示停用或启用」，
  `SupplierEditDialog` 新增「透传可用性与停用回调」；全量 3640 条全绿。

剩余：
- 真正的自绘账号行组件（替换 embed 的授权中心组件）才算 UI 完全统一；当前是复用其行组件 +
  外层区块。`AuthCenterPanel` 仍在应用侧表单里（应用页入口保留），彻底退役放 P5。
- 若以后要「禁用 = 纯供应商侧标记、不碰队列」，需要迁移存储（`upstream_credentials.enabled`），
  现在沿用的仍是队列语义，界面上已写清「切换时不会再选它」。
- 验收：外面 N 个 = 里面 N 个；点切换里外一致；删除只有一种结果；两个入口行为相同。

### P3 上游身份归并（修 16）— 已完成 2026-11-07
- 托管供应商的上游身份 = `authProvider`（官方 Codex 一家）；模型/凭据/发现/覆盖只存一份。
- 中转站仍按 baseURL 各算一家；`up-provider-*` 的历史行做一次性归并迁移。
- 验收：Codex 卡片 = 上游真实模型数（10），左 6 / 右 4，不再出现 16。

实现：
- `schema.rs::p1_derive` 身份优先级改为 **托管服务身份 > host > provider id**：
  托管行统一派生 `up-managed-<authProvider>`；身份来源：`meta.authBinding.authProvider` →
  Claude Desktop `claudeDesktopMode=proxy` + chatgpt.com → `*-official` 占位行 id。
- `p1_canonical_names` 给托管身份选显示名（优先主 app 的官方占位行：
  codex→OpenAI Official、claude→Claude Official、gemini→Google Official）。
- `migrate_v30_to_v31`：把老上游（`up-chatgpt.com`、`up-provider-codex-official` …）的端点 /
  凭证 / 绑定 / `gateway_upstream_models` / `gateway_model_overrides` 搬到新身份后删老行；
  端点 id 里含上游 id，迁完重算并改引用；可重复执行。`SCHEMA_VERSION = 31`。
- `collect()`：供应商同时按上游 id 前缀认托管身份（`up-managed-*`），否则 Codex 代理那条
  端点会掉进 key 型分组、卡片上看不到它在被使用。
- 测试：`p1_derive_merges_managed_rows_into_one_upstream`、
  `migrate_v30_to_v31_merges_managed_upstreams`、
  `managed_codex_subscription_merges_into_one_supplier`。
- 真库验证（v31 迁移后）：`up-managed-codex_oauth` 一家上游、2 个端点（anthropic 代理 +
  openai_responses 官方）、目录 10 个模型（迁移前 up-chatgpt.com 10 + up-provider-codex-official 10
  → 页面 16）。

### P4 新增向导（对齐 CCR）— 已完成 2026-11-07
- `新增供应商` → 下拉菜单：导入本机 Agent（Codex ChatGPT 登录态、Kimi CLI OAuth…，检测到才出现）/
  预设供应商（可搜索，复用 `src/config/*Presets.ts`）/ 其他·自定义地址。
- 选中后打开 = 编辑弹窗同一套表单；创建专属多一块「接入应用」（可空）。
- 验收：新增与编辑同构，没有第二套表单。

实现：
- `CreateSupplierDialog` 改成两步向导：先选来源（预设 / 导入本机登录态 / 自定义地址），
  再进表单；表单字段与编辑弹窗一致（名称、服务根、上游协议、密钥、接入应用），
  地址 / 协议可继续改。返回按钮回来源选择，提交中整页禁用。
- 预设列表 `supplierPresets.ts`：从 `claude/codex/gemini` 三份应用预设里抽「名字 + 服务根 +
  上游协议」，按「协议 + 地址」去重、按名排序；支持按名称 / 地址 / 协议搜索。不引预设里
  app 专属的 env、模型槽位和 OAuth 流程。
- 导入本机 Agent：只列有账号的官方订阅（`codex_oauth` / `claude_official`），点「导入」调用新命令
  `ensure_managed_supplier`。它落 `up-managed-<authProvider>` 这家上游 + 端点（协议由服务决定），
  不写明文密钥、不接 app；幂等，重复导入仍是同一张卡。Kimi 的登录态属于 P5 的账号区，不在此列。
- 接入应用默认不勾（创建与接入解耦）；只有 anthropic 协议显示 claude 系勾选，其它协议提示
  创建后在卡片上按协议接入——后端也加了同样的拦截，避免把 openai/gemini 协议写成 anthropic 配置。
- 测试：`CreateSupplierDialog.test.tsx` 18 个（来源选择 / 预设搜索回填 / 导入幂等入口 /
  非 anthropic 隐藏应用 / 协议随创建提交）、`supplierPresets.test.ts` 4 个、
  后端 `ensure_managed_supplier_is_idempotent`、`create_rejects_apps_for_non_anthropic_protocol`。

### P5 收尾 — 已完成 2026-11-07
- Kimi / DSH 纳入同一套账号区；清理 i18n 里"授权中心"残留；补测试与回归。
- 验收：`pnpm test` / `tsc` / `cargo test` 绿；构建安装重启。

实现：
- **DSH 进供应商页账号区**：新增 `SupplierDshAccountCard`（`DeepSeek Harness` 卡，只有账号区，没有
  地址/协议/启停/删除）；DSH 账号登录了才出现（`dsh_account_status.signedIn`），登出即消失。
  卡片内嵌 `DshAccountLogin embedded headerLabel="凭据"`，和 Kimi 卡、托管账号卡的账号区同一套
  行组件（`ManagedAccountsGroup`），登录/切换/停用/退出登录语义一致。
- **应用页「管理账号」不再打开授权中心**：`AuthSettingsPanel` 改为按 target 渲染共享账号区——
  托管服务 → `SupplierManagedAccounts`，`kimi_oauth` → `KimiAccountLogin`，`dsh_account` →
  `DshAccountLogin`；面板标题用服务名（OpenAI Official / Kimi Code / DeepSeek Harness），
  不再出现第二套「授权中心」页面。`AuthCenterPanel` 已标退役（文件暂留，不再被入口引用）。
- **i18n 清理**：Kimi / DSH / agy / 额度 / 订阅过期的提示文案从「去授权中心」改成
  「在供应商卡片的凭据区」；`nav.auth` 从「授权中心」改为「账号」；删掉 zh-TW 里过期的
  「到授权中心登录」文案。代理/中转相关文案不动。
- 测试：`SupplierPage.test.tsx` 新增「DSH 登录时出现在同一账号区 / 登出时消失且没有供应商动作」；
  新增 `AuthSettingsPanel.test.tsx`（4 个：托管共享账号区 / Kimi / DSH / 关闭态）；
  `SubscriptionQuotaFooter` 断言跟着新文案更新。全量 294 文件 / 3655 条绿，`tsc` 干净。

### 后续修正：模型映射只在接入端配（2026-11-07）
- 用户验收时指出：供应商卡上的「模型映射」按钮/下拉是错的位置——模型映射是「某个 app 的这条接入」
  自己的配置，应该在 app 侧（接入端）配，不该在供应商侧配。
- 已改：`SupplierPage` 删掉卡片上的 `模型映射` 下拉、`ModelMapEditor` 内嵌编辑器及
  `openModelMap/saveModelMap` 一套状态；`suppliers.modelMapAction` i18n 五语言密钥删除。
  接入端页（`BindingsPage`）本来就按 app + 接入展示 `bindings.modelMap` 并提供编辑，是唯一入口。
- 测试：`SupplierPage.test.tsx` 改为断言供应商卡上不再有模型映射入口；全量 294 文件 / 3655 条绿。

## 4. 约束与风险

- 工作区与另一个会话共享：`SupplierPage.tsx`、`SupplierEditDialog.tsx`、`commands/suppliers.rs`、
  `stream_check.rs` 等可能同时在改；每次编辑前重读，改动限定在本次范围内，不回收他人修改。
- 迁移必须可回退、不丢数据：先加列/加表，再改读取路径，最后清理；每期构建并重启验证。
- 不允许把账号池（文件）并进数据库；方向是让供应商去用池，不是反过来。
