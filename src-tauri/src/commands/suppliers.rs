//! 供应商（网关级上游）总览。
//!
//! 这是「以供应商为一级公民」的读入口：前端不再从「某个 app 的 provider 列表」
//! 出发，而是从网关出发看——一家供应商对应**一个确切的上游 API**，被哪些 app 使用。
//!
//! ## 上游协议与下游协议
//!
//! 一条请求穿过代理时有两段协议，不能混为一谈：
//!
//! - **下游协议**：客户端 → 代理。由 **app** 决定（Claude Code 说 Anthropic Messages，
//!   Codex 说 OpenAI Responses），是 app 的固有属性。
//! - **上游协议**：代理 → 供应商。由 **`provider.meta.apiFormat`** 决定
//!   （`openai_chat` / `anthropic` / `openai_responses` / `gemini_native`），是供应商的属性。
//!
//! 代理就是两段之间的转换器；两者不同正是路由模式存在的理由。**供应商的身份里只能有
//! 上游协议**——按 app 推协议会把同一个上游拆成好几家（曾经就这么错过了：Claude 用户
//! 接的 CTYun 上游其实是 `openai_chat`，却被标成 `anthropic`）。
//!
//! ## 为什么没有「端点」这一层
//!
//! 同一个上游 API 在不同 app 的配置里会写成不同的 URL 形式（OpenAI 客户端写 `.../v1`，
//! 开了「完整 URL」的客户端写 `.../v1/chat/completions`）。这些是**同一条路由的不同写法**，
//! 不是多个端点。这里把它们规范化到服务根再归并，界面上一个供应商只有**一个地址**。

use crate::app_config::AppType;
use crate::database::lock_conn;
use crate::error::AppError;
use crate::store::AppState;
use serde::Serialize;
use std::collections::BTreeMap;
use std::str::FromStr;

/// 一条接入：哪个 app、哪个 provider、当前模型映射、下游协议。
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BindingDetail {
    pub app_type: String,
    pub provider_id: String,
    /// 槽位化 JSON（haiku/sonnet/opus/fable/subagent/default），空对象表示没配
    pub model_map: serde_json::Value,
    /// 客户端 → 代理这一段用的协议（app 决定）
    pub downstream_protocol: String,
}

/// 网关里的一家供应商 = **一个确切的上游 API**（服务根 + 上游协议）+ 用它的一组接入。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Supplier {
    /// `sup-{服务根+上游协议的哈希}`；托管登录账号（没有网关地址）用上游 id 兜底
    pub id: String,
    pub name: String,
    pub host: String,
    /// 规范化后的服务根（去掉 `/chat/completions`、`/responses`、`/messages` 这类完整接口后缀）；
    /// 托管登录账号为空
    pub base_url: String,
    /// 代理 → 上游这一段用的协议
    pub upstream_protocol: String,
    /// 使用它的 app（去重、有序）
    pub apps: Vec<String>,
    /// `app:providerId` 明细，方便前端跳回原配置
    pub consumers: Vec<String>,
    /// 接入明细（含模型映射与下游协议），前端编辑映射时用来预填
    pub bindings: Vec<BindingDetail>,
    /// 挂在这个地址上的凭据数
    pub credential_count: i64,
    /// 这个地址上出现过的凭据形态：literal / env_ref / managed / none
    pub credential_kinds: Vec<String>,
    /// 托管登录的认证供应商（`codex_oauth` / `claude_official` / …）。有值说明
    /// 这家供应商的凭据来自授权中心的账号池，卡片要列账号而不是只报「1 个凭据」。
    pub managed_auth_provider: Option<String>,
    /// 绑定到这家供应商的 provider 分别选了账号池里的哪个账号
    /// （`provider.meta.authBinding.accountId`），用于在账号行上标「使用中」。
    pub bound_account_ids: Vec<String>,
    /// 归到这家供应商的上游 id（目录与模型覆盖按它对齐，一个服务根可能对应多条上游）
    pub upstream_ids: Vec<String>,
    /// 启停开关（供应商页的「启用/停用」）：停用 = 网关选路时跳过这家上游，
    /// 配置、凭证、模型映射全部保留，重新启用即恢复。
    pub enabled: bool,
}

/// 把 `apiFormat` 的常见写法归一。
fn normalize_api_format(raw: &str) -> String {
    match raw.trim().to_ascii_lowercase().as_str() {
        "openai_chat" | "openai-chat" | "chat" | "chat_completions" | "openai_chat_completions" => {
            "openai_chat".to_string()
        }
        "openai_responses" | "responses" => "openai_responses".to_string(),
        "anthropic" | "anthropic_messages" | "messages" => "anthropic".to_string(),
        "gemini_native" | "gemini" | "gemini_generate_content" => "gemini_native".to_string(),
        "kimi" | "kimi_managed" => "kimi_managed".to_string(),
        "alibaba" => "alibaba".to_string(),
        other => other.to_string(),
    }
}

/// 上游协议：`meta.apiFormat` → `settings_config.api_format/apiFormat` → codex TOML 的
/// `wire_api`。都没有时返回 `None`，由调用方回落到端点自身的协议列（那一列是 app 推出来的，
/// 只在直连语义下等于上游协议）。
fn upstream_protocol_from(meta: &str, config: &serde_json::Value) -> Option<String> {
    if let Ok(meta) = serde_json::from_str::<serde_json::Value>(meta) {
        for key in ["apiFormat", "api_format"] {
            if let Some(raw) = meta.get(key).and_then(|value| value.as_str()) {
                if !raw.trim().is_empty() {
                    return Some(normalize_api_format(raw));
                }
            }
        }
    }
    for key in ["api_format", "apiFormat"] {
        if let Some(raw) = config.get(key).and_then(|value| value.as_str()) {
            if !raw.trim().is_empty() {
                return Some(normalize_api_format(raw));
            }
        }
    }
    // codex 的 `config` 是一段 TOML，wire_api 在里面
    let toml = config.get("config").and_then(|value| value.as_str())?;
    for line in toml.lines() {
        let line = line.trim();
        let Some(rest) = line.strip_prefix("wire_api") else {
            continue;
        };
        let Some(rest) = rest.trim_start().strip_prefix('=') else {
            continue;
        };
        let value = rest.trim().trim_matches('"').trim_matches('\'');
        if !value.is_empty() {
            return Some(normalize_api_format(value));
        }
    }
    None
}

/// 服务根：去掉「完整接口后缀」，让 `.../v1` 和 `.../v1/chat/completions` 归到同一家。
fn canonical_base_url(base_url: &str) -> String {
    let trimmed = base_url.trim();
    let mut url = trimmed.trim_end_matches('/').to_string();
    for suffix in ["/chat/completions", "/responses", "/messages"] {
        if url.to_ascii_lowercase().ends_with(suffix) {
            let keep = url.len() - suffix.len();
            url.truncate(keep);
            break;
        }
    }
    url.trim_end_matches('/').to_string()
}

/// 一家供应商的稳定 id：服务根 + 上游协议。托管账号没有地址，用上游 id 兜底。
fn supplier_id_for(base_url: &str, upstream_protocol: &str, upstream_id: &str) -> String {
    let key = if base_url.trim().is_empty() {
        format!("upstream|{upstream_id}")
    } else {
        format!("url|{}|{upstream_protocol}", canonical_base_url(base_url))
    };
    format!("sup-{}", crate::database::Database::p1_stable_hash(&key))
}

/// endpoint_id → 该端点的上游协议。
///
/// 端点上可能挂着多个 app 的接入，它们的 `apiFormat` 正常应该一致；不一致时按出现次数
/// 取最多的那个（数据有冲突时至少不会把同一家拆开），并用 `mixed` 标出来。
fn endpoint_upstream_protocols(
    conn: &rusqlite::Connection,
) -> Result<(BTreeMap<String, String>, BTreeMap<String, bool>), AppError> {
    let mut providers: BTreeMap<(String, String), (String, serde_json::Value)> = BTreeMap::new();
    {
        let mut stmt = conn.prepare(
            "SELECT app_type, id, COALESCE(meta, ''), COALESCE(settings_config, '{}') FROM providers",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?;
        for row in rows {
            let (app_type, provider_id, meta, config) = row?;
            let config = serde_json::from_str(&config).unwrap_or(serde_json::Value::Null);
            providers.insert((app_type, provider_id), (meta, config));
        }
    }

    let mut votes: BTreeMap<String, BTreeMap<String, i64>> = BTreeMap::new();
    {
        let mut stmt =
            conn.prepare("SELECT endpoint_id, app_type, provider_id FROM provider_bindings")?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?;
        for row in rows {
            let (endpoint_id, app_type, provider_id) = row?;
            let Some((meta, config)) = providers.get(&(app_type, provider_id)) else {
                continue;
            };
            if let Some(protocol) = upstream_protocol_from(meta, config) {
                *votes
                    .entry(endpoint_id)
                    .or_default()
                    .entry(protocol)
                    .or_insert(0) += 1;
            }
        }
    }

    // 没有 apiFormat 的端点回落到端点自身的协议列（直连语义下它等于上游协议）
    {
        let mut stmt =
            conn.prepare("SELECT id, COALESCE(protocol, 'unknown') FROM upstream_endpoints")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        for row in rows {
            let (endpoint_id, protocol) = row?;
            let counts = votes.entry(endpoint_id).or_default();
            if counts.is_empty() {
                *counts.entry(normalize_api_format(&protocol)).or_insert(0) += 1;
            }
        }
    }

    let mut protocols = BTreeMap::new();
    let mut mixed = BTreeMap::new();
    for (endpoint_id, counts) in votes {
        if counts.is_empty() {
            protocols.insert(endpoint_id, "unknown".to_string());
            continue;
        }
        let total: i64 = counts.values().sum();
        let (winner, votes_for_winner) = counts
            .iter()
            .max_by_key(|(_, count)| **count)
            .map(|(protocol, count)| (protocol.clone(), *count))
            .unwrap_or_else(|| ("unknown".to_string(), 0));
        if total > votes_for_winner {
            mixed.insert(endpoint_id.clone(), true);
        }
        protocols.insert(endpoint_id, winner);
    }
    Ok((protocols, mixed))
}

/// 从 provider 的 meta 认托管登录：优先读新的 `authBinding`，老官方卡再按 id 兜底。
///
/// 返回（认证供应商, 绑定账号）。`authBinding.accountId` 为空表示跟随该服务的默认账号。
fn managed_auth_binding(meta: &str, provider_id: &str) -> (Option<String>, Option<String>) {
    if let Ok(meta) = serde_json::from_str::<serde_json::Value>(meta) {
        let provider = meta
            .pointer("/authBinding/authProvider")
            .and_then(|value| value.as_str())
            .map(str::to_string);
        let account = meta
            .pointer("/authBinding/accountId")
            .and_then(|value| value.as_str())
            .map(str::to_string);
        if provider.is_some() || account.is_some() {
            return (provider, account);
        }
    }
    let fallback = match provider_id {
        "codex-official" => Some("codex_oauth"),
        "claude-official" | "claude-desktop-official" => Some("claude_official"),
        _ => None,
    };
    (fallback.map(str::to_string), None)
}

fn collect(db: &crate::database::Database) -> Result<Vec<Supplier>, AppError> {
    let mut conn = lock_conn!(db.conn);

    // 读之前先对账：绑定是由路由器的读路径派生出来的，从未走过代理的 app
    // 在库里没有绑定，直接查会让总览漏掉它们。
    let apps: Vec<String> = {
        let mut stmt = conn.prepare("SELECT DISTINCT app_type FROM providers")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        rows.collect::<Result<Vec<_>, _>>()?
    };
    for app in apps {
        if let Err(error) = crate::database::Database::reconcile_provider_bindings(&mut conn, &app)
        {
            log::warn!("[suppliers] {app} 上游绑定对账失败: {error}");
        }
    }

    let (upstream_protocols, _mixed) = endpoint_upstream_protocols(&conn)?;

    // (app_type, provider_id) → provider.meta：认托管登录的认证供应商和绑定的账号。
    // provider 行才是账号归属的真相，端点和供应商都是从它派生出来的。
    let provider_metas: BTreeMap<(String, String), String> = {
        let mut stmt = conn.prepare("SELECT app_type, id, COALESCE(meta, '') FROM providers")?;
        let rows = stmt.query_map([], |row| {
            Ok((
                (row.get::<_, String>(0)?, row.get::<_, String>(1)?),
                row.get::<_, String>(2)?,
            ))
        })?;
        rows.collect::<Result<_, _>>()?
    };

    // endpoint_id → 该端点上的一条条接入
    let mut consumers: BTreeMap<String, Vec<(String, String, String)>> = BTreeMap::new();
    {
        let mut stmt = conn.prepare(
            "SELECT endpoint_id, app_type, provider_id, COALESCE(model_map, '')
             FROM provider_bindings ORDER BY endpoint_id, app_type, provider_id",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?;
        for row in rows {
            let (endpoint_id, app_type, provider_id, model_map) = row?;
            consumers
                .entry(endpoint_id)
                .or_default()
                .push((app_type, provider_id, model_map));
        }
    }

    // endpoint_id → (凭据数, 凭据形态)
    let mut per_endpoint: BTreeMap<String, (i64, Vec<String>)> = BTreeMap::new();
    {
        let mut stmt = conn.prepare(
            "SELECT endpoint_id, COALESCE(secret_kind, 'none') FROM upstream_credentials",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        for row in rows {
            let (endpoint_id, kind) = row?;
            let entry = per_endpoint.entry(endpoint_id).or_insert((0, Vec::new()));
            entry.0 += 1;
            if !entry.1.contains(&kind) {
                entry.1.push(kind);
            }
        }
    }

    let mut suppliers: BTreeMap<String, Supplier> = BTreeMap::new();
    {
        let mut stmt = conn.prepare(
            "SELECT e.id, e.upstream_id, COALESCE(e.protocol, 'unknown'), e.base_url, u.name, u.host,
                    COALESCE(e.enabled, 1)
             FROM upstream_endpoints e JOIN upstreams u ON u.id = e.upstream_id
             ORDER BY u.name COLLATE NOCASE, e.base_url, e.protocol",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, i64>(6)?,
            ))
        })?;
        for row in rows {
            let (endpoint_id, upstream_id, fallback_protocol, base_url, name, host, enabled) = row?;
            let protocol = upstream_protocols
                .get(&endpoint_id)
                .cloned()
                .unwrap_or_else(|| normalize_api_format(&fallback_protocol));
            let id = supplier_id_for(&base_url, &protocol, &upstream_id);
            let supplier = suppliers.entry(id.clone()).or_insert_with(|| Supplier {
                id,
                name,
                host,
                base_url: canonical_base_url(&base_url),
                upstream_protocol: protocol.clone(),
                apps: Vec::new(),
                consumers: Vec::new(),
                bindings: Vec::new(),
                credential_count: 0,
                credential_kinds: Vec::new(),
                managed_auth_provider: None,
                bound_account_ids: Vec::new(),
                upstream_ids: Vec::new(),
                enabled: enabled != 0,
            });

            // 一个供应商理论上只对应一条上游；真合并了多条时以「全部启用」为启用
            if enabled == 0 {
                supplier.enabled = false;
            }

            if !supplier.upstream_ids.contains(&upstream_id) {
                supplier.upstream_ids.push(upstream_id.clone());
            }

            let (credential_count, kinds) = per_endpoint
                .get(&endpoint_id)
                .cloned()
                .unwrap_or((0, Vec::new()));
            supplier.credential_count += credential_count;
            for kind in kinds {
                if !supplier.credential_kinds.contains(&kind) {
                    supplier.credential_kinds.push(kind);
                }
            }

            for (app_type, provider_id, model_map) in
                consumers.get(&endpoint_id).into_iter().flatten()
            {
                if !supplier.apps.contains(app_type) {
                    supplier.apps.push(app_type.clone());
                }
                let consumer = format!("{app_type}:{provider_id}");
                if !supplier.consumers.contains(&consumer) {
                    supplier.consumers.push(consumer);
                }
                if let Some(meta) = provider_metas.get(&(app_type.clone(), provider_id.clone())) {
                    let (auth_provider, account_id) = managed_auth_binding(meta, provider_id);
                    if let Some(auth_provider) = auth_provider {
                        supplier.managed_auth_provider.get_or_insert(auth_provider);
                    }
                    if let Some(account_id) = account_id {
                        if !supplier.bound_account_ids.contains(&account_id) {
                            supplier.bound_account_ids.push(account_id);
                        }
                    }
                }
                if supplier.bindings.iter().any(|binding| {
                    binding.app_type == *app_type && binding.provider_id == *provider_id
                }) {
                    continue;
                }
                supplier.bindings.push(BindingDetail {
                    app_type: app_type.clone(),
                    provider_id: provider_id.clone(),
                    model_map: serde_json::from_str(model_map)
                        .unwrap_or_else(|_| serde_json::json!({})),
                    downstream_protocol: downstream_protocol_of(app_type),
                });
            }
        }
    }

    let mut list: Vec<Supplier> = suppliers.into_values().collect();
    list.sort_by(|a, b| {
        a.name
            .to_ascii_lowercase()
            .cmp(&b.name.to_ascii_lowercase())
            .then_with(|| a.upstream_protocol.cmp(&b.upstream_protocol))
            .then_with(|| a.base_url.cmp(&b.base_url))
    });
    Ok(list)
}

/// 客户端 → 代理这一段的协议：由 app 决定，跟供应商无关。
fn downstream_protocol_of(app_type: &str) -> String {
    match app_type {
        "claude" | "claude-desktop" => "anthropic".to_string(),
        "codex" | "grokbuild" => "openai_responses".to_string(),
        "gemini" => "gemini_native".to_string(),
        "kimi" => "kimi_managed".to_string(),
        "opencode" | "openclaw" | "hermes" | "omp" | "pi" | "mcode" | "dsh" | "agy" => {
            "openai_chat".to_string()
        }
        other => other.to_string(),
    }
}

/// 这个供应商下所有底层端点 id（改地址 / 换密钥要逐条写）。
fn endpoint_ids_of(conn: &rusqlite::Connection, supplier_id: &str) -> Result<Vec<String>, String> {
    let (protocols, _) = endpoint_upstream_protocols(conn).map_err(|error| error.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, upstream_id, COALESCE(protocol, 'unknown'), base_url FROM upstream_endpoints")
        .map_err(|error| error.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })
        .map_err(|error| error.to_string())?;
    let mut ids = Vec::new();
    for row in rows {
        let (endpoint_id, upstream_id, fallback_protocol, base_url) =
            row.map_err(|error| error.to_string())?;
        let protocol = protocols
            .get(&endpoint_id)
            .cloned()
            .unwrap_or_else(|| normalize_api_format(&fallback_protocol));
        if supplier_id_for(&base_url, &protocol, &upstream_id) == supplier_id {
            ids.push(endpoint_id);
        }
    }
    Ok(ids)
}

/// 供应商总览：网关级视角的只读聚合。
#[tauri::command]
pub async fn get_suppliers_overview(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<Supplier>, String> {
    collect(&state.db).map_err(|error| error.to_string())
}

/// 改一家供应商的地址：所有使用它的 app 一起跟着变。
///
/// 一个供应商只有一个地址，所以这里改的是这家供应商底下**所有**底层端点（它们只是
/// 同一个 API 在不同 app 配置里的写法差异）。
#[tauri::command]
pub async fn update_supplier_base_url(
    state: tauri::State<'_, AppState>,
    supplier_id: String,
    base_url: String,
) -> Result<Vec<Supplier>, String> {
    let normalized = crate::database::Database::p1_normalize_url(&base_url)
        .map(|(_, url)| url)
        .ok_or_else(|| "base_url 格式无效（需要完整的 http(s) 地址）".to_string())?;

    {
        let mut conn = lock_conn!(state.db.conn);
        let endpoint_ids = endpoint_ids_of(&conn, &supplier_id)?;
        if endpoint_ids.is_empty() {
            return Err("找不到这家供应商（可能已被删除）".to_string());
        }

        let mut changed = 0usize;
        let mut skipped: Vec<String> = Vec::new();
        let mut touched_apps: Vec<String> = Vec::new();
        for endpoint_id in &endpoint_ids {
            let outcome = crate::database::Database::rewrite_endpoint_base_url(
                &mut conn,
                endpoint_id,
                &normalized,
            )
            .map_err(|error| error.to_string())?;
            changed += outcome.changed;
            skipped.extend(outcome.skipped);
            for app in outcome.apps {
                if !touched_apps.contains(&app) {
                    touched_apps.push(app);
                }
            }
        }

        if changed == 0 {
            return Err(if skipped.is_empty() {
                "这家供应商没有任何接入".to_string()
            } else {
                format!(
                    "这些接入的地址不在配置里，无法自动改写：{}",
                    skipped.join("、")
                )
            });
        }
        if !skipped.is_empty() {
            log::warn!(
                "[suppliers] 供应商 {supplier_id} 改址时跳过 {} 条接入：{}",
                skipped.len(),
                skipped.join("、")
            );
        }
        for app in &touched_apps {
            crate::database::Database::reconcile_provider_bindings(&mut conn, app)
                .map_err(|error| error.to_string())?;
        }
    }

    collect(&state.db).map_err(|error| error.to_string())
}

/// 换一把密钥：所有使用这家供应商的 app 一起换。
#[tauri::command]
pub async fn update_supplier_secret(
    state: tauri::State<'_, AppState>,
    supplier_id: String,
    secret: String,
) -> Result<Vec<Supplier>, String> {
    let secret = secret.trim().to_string();
    if secret.is_empty() {
        return Err("密钥不能为空".to_string());
    }

    {
        let mut conn = lock_conn!(state.db.conn);
        let endpoint_ids = endpoint_ids_of(&conn, &supplier_id)?;
        if endpoint_ids.is_empty() {
            return Err("找不到这家供应商（可能已被删除）".to_string());
        }

        let mut changed = 0usize;
        let mut skipped: Vec<String> = Vec::new();
        let mut touched_apps: Vec<String> = Vec::new();
        for endpoint_id in &endpoint_ids {
            let outcome =
                crate::database::Database::rewrite_endpoint_secret(&mut conn, endpoint_id, &secret)
                    .map_err(|error| error.to_string())?;
            changed += outcome.changed;
            skipped.extend(outcome.skipped);
            for app in outcome.apps {
                if !touched_apps.contains(&app) {
                    touched_apps.push(app);
                }
            }
        }

        if changed == 0 {
            return Err(if skipped.is_empty() {
                "这家供应商没有任何接入".to_string()
            } else {
                format!(
                    "这些接入的密钥存在环境变量里，库里没有明文可换：{}",
                    skipped.join("、")
                )
            });
        }
        if !skipped.is_empty() {
            log::warn!(
                "[suppliers] 供应商 {supplier_id} 换密钥时跳过 {} 条接入：{}",
                skipped.len(),
                skipped.join("、")
            );
        }
        for app in &touched_apps {
            crate::database::Database::reconcile_provider_bindings(&mut conn, app)
                .map_err(|error| error.to_string())?;
        }
    }

    collect(&state.db).map_err(|error| error.to_string())
}

/// 供应商启停开关：停用后网关选路跳过这家上游，所有 app 的配置原样保留。
///
/// 开关落在 `upstream_endpoints.enabled`（一家供应商一个开关，端点行按「服务根 +
/// 协议」区分），不是每个 app 各一个：供应商页的语义就是「这家上游还能不能用」，
/// 与它被几个 app 接进来无关。
#[tauri::command]
pub async fn set_supplier_enabled(
    state: tauri::State<'_, AppState>,
    supplier_id: String,
    enabled: bool,
) -> Result<Vec<Supplier>, String> {
    set_supplier_enabled_in_db(&state.db, &supplier_id, enabled)
}

fn set_supplier_enabled_in_db(
    db: &crate::database::Database,
    supplier_id: &str,
    enabled: bool,
) -> Result<Vec<Supplier>, String> {
    let supplier = collect(db)
        .map_err(|error| error.to_string())?
        .into_iter()
        .find(|supplier| supplier.id == supplier_id)
        .ok_or_else(|| "找不到这家供应商（可能已被删除）".to_string())?;

    // 托管登录账号没有网关地址，不参与选路：开关对它没有意义
    if supplier.base_url.trim().is_empty() {
        return Err("托管登录账号没有网关地址，不能启用/停用".to_string());
    }

    {
        let mut conn = lock_conn!(db.conn);
        let endpoint_ids = endpoint_ids_of(&conn, supplier_id)?;
        let tx = conn
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|error| error.to_string())?;
        // 一家供应商可能对应多条端点行（完整 URL / 服务根的不同写法），一起改
        for endpoint_id in &endpoint_ids {
            tx.execute(
                "UPDATE upstream_endpoints SET enabled = ?1 WHERE id = ?2",
                rusqlite::params![if enabled { 1i64 } else { 0i64 }, endpoint_id],
            )
            .map_err(|error| error.to_string())?;
        }
        tx.commit().map_err(|error| error.to_string())?;
    }

    collect(db).map_err(|error| error.to_string())
}

/// 删除一家供应商：把它在每个接入的 app 里的那几条 provider 一起删掉。
///
/// 供应商不是库里独立的一行——它是从各 app 的 `providers` 对账派生出来的。所以「删除
/// 供应商」必须真的删底层 provider，否则下一次读又被对账重建回来。删除走各 app 既有的
/// `ProviderService::delete`（客户端配置、快照、只读限制都由它处理）；删不掉的（官方
/// 只读、托管账号等）不阻断其它 app，原样回报给界面。
#[tauri::command]
pub async fn delete_supplier(
    state: tauri::State<'_, AppState>,
    supplier_id: String,
) -> Result<DeleteSupplierOutcome, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || delete_supplier_consumers(&state, &supplier_id))
        .await
        .map_err(|error| format!("删除供应商任务失败: {error}"))?
}

/// 删除时被跳过的接入（原因原样透出给界面）。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteSupplierSkip {
    pub app_type: String,
    pub provider_id: String,
    pub reason: String,
}

/// 删除结果：删除后的总览 + 删掉的接入数 + 删不掉的明细。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteSupplierOutcome {
    pub suppliers: Vec<Supplier>,
    pub deleted: usize,
    pub skipped: Vec<DeleteSupplierSkip>,
}

fn delete_supplier_consumers(
    state: &AppState,
    supplier_id: &str,
) -> Result<DeleteSupplierOutcome, String> {
    let supplier = collect(&state.db)
        .map_err(|error| error.to_string())?
        .into_iter()
        .find(|supplier| supplier.id == supplier_id)
        .ok_or_else(|| "找不到这家供应商（可能已被删除）".to_string())?;

    if supplier.base_url.trim().is_empty() {
        return Err("托管登录账号没有网关地址，请在「账号」页删除".to_string());
    }

    let mut deleted = 0usize;
    let mut skipped = Vec::new();
    let mut touched_apps: Vec<String> = Vec::new();
    for binding in &supplier.bindings {
        let Ok(app_type) = AppType::from_str(&binding.app_type) else {
            skipped.push(DeleteSupplierSkip {
                app_type: binding.app_type.clone(),
                provider_id: binding.provider_id.clone(),
                reason: format!("未知应用 {}", binding.app_type),
            });
            continue;
        };
        match crate::services::provider::ProviderService::delete(
            state,
            app_type,
            &binding.provider_id,
        ) {
            Ok(()) => {
                deleted += 1;
                if !touched_apps.contains(&binding.app_type) {
                    touched_apps.push(binding.app_type.clone());
                }
            }
            Err(error) => skipped.push(DeleteSupplierSkip {
                app_type: binding.app_type.clone(),
                provider_id: binding.provider_id.clone(),
                reason: error.to_string(),
            }),
        }
    }

    // 删掉的接入由 delete_provider 顺带清好；删不掉的仍在。再兜一次孤儿清理，保证
    // 「全部删光」之后这家供应商立刻从总览里消失。
    {
        let mut conn = lock_conn!(state.db.conn);
        for app in &touched_apps {
            if let Err(error) =
                crate::database::Database::reconcile_provider_bindings(&mut conn, app)
            {
                log::warn!("[suppliers] 删除后对账 {app} 失败: {error}");
            }
        }
        if let Err(error) = crate::database::Database::cleanup_orphan_upstreams(&conn) {
            log::warn!("[suppliers] 删除后清理孤儿上游失败: {error}");
        }
    }

    let suppliers = collect(&state.db).map_err(|error| error.to_string())?;
    Ok(DeleteSupplierOutcome {
        suppliers,
        deleted,
        skipped,
    })
}

/// 一键采用：把网关上的某家供应商接进某个 agent。
///
/// 这是「填 0 次 key」的实现——地址和密钥都从网关读，agent 侧只生成一条配置。
///
/// 只对**能忠实渲染**的 app 开放：渲染错了比不渲染更糟。
/// 目前支持 `claude` / `claude-desktop`（同一套 `env` 形状）；
/// opencode 需要逐家维护 models 列表、codex 需要 TOML + auth，都不能凭空生成，
/// 因此明确拒绝而不是给个半成品。
#[tauri::command]
pub async fn adopt_supplier(
    state: tauri::State<'_, AppState>,
    app_type: String,
    supplier_id: String,
) -> Result<AdoptedProvider, String> {
    adopt_into(&state.db, &app_type, &supplier_id)
}

fn adopt_into(
    db: &crate::database::Database,
    app_type: &str,
    supplier_id: &str,
) -> Result<AdoptedProvider, String> {
    if !matches!(app_type, "claude" | "claude-desktop") {
        return Err(format!(
            "{app_type} 暂不支持一键采用：它的配置需要按供应商维护模型列表/认证，无法凭空生成。请在应用页手动添加。"
        ));
    }

    let conn = lock_conn!(db.conn);
    let endpoint_ids = endpoint_ids_of(&conn, supplier_id)?;
    if endpoint_ids.is_empty() {
        return Err("找不到这家供应商（可能已被删除）".to_string());
    }

    // 停用的供应商不再允许「采用」进来：采用会把它写成 agent 的直连配置，而停用是
    // 网关级决定，两边不一致只会制造「刚采用就报错」的困惑。
    let disabled = endpoint_ids.iter().any(|endpoint_id| {
        conn.query_row(
            "SELECT COALESCE(enabled, 1) FROM upstream_endpoints WHERE id = ?1",
            rusqlite::params![endpoint_id],
            |row| row.get::<_, i64>(0),
        )
        .map(|enabled| enabled == 0)
        .unwrap_or(false)
    });
    if disabled {
        return Err("这家供应商已停用，先启用再采用".to_string());
    }

    let (protocols, _) = endpoint_upstream_protocols(&conn).map_err(|error| error.to_string())?;

    // 直连模式下没有代理做转换，下游协议必须等于上游协议：
    // claude / claude-desktop 的下游是 Anthropic Messages，只能接 anthropic 上游。
    let mut target: Option<(String, String, String)> = None;
    for endpoint_id in &endpoint_ids {
        let protocol = protocols.get(endpoint_id).cloned().unwrap_or_default();
        if protocol != "anthropic" {
            continue;
        }
        let row = conn
            .query_row(
                "SELECT u.name, e.base_url
                 FROM upstream_endpoints e JOIN upstreams u ON u.id = e.upstream_id
                 WHERE e.id = ?1",
                rusqlite::params![endpoint_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .map_err(|error| error.to_string())?;
        target = Some((endpoint_id.clone(), row.0, row.1));
        break;
    }
    let Some((endpoint_id, name, base_url)) = target else {
        let protocols: Vec<String> = endpoint_ids
            .iter()
            .map(|id| {
                protocols
                    .get(id)
                    .cloned()
                    .unwrap_or_else(|| "unknown".to_string())
            })
            .collect();
        return Err(format!(
            "这家供应商的上游协议是 {}，直连模式的 {app_type} 只吃 anthropic 协议，无法直接采用（要走它得开代理路由）。",
            protocols.join("、")
        ));
    };

    let credential: Option<(Option<String>, String)> = conn
        .query_row(
            "SELECT secret, COALESCE(secret_kind, 'none') FROM upstream_credentials
             WHERE endpoint_id = ?1 ORDER BY app_type LIMIT 1",
            rusqlite::params![endpoint_id],
            |row| Ok((row.get::<_, Option<String>>(0)?, row.get::<_, String>(1)?)),
        )
        .ok();

    let secret = match credential {
        Some((Some(secret), kind)) if kind == "literal" && !secret.trim().is_empty() => secret,
        Some((_, kind)) if kind == "env_ref" => {
            return Err("这家供应商的密钥只存了环境变量名，库里没有明文，无法一键采用".to_string())
        }
        Some((_, kind)) if kind == "managed" => {
            return Err("这家供应商用的是托管登录态，无法一键采用".to_string())
        }
        _ => return Err("这家供应商还没有可用的密钥，请先补上".to_string()),
    };
    drop(conn);

    let digest = crate::database::Database::p1_stable_hash(&format!("{base_url}|{app_type}"));
    let provider_id = format!("sup-{digest}");
    let provider = crate::provider::Provider::with_id(
        provider_id.clone(),
        name.clone(),
        serde_json::json!({
            "env": {
                "ANTHROPIC_BASE_URL": base_url,
                "ANTHROPIC_AUTH_TOKEN": secret,
            }
        }),
        None,
    );
    db.save_provider(app_type, &provider)
        .map_err(|error| error.to_string())?;

    Ok(AdoptedProvider {
        app_type: app_type.to_string(),
        provider_id,
        provider_name: name,
    })
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdoptedProvider {
    pub app_type: String,
    pub provider_id: String,
    pub provider_name: String,
}

/// 在网关侧新建一家供应商，并直接接进选中的 app。
///
/// 这是「一次配齐」的入口：以前只能先在某个 app 里建一条 provider，再「采用」到别的
/// app；现在反过来——先在网关填名字/地址/密钥，勾选要吃这条路由的 agent。
/// 每个 app 落一条**确定性 id**（同一地址+app 永远同一条），重复提交不会出重复行。
#[tauri::command]
pub async fn create_supplier(
    state: tauri::State<'_, AppState>,
    name: String,
    base_url: String,
    api_key: String,
    apps: Vec<String>,
) -> Result<Vec<Supplier>, String> {
    create_into(&state.db, &name, &base_url, &api_key, &apps)
}

const CREATABLE_APPS: [&str; 2] = ["claude", "claude-desktop"];

fn create_into(
    db: &crate::database::Database,
    name: &str,
    base_url: &str,
    api_key: &str,
    apps: &[String],
) -> Result<Vec<Supplier>, String> {
    let name = name.trim();
    let api_key = api_key.trim();
    if name.is_empty() {
        return Err("供应商名称不能为空".to_string());
    }
    if api_key.is_empty() {
        return Err("密钥不能为空".to_string());
    }
    let (_, base_url) = crate::database::Database::p1_normalize_url(base_url)
        .ok_or_else(|| "base_url 格式无效（需要完整的 http(s) 地址）".to_string())?;

    // 先去重再校验：界面可能从多个入口勾同一个 app
    let mut targets: Vec<String> = Vec::new();
    for app in apps {
        let app = app.trim().to_string();
        if app.is_empty() || targets.contains(&app) {
            continue;
        }
        targets.push(app);
    }
    if targets.is_empty() {
        return Err("至少要接入一个应用".to_string());
    }
    if let Some(bad) = targets
        .iter()
        .find(|app| !CREATABLE_APPS.contains(&app.as_str()))
    {
        return Err(format!(
            "{bad} 暂不支持从网关新建：它的配置需要按供应商维护模型列表/认证，请在应用页手动添加。"
        ));
    }

    // 全部校验通过后才落库，避免"建了一半"这种半成品状态
    for app in &targets {
        // 直连写入：上游协议就是 anthropic（claude 系的下游协议）
        let digest = crate::database::Database::p1_stable_hash(&format!("{base_url}|{app}"));
        let provider = crate::provider::Provider::with_id(
            format!("sup-{digest}"),
            name.to_string(),
            serde_json::json!({
                "env": {
                    "ANTHROPIC_BASE_URL": base_url,
                    "ANTHROPIC_AUTH_TOKEN": api_key,
                }
            }),
            None,
        );
        db.save_provider(app, &provider)
            .map_err(|error| error.to_string())?;
    }

    let mut conn = lock_conn!(db.conn);
    let apps_in_db: Vec<String> = {
        let mut stmt = conn
            .prepare("SELECT DISTINCT app_type FROM providers")
            .map_err(|error| error.to_string())?;
        let rows = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|error| error.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())?
    };
    for app in apps_in_db {
        let _ = crate::database::Database::reconcile_provider_bindings(&mut conn, &app);
    }
    drop(conn);
    collect(db).map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn update_binding_model_map(
    state: tauri::State<'_, AppState>,
    app_type: String,
    provider_id: String,
    model_map: serde_json::Value,
) -> Result<Vec<Supplier>, String> {
    {
        let mut conn = lock_conn!(state.db.conn);
        let changed = crate::database::Database::update_binding_model_map(
            &mut conn,
            &app_type,
            &provider_id,
            &model_map,
        )
        .map_err(|error| error.to_string())?;
        if !changed {
            return Err(format!(
                "{app_type} 的这条配置里没有可写的模型字段（只有 claude 系支持模型映射）"
            ));
        }
        crate::database::Database::reconcile_provider_bindings(&mut conn, &app_type)
            .map_err(|error| error.to_string())?;
    }

    collect(&state.db).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;
    use serde_json::json;

    fn insert_provider(db: &Database, app: &str, id: &str, config: serde_json::Value) {
        let provider =
            crate::provider::Provider::with_id(id.to_string(), id.to_string(), config, None);
        db.save_provider(app, &provider).expect("插入 provider");
    }

    /// 带 meta 插入：托管登录的认证供应商与绑定账号都在 provider.meta 里。
    fn insert_provider_with_meta(
        db: &Database,
        app: &str,
        id: &str,
        config: serde_json::Value,
        meta: serde_json::Value,
    ) {
        let mut provider =
            crate::provider::Provider::with_id(id.to_string(), id.to_string(), config, None);
        provider.meta = Some(serde_json::from_value(meta).expect("provider meta"));
        db.save_provider(app, &provider).expect("插入 provider");
    }

    /// 托管登录的供应商要带上账号池信息，卡片才能列账号而不是只报「1 个凭据」。
    #[test]
    fn managed_provider_exposes_account_pool_metadata() {
        let db = Database::memory().expect("内存库");
        insert_provider_with_meta(
            &db,
            "claude-desktop",
            "codex-link",
            json!({"env": {
                "ANTHROPIC_BASE_URL": "https://chatgpt.com/backend-api/codex",
                "ANTHROPIC_AUTH_TOKEN": ""
            }}),
            json!({"authBinding": {
                "source": "managed_account",
                "authProvider": "codex_oauth",
                "accountId": "acc-default"
            }}),
        );
        let suppliers = collect(&db).expect("总览");
        let supplier = suppliers
            .iter()
            .find(|supplier| supplier.host == "chatgpt.com")
            .expect("codex 供应商");
        assert_eq!(
            supplier.managed_auth_provider.as_deref(),
            Some("codex_oauth")
        );
        assert_eq!(supplier.bound_account_ids, vec!["acc-default".to_string()]);
        assert!(supplier.credential_kinds.contains(&"managed".to_string()));
    }

    /// 老官方卡的 meta 没写 authBinding（Claude 官方卡是 {}）：按 provider id 兜底认成
    /// claude_official，卡片仍然能列出 Claude 账号池。
    #[test]
    fn official_provider_falls_back_to_known_auth_provider() {
        let db = Database::memory().expect("内存库");
        insert_provider(&db, "claude", "claude-official", json!({"env": {}}));
        let suppliers = collect(&db).expect("总览");
        let supplier = suppliers
            .iter()
            .find(|supplier| {
                supplier
                    .bindings
                    .iter()
                    .any(|binding| binding.provider_id == "claude-official")
            })
            .expect("Claude 官方供应商");
        assert_eq!(
            supplier.managed_auth_provider.as_deref(),
            Some("claude_official")
        );
        assert!(supplier.bound_account_ids.is_empty());
    }

    /// 同一个网关被两个 app 使用时，总览里应当只有一条供应商、一个端点，被 2 个 app 使用。
    #[test]
    fn overview_aggregates_one_supplier_across_apps() {
        let db = Database::memory().expect("内存库");
        let config = |key: &str| {
            json!({
                "env": {
                    "ANTHROPIC_BASE_URL": "https://ai.ctaigw.cn/v1",
                    "ANTHROPIC_AUTH_TOKEN": key,
                    "ANTHROPIC_DEFAULT_SONNET_MODEL": "kimi-k2",
                }
            })
        };
        insert_provider(&db, "claude", "a", config("sk-1"));
        insert_provider(&db, "claude-desktop", "b", config("sk-1"));

        let suppliers = collect(&db).expect("总览");
        let ctaigw: Vec<_> = suppliers
            .iter()
            .filter(|supplier| supplier.host == "ai.ctaigw.cn")
            .collect();
        assert_eq!(ctaigw.len(), 1, "同一网关只能出现一条供应商");
        let supplier = ctaigw[0];
        assert_eq!(supplier.upstream_protocol, "anthropic");
        assert_eq!(supplier.base_url, "https://ai.ctaigw.cn/v1");
        assert_eq!(supplier.apps, vec!["claude", "claude-desktop"]);
        assert_eq!(supplier.credential_count, 2);
        assert!(supplier.consumers.contains(&"claude:a".to_string()));
    }

    /// 一键采用：密钥和地址都从网关来，agent 侧一条配置都不用手填。
    #[test]
    fn adopt_renders_agent_config_from_gateway() -> Result<(), AppError> {
        let db = Database::memory().expect("内存库");
        insert_provider(
            &db,
            "claude",
            "src",
            json!({"env": {
                "ANTHROPIC_BASE_URL": "https://ai.ctaigw.cn/v1",
                "ANTHROPIC_AUTH_TOKEN": "sk-secret"
            }}),
        );
        let suppliers = collect(&db).expect("总览");
        let supplier = suppliers
            .iter()
            .find(|supplier| supplier.base_url == "https://ai.ctaigw.cn/v1")
            .expect("供应商");

        let adopted = adopt_into(&db, "claude-desktop", &supplier.id).expect("应能采用");
        // 供应商名沿用网关上的上游名（这里由 provider 名派生）
        assert!(!adopted.provider_name.trim().is_empty());
        assert!(adopted.provider_id.starts_with("sup-"));

        let stored: String = {
            let conn = lock_conn!(db.conn);
            conn.query_row(
                "SELECT settings_config FROM providers
                 WHERE app_type = 'claude-desktop' AND id = ?1",
                rusqlite::params![adopted.provider_id],
                |row| row.get(0),
            )
            .expect("provider 应已落库")
        };
        let stored: serde_json::Value = serde_json::from_str(&stored).expect("可解析");
        assert_eq!(
            stored["env"]["ANTHROPIC_BASE_URL"],
            json!("https://ai.ctaigw.cn/v1")
        );
        assert_eq!(stored["env"]["ANTHROPIC_AUTH_TOKEN"], json!("sk-secret"));

        // 采用后两个 app 应落在同一个网关上
        let suppliers = collect(&db).expect("总览2");
        let ctaigw: Vec<_> = suppliers
            .iter()
            .filter(|supplier| supplier.host == "ai.ctaigw.cn")
            .collect();
        assert_eq!(ctaigw.len(), 1, "同一网关只能出现一条供应商");
        assert_eq!(ctaigw[0].apps.len(), 2, "claude 与 claude-desktop 都应算上");
        Ok(())
    }

    /// 不能忠实渲染的 app 必须明确拒绝，不能给个半成品配置。
    #[test]
    fn adopt_rejects_apps_without_faithful_renderer() {
        let db = Database::memory().expect("内存库");
        insert_provider(
            &db,
            "claude",
            "src",
            json!({"env": {
                "ANTHROPIC_BASE_URL": "https://ai.ctaigw.cn/v1",
                "ANTHROPIC_AUTH_TOKEN": "sk-secret"
            }}),
        );
        let suppliers = collect(&db).expect("总览");
        let supplier_id = suppliers[0].id.clone();
        let error = adopt_into(&db, "opencode", &supplier_id).expect_err("应拒绝");
        assert!(error.contains("暂不支持"), "错误应说明原因: {error}");
    }

    /// 协议不匹配要拒绝：anthropic 的端点塞不进只吃 openai 协议的 agent。
    #[test]
    fn adopt_rejects_protocol_mismatch() {
        let db = Database::memory().expect("内存库");
        insert_provider(
            &db,
            "opencode",
            "src",
            json!({"options": {
                "baseURL": "https://wishub-x6.ctyun.cn/v1",
                "apiKey": "sk-oc"
            }}),
        );
        let suppliers = collect(&db).expect("总览");
        let supplier = &suppliers[0];
        // opencode 的协议由 npm 包决定，派生不出 anthropic —— 正是要拒绝的情况
        assert_ne!(supplier.upstream_protocol, "anthropic");
        let error = adopt_into(&db, "claude", &supplier.id).expect_err("应拒绝");
        assert!(error.contains("协议"), "错误应说明协议不匹配: {error}");
    }

    /// 临时：打印真实库副本的供应商总览，人工核对归并结果。
    #[test]
    #[ignore = "手动：仅调试用"]
    fn manual_dump_real_overview() -> Result<(), AppError> {
        let Ok(path) = std::env::var("CCS_VERIFY_DB") else {
            return Ok(());
        };
        let db = Database::open_for_test(std::path::Path::new(&path))?;
        let suppliers = collect(&db)?;
        println!("共 {} 家：", suppliers.len());
        for supplier in &suppliers {
            println!(
                "  {:<28} {:<14} {:<16} apps={:?} creds={} kinds={:?}",
                supplier.name,
                supplier.upstream_protocol,
                supplier.base_url,
                supplier.apps,
                supplier.credential_count,
                supplier.credential_kinds
            );
        }
        Ok(())
    }

    /// 手动跑（不在 CI 里）：拿真实数据库的副本验证 create_supplier 的整条链路。
    /// `CCS_VERIFY_DB=/path/to/copy.db cargo test --lib manual_create_against_real_db_copy -- --ignored --nocapture`
    #[test]
    #[ignore = "手动：用真实库副本验证，需 CCS_VERIFY_DB"]
    fn manual_create_against_real_db_copy() -> Result<(), AppError> {
        let Ok(path) = std::env::var("CCS_VERIFY_DB") else {
            return Ok(());
        };
        let db = Database::open_for_test(std::path::Path::new(&path))?;

        let before = collect(&db).expect("读总览");
        println!("建之前：{} 家供应商", before.len());

        let apps = vec!["claude".to_string(), "claude-desktop".to_string()];
        let suppliers = create_into(
            &db,
            "Verify Gateway",
            "https://verify.example.com/v1/",
            "sk-verify-real",
            &apps,
        )
        .map_err(AppError::Database)?;
        println!("建之后：{} 家供应商", suppliers.len());

        let supplier = suppliers
            .iter()
            .find(|supplier| supplier.name == "Verify Gateway")
            .expect("新供应商应在总览里");
        println!(
            "命中：name={} host={} apps={:?} protocol={} base_url={} creds={}",
            supplier.name,
            supplier.host,
            supplier.apps,
            supplier.upstream_protocol,
            supplier.base_url,
            supplier.credential_count
        );
        assert_eq!(supplier.host, "verify.example.com");
        assert_eq!(supplier.apps.len(), 2);
        assert_eq!(supplier.base_url, "https://verify.example.com/v1");

        // 每个 app 真的落了一条能读出地址/密钥的配置
        for app in ["claude", "claude-desktop"] {
            let config: String = {
                let conn = lock_conn!(db.conn);
                conn.query_row(
                    "SELECT settings_config FROM providers WHERE app_type = ?1 AND name = 'Verify Gateway'",
                    rusqlite::params![app],
                    |row| row.get(0),
                )
                .expect("配置应存在")
            };
            let config: serde_json::Value = serde_json::from_str(&config).expect("json");
            println!("{app}: {}", config);
            assert_eq!(
                config["env"]["ANTHROPIC_BASE_URL"],
                "https://verify.example.com/v1"
            );
            assert_eq!(config["env"]["ANTHROPIC_AUTH_TOKEN"], "sk-verify-real");
        }

        // 重复提交同一家：不该多出一行
        let again = create_into(
            &db,
            "Verify Gateway",
            "https://verify.example.com/v1",
            "sk-verify-real-2",
            &apps,
        )
        .map_err(AppError::Database)?;
        let dup = again.iter().filter(|s| s.name == "Verify Gateway").count();
        println!("重复提交后同名声明的供应商数：{dup}");
        assert_eq!(dup, 1, "重复提交不该堆行");
        Ok(())
    }

    /// 网关侧新建：一次填名字/地址/密钥，勾几个 app 就接几个。
    #[test]
    fn create_supplier_wires_every_selected_app() -> Result<(), AppError> {
        let db = Database::memory().expect("内存库");

        let suppliers = create_into(
            &db,
            "  CTYun  ",
            "https://ai.ctaigw.cn/v1/",
            "sk-test-123",
            &["claude".to_string(), "claude-desktop".to_string()],
        )
        .expect("新建成功");

        let supplier = suppliers
            .iter()
            .find(|supplier| supplier.name == "CTYun")
            .expect("新供应商应在总览里");
        assert_eq!(supplier.host, "ai.ctaigw.cn");
        assert_eq!(supplier.apps.len(), 2, "两个 app 都该接上");
        assert_eq!(
            supplier.base_url, "https://ai.ctaigw.cn/v1",
            "尾部斜杠应被规范化"
        );
        assert_eq!(supplier.upstream_protocol, "anthropic");
        assert_eq!(supplier.apps.len(), 2);

        // 每个 app 都真的落了一条能读出地址和密钥的配置
        for app in ["claude", "claude-desktop"] {
            let (config,): (String,) = {
                let conn = lock_conn!(db.conn);
                conn.query_row(
                    "SELECT settings_config FROM providers WHERE app_type = ?1",
                    rusqlite::params![app],
                    |row| Ok((row.get(0)?,)),
                )
                .expect("配置应存在")
            };
            let config: serde_json::Value = serde_json::from_str(&config).expect("json");
            assert_eq!(
                config["env"]["ANTHROPIC_BASE_URL"],
                "https://ai.ctaigw.cn/v1"
            );
            assert_eq!(config["env"]["ANTHROPIC_AUTH_TOKEN"], "sk-test-123");
        }
        Ok(())
    }

    /// 重复提交同一家供应商不该堆出第二行（确定性 id 的意义）。
    #[test]
    fn create_supplier_is_idempotent() -> Result<(), AppError> {
        let db = Database::memory().expect("内存库");
        let apps = vec!["claude".to_string()];
        create_into(&db, "Acme", "https://acme.dev/v1", "sk-1", &apps).expect("第一次");
        let suppliers =
            create_into(&db, "Acme", "https://acme.dev/v1", "sk-2", &apps).expect("第二次");

        assert_eq!(suppliers.len(), 1, "同一家只应有一条供应商");
        assert_eq!(suppliers[0].apps.len(), 1);
        // 第二次提交的密钥应覆盖第一条
        let config: String = {
            let conn = lock_conn!(db.conn);
            conn.query_row(
                "SELECT settings_config FROM providers WHERE app_type = 'claude'",
                [],
                |row| row.get(0),
            )
            .expect("配置")
        };
        assert!(config.contains("sk-2"), "后提交的密钥应生效");
        Ok(())
    }

    /// 校验失败时一条都不该落库——不能留下"建了一半"的供应商。
    #[test]
    fn create_supplier_rejects_without_partial_writes() -> Result<(), AppError> {
        let db = Database::memory().expect("内存库");

        let error = create_into(
            &db,
            "Acme",
            "https://acme.dev/v1",
            "sk-1",
            &["claude".to_string(), "codex".to_string()],
        )
        .expect_err("codex 不支持从网关新建");
        assert!(error.contains("codex"), "错误应点名是哪个 app: {error}");

        let count: i64 = {
            let conn = lock_conn!(db.conn);
            conn.query_row("SELECT COUNT(*) FROM providers", [], |row| row.get(0))
                .expect("计数")
        };
        assert_eq!(count, 0, "被拒时不该写入任何 provider");

        // 空名字 / 空密钥 / 非法地址 / 空 app 列表也要拦住
        let apps = vec!["claude".to_string()];
        assert!(create_into(&db, "  ", "https://a.dev", "k", &apps).is_err());
        assert!(create_into(&db, "A", "https://a.dev", "  ", &apps).is_err());
        assert!(create_into(&db, "A", "not-a-url", "k", &apps).is_err());
        assert!(create_into(&db, "A", "https://a.dev", "k", &[]).is_err());
        Ok(())
    }

    /// 没有显式 URL 的托管 provider 自成上游，不能被误并入别的网关。
    #[test]
    fn managed_provider_becomes_its_own_supplier() {
        let db = Database::memory().expect("内存库");
        insert_provider(&db, "claude", "claude-official", json!({"env": {}}));

        let suppliers = collect(&db).expect("总览");
        let official = suppliers
            .iter()
            .find(|supplier| supplier.base_url.is_empty())
            .expect("托管 provider 必须自成一条供应商");
        assert_eq!(official.host, "");
        assert_eq!(official.name, "claude-official");
    }

    /// 启停开关落库后总览要如实返回；托管账号没有网关地址，不允许启停。
    #[test]
    fn supplier_enabled_toggle_roundtrip() {
        let db = Database::memory().expect("内存库");
        insert_provider(
            &db,
            "claude",
            "a",
            json!({
                "env": {
                    "ANTHROPIC_BASE_URL": "https://acme.dev/v1",
                    "ANTHROPIC_AUTH_TOKEN": "sk-1",
                }
            }),
        );

        let supplier = collect(&db)
            .expect("总览")
            .into_iter()
            .find(|supplier| supplier.host == "acme.dev")
            .expect("应有这家供应商");
        assert!(supplier.enabled, "默认启用");

        let updated = set_supplier_enabled_in_db(&db, &supplier.id, false).expect("停用应成功");
        let disabled = updated
            .iter()
            .find(|item| item.id == supplier.id)
            .expect("停用后供应商还在总览里");
        assert!(!disabled.enabled, "停用后 enabled 应为 false");
        assert_eq!(disabled.consumers.len(), 1, "停用不删接入");

        let updated = set_supplier_enabled_in_db(&db, &supplier.id, true).expect("启用应成功");
        assert!(
            updated
                .iter()
                .find(|item| item.id == supplier.id)
                .expect("供应商")
                .enabled,
            "重新启用后应恢复"
        );
    }

    /// 托管登录账号没有网关地址，既不能停用，也不该被杀掉（删除只在账号页做）。
    #[test]
    fn managed_supplier_cannot_be_toggled() {
        let db = Database::memory().expect("内存库");
        insert_provider(&db, "claude", "claude-official", json!({"env": {}}));

        let supplier = collect(&db)
            .expect("总览")
            .into_iter()
            .find(|supplier| supplier.base_url.is_empty())
            .expect("托管供应商");
        let error =
            set_supplier_enabled_in_db(&db, &supplier.id, false).expect_err("托管账号不允许启停");
        assert!(error.contains("托管"), "错误应说明原因: {error}");
    }

    /// 删除供应商 = 删掉它在各 app 里的 provider；底层删光后总览里不应再出现。
    #[test]
    fn deleting_underlying_providers_removes_supplier() {
        let db = Database::memory().expect("内存库");
        for (app, id) in [("claude", "a"), ("claude-desktop", "b")] {
            insert_provider(
                &db,
                app,
                id,
                json!({
                    "env": {
                        "ANTHROPIC_BASE_URL": "https://acme.dev/v1",
                        "ANTHROPIC_AUTH_TOKEN": "sk-1",
                    }
                }),
            );
        }
        let supplier = collect(&db)
            .expect("总览")
            .into_iter()
            .find(|supplier| supplier.host == "acme.dev")
            .expect("应有这家供应商");
        assert_eq!(supplier.bindings.len(), 2, "两个 app 各一条接入");

        for (app, id) in [("claude", "a"), ("claude-desktop", "b")] {
            db.delete_provider(app, id).expect("删底层 provider");
        }

        assert!(
            collect(&db)
                .expect("总览")
                .iter()
                .all(|supplier| supplier.host != "acme.dev"),
            "底层 provider 删光后供应商应消失"
        );
    }
}

// >>> command adapters（生成物：scripts/gen_command_adapters.py，勿手改）
#[allow(clippy::all, dead_code, non_snake_case, unused_imports)]
pub(crate) mod ipc_adapters {
    include!("../ipc/adapters/suppliers.rs");
}
// <<< command adapters
