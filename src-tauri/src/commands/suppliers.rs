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

/// 一把凭据被哪些接入使用：一个 app 可以为同一把 Key 存多条 provider 配置。
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SupplierCredentialApp {
    pub app_type: String,
    pub provider_id: String,
    pub provider_name: String,
    /// 是否在该 app 的故障转移队列里；池内备用 Key 只有进队列才会被选路
    pub in_failover_queue: bool,
    /// 这条接入当前是否就是该 app 正在使用的 provider（托管账号池用来标「使用中」）
    pub is_current: bool,
}

/// 供应商的一把凭据（按密钥值去重后的视图）。
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SupplierCredential {
    /// 稳定指纹：literal 按密钥值，env_ref 按环境变量名
    pub key_hash: String,
    /// 掩码后的密钥，绝不返回明文
    pub masked_secret: String,
    /// literal / env_ref
    pub secret_kind: String,
    /// env_ref 时展示环境变量名
    pub env_name: Option<String>,
    /// 托管账号 id（`managed` 凭据才有）；前端据此关联授权中心的账号名称/头像
    pub account_id: Option<String>,
    /// 使用这把凭据的接入
    pub apps: Vec<SupplierCredentialApp>,
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
    /// 明文密钥的去重指纹：同一个 key 被多个 app 各存一份时只算一把，
    /// 供应商页据此显示「几把密钥」，而不是把每个 app 的接入记录都数一遍。
    pub credential_key_hashes: Vec<String>,
    /// 凭据明细：每把不同的明文 Key / 环境变量一行，含使用它的接入。
    /// 托管登录不在这里（账号由授权中心的账号池管理）。
    pub credentials: Vec<SupplierCredential>,
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

/// 密钥只展示头尾，前端永远拿不到明文。
fn mask_secret(secret: &str) -> String {
    let chars: Vec<char> = secret.chars().collect();
    if chars.len() <= 8 {
        return "••••".to_string();
    }
    let head: String = chars.iter().take(3).collect();
    let tail: String = chars.iter().skip(chars.len() - 4).collect();
    format!("{head}…{tail}")
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

    // endpoint_id → 明文密钥的去重指纹（同一个 key 在多个 app 里各存一份，只留一个）
    let mut per_endpoint_keys: BTreeMap<String, Vec<String>> = BTreeMap::new();
    {
        let mut stmt = conn.prepare(
            "SELECT endpoint_id, secret FROM upstream_credentials
             WHERE secret_kind = 'literal' AND secret IS NOT NULL AND secret <> ''",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        for row in rows {
            let (endpoint_id, secret) = row?;
            let hash = crate::database::Database::p1_stable_hash(&secret);
            let entry = per_endpoint_keys.entry(endpoint_id).or_default();
            if !entry.contains(&hash) {
                entry.push(hash);
            }
        }
    }

    // endpoint_id → 凭据明细（literal 按密钥值去重，env_ref 按环境变量名去重）
    let mut per_endpoint_credentials: BTreeMap<String, Vec<SupplierCredential>> = BTreeMap::new();
    {
        let mut stmt = conn.prepare(
            "SELECT c.endpoint_id, c.app_type, c.provider_id, c.secret, COALESCE(c.secret_kind, 'none'),
                    COALESCE(p.name, c.provider_id), COALESCE(p.settings_config, '{}'),
                    COALESCE(b.in_failover_queue, 0), COALESCE(p.is_current, 0)
             FROM upstream_credentials c
             LEFT JOIN providers p ON p.app_type = c.app_type AND p.id = c.provider_id
             LEFT JOIN provider_bindings b ON b.app_type = c.app_type AND b.provider_id = c.provider_id
             ORDER BY c.endpoint_id, c.app_type, c.provider_id",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, i64>(7)?,
                row.get::<_, i64>(8)?,
            ))
        })?;
        for row in rows {
            let (
                endpoint_id,
                app_type,
                provider_id,
                secret,
                kind,
                provider_name,
                config,
                in_queue,
                is_current,
            ) = row?;
            if kind != "literal" && kind != "env_ref" {
                continue;
            }
            let config: serde_json::Value = serde_json::from_str(&config).unwrap_or_default();
            let env_name = config
                .get("key_env")
                .and_then(|value| value.as_str())
                .filter(|name| !name.trim().is_empty())
                .map(str::to_string);
            let (key_hash, masked_secret) = if kind == "literal" {
                let secret = secret.unwrap_or_default();
                if secret.is_empty() {
                    continue;
                }
                (
                    crate::database::Database::p1_stable_hash(&secret),
                    mask_secret(&secret),
                )
            } else {
                let name = match env_name.clone() {
                    Some(name) => name,
                    None => continue,
                };
                (
                    crate::database::Database::p1_stable_hash(&format!("env:{name}")),
                    String::new(),
                )
            };
            let entries = per_endpoint_credentials.entry(endpoint_id).or_default();
            let credential = match entries.iter_mut().find(|item| item.key_hash == key_hash) {
                Some(item) => item,
                None => {
                    entries.push(SupplierCredential {
                        key_hash: key_hash.clone(),
                        masked_secret,
                        secret_kind: kind.clone(),
                        env_name: env_name.clone(),
                        account_id: None,
                        apps: Vec::new(),
                    });
                    entries.last_mut().expect("just pushed")
                }
            };
            if !credential
                .apps
                .iter()
                .any(|app| app.app_type == app_type && app.provider_id == provider_id)
            {
                credential.apps.push(SupplierCredentialApp {
                    app_type,
                    provider_id,
                    provider_name,
                    in_failover_queue: in_queue != 0,
                    is_current: is_current != 0,
                });
            }
        }
    }

    // endpoint_id → 托管账号（按 accountId 去重）。托管账号就是 provider 行上的
    // `authBinding.accountId`；同一个账号在多个 app 各有一条 provider 时合成一行。
    let mut per_endpoint_accounts: BTreeMap<String, Vec<SupplierCredential>> = BTreeMap::new();
    {
        let mut stmt = conn.prepare(
            "SELECT b.endpoint_id, b.app_type, b.provider_id,
                    COALESCE(p.name, c.provider_id), COALESCE(p.meta, ''),
                    COALESCE(b.in_failover_queue, 0), COALESCE(p.is_current, 0)
             FROM provider_bindings b
             LEFT JOIN providers p ON p.app_type = b.app_type AND p.id = b.provider_id
             LEFT JOIN upstream_credentials c ON c.app_type = b.app_type AND c.provider_id = b.provider_id
             ORDER BY b.endpoint_id, b.app_type, b.provider_id",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, i64>(5)?,
                row.get::<_, i64>(6)?,
            ))
        })?;
        for row in rows {
            let (endpoint_id, app_type, provider_id, provider_name, meta, in_queue, is_current) =
                row?;
            let (_, Some(account_id)) = managed_auth_binding(&meta, &provider_id) else {
                continue;
            };
            let key_hash = format!("account:{account_id}");
            let entries = per_endpoint_accounts.entry(endpoint_id).or_default();
            let credential = match entries.iter_mut().find(|item| item.key_hash == key_hash) {
                Some(item) => item,
                None => {
                    entries.push(SupplierCredential {
                        key_hash: key_hash.clone(),
                        masked_secret: String::new(),
                        secret_kind: "managed".to_string(),
                        env_name: None,
                        account_id: Some(account_id),
                        apps: Vec::new(),
                    });
                    entries.last_mut().expect("just pushed")
                }
            };
            if !credential
                .apps
                .iter()
                .any(|app| app.app_type == app_type && app.provider_id == provider_id)
            {
                credential.apps.push(SupplierCredentialApp {
                    app_type,
                    provider_id,
                    provider_name,
                    in_failover_queue: in_queue != 0,
                    is_current: is_current != 0,
                });
            }
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
                credential_key_hashes: Vec::new(),
                credentials: Vec::new(),
                managed_auth_provider: None,
                bound_account_ids: Vec::new(),
                upstream_ids: Vec::new(),
                enabled: enabled != 0,
            });

            // 托管身份也写在上游 id 上（`up-managed-<authProvider>`）：同一份订阅的每个
            // 端点都要认出是托管，否则 Codex 代理那一行会掉进 key 型分组、卡片上看不到它在用。
            if supplier.managed_auth_provider.is_none() {
                if let Some(provider) = upstream_id.strip_prefix("up-managed-") {
                    supplier.managed_auth_provider = Some(provider.to_string());
                }
            }

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
            for hash in per_endpoint_keys.get(&endpoint_id).into_iter().flatten() {
                if !supplier.credential_key_hashes.contains(hash) {
                    supplier.credential_key_hashes.push(hash.clone());
                }
            }
            for credential in per_endpoint_credentials
                .get(&endpoint_id)
                .into_iter()
                .flatten()
            {
                match supplier
                    .credentials
                    .iter_mut()
                    .find(|item| item.key_hash == credential.key_hash)
                {
                    Some(existing) => {
                        for app in &credential.apps {
                            if !existing.apps.iter().any(|current| {
                                current.app_type == app.app_type
                                    && current.provider_id == app.provider_id
                            }) {
                                existing.apps.push(app.clone());
                            }
                        }
                    }
                    None => supplier.credentials.push(credential.clone()),
                }
            }

            for credential in per_endpoint_accounts
                .get(&endpoint_id)
                .into_iter()
                .flatten()
            {
                match supplier
                    .credentials
                    .iter_mut()
                    .find(|item| item.key_hash == credential.key_hash)
                {
                    Some(existing) => {
                        for app in &credential.apps {
                            if !existing.apps.iter().any(|current| {
                                current.app_type == app.app_type
                                    && current.provider_id == app.provider_id
                            }) {
                                existing.apps.push(app.clone());
                            }
                        }
                    }
                    None => supplier.credentials.push(credential.clone()),
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

/// 改一家供应商的名字：同一家供应商的所有底层端点共用一个上游名。
#[tauri::command]
pub async fn update_supplier_name(
    state: tauri::State<'_, AppState>,
    supplier_id: String,
    name: String,
) -> Result<Vec<Supplier>, String> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err("名称不能为空".to_string());
    }
    if name.chars().count() > 120 {
        return Err("名称不能超过 120 个字".to_string());
    }

    {
        let mut conn = lock_conn!(state.db.conn);
        let endpoint_ids = endpoint_ids_of(&conn, &supplier_id)?;
        if endpoint_ids.is_empty() {
            return Err("找不到这家供应商（可能已被删除）".to_string());
        }

        let mut upstream_ids: Vec<String> = Vec::new();
        for endpoint_id in &endpoint_ids {
            let upstream_id: Option<String> = conn
                .query_row(
                    "SELECT upstream_id FROM upstream_endpoints WHERE id = ?1",
                    rusqlite::params![endpoint_id],
                    |row| row.get(0),
                )
                .ok();
            if let Some(upstream_id) = upstream_id {
                if !upstream_ids.contains(&upstream_id) {
                    upstream_ids.push(upstream_id);
                }
            }
        }
        if upstream_ids.is_empty() {
            return Err("找不到这家供应商（可能已被删除）".to_string());
        }

        let tx = conn.transaction().map_err(|error| error.to_string())?;
        for upstream_id in &upstream_ids {
            tx.execute(
                "UPDATE upstreams SET name = ?1 WHERE id = ?2",
                rusqlite::params![name, upstream_id],
            )
            .map_err(|error| error.to_string())?;
        }
        tx.commit().map_err(|error| error.to_string())?;
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

/// 让这家托管登录的供应商改用账号池里的某个账号。
///
/// 供应商卡片上的「设为默认」必须真的换掉这家供应商的账号：托管登录的凭据就是账号池
/// 里的某个账号，而网关取号是「接入绑的账号优先，没绑才用账号池的默认账号」。卡片上
/// 这家供应商的接入通常都带着 `authBinding.accountId`，只改全局默认账号对它一点用都
/// 没有（用户点了像没反应）。所以这里改每条接入的 `authBinding.accountId`，再交给
/// `ProviderService::update` 按当前模式重写 live / 代理契约（Codex 官方直连还会先把旧
/// 账号的登录代次采纳过来）。账号池的全局默认账号由调用方另外同步，那个字段只对没绑定
/// 账号的接入生效。
#[tauri::command(rename_all = "camelCase")]
pub async fn set_supplier_managed_account(
    state: tauri::State<'_, AppState>,
    supplier_ids: Vec<String>,
    account_id: String,
) -> Result<Vec<Supplier>, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        set_supplier_managed_account_inner(&state, &supplier_ids, &account_id)
    })
    .await
    .map_err(|error| format!("切换供应商账号任务失败: {error}"))?
}

fn set_supplier_managed_account_inner(
    state: &AppState,
    supplier_ids: &[String],
    account_id: &str,
) -> Result<Vec<Supplier>, String> {
    let account_id = account_id.trim();
    if account_id.is_empty() {
        return Err("账号 id 为空".to_string());
    }
    let suppliers = collect(&state.db).map_err(|error| error.to_string())?;
    let (auth_provider, targets) = managed_binding_targets(&suppliers, supplier_ids)?;

    let mut rebound = 0usize;
    for (app_type, provider_id) in &targets {
        let app = AppType::from_str(app_type).map_err(|error| error.to_string())?;
        let Some(mut provider) = state
            .db
            .get_provider_by_id(provider_id, app_type)
            .map_err(|error| error.to_string())?
        else {
            // 供应商是从 provider 对账派生出来的，行已经不在了就跳过，别让一条旧快照
            // 挡住其它接入的切换。
            continue;
        };
        let changed = apply_managed_account_binding(&mut provider, &auth_provider, account_id);
        if !changed {
            continue;
        }
        crate::services::provider::ProviderService::update(
            state,
            app,
            Some(provider_id.as_str()),
            provider,
        )
        .map_err(|error| format!("{app_type} 的接入改绑账号失败: {error}"))?;
        rebound += 1;
    }

    // 已经绑的就是这个账号时不算失败：卡片上的「默认」徽标可能落后于绑定（以前只改全局
    // 默认账号），点一下正好把两边对齐，不该报错。
    if rebound == 0 {
        log::info!("[suppliers] 供应商 {supplier_ids:?} 已绑定账号 {account_id}，无需改绑");
        return collect(&state.db).map_err(|error| error.to_string());
    }

    log::info!("[suppliers] 供应商账号切换: {auth_provider} -> {account_id}（{rebound} 条接入）");
    collect(&state.db).map_err(|error| error.to_string())
}

/// 这张卡片（同一账号服务的多条 supplier 行）里的接入：返回认证供应商和去重后的
/// `(app, providerId)`。托管登录服务必须一致，混着不同的账号服务时拒绝猜。
fn managed_binding_targets(
    suppliers: &[Supplier],
    supplier_ids: &[String],
) -> Result<(String, Vec<(String, String)>), String> {
    let mut auth_provider: Option<String> = None;
    let mut targets: Vec<(String, String)> = Vec::new();
    let mut matched = 0usize;
    for supplier in suppliers {
        if !supplier_ids.iter().any(|id| id == &supplier.id) {
            continue;
        }
        matched += 1;
        let Some(provider) = supplier.managed_auth_provider.as_deref() else {
            return Err("这家供应商用的是密钥，不是托管登录账号".to_string());
        };
        if auth_provider
            .as_deref()
            .is_some_and(|current| current != provider)
        {
            return Err("这张卡片里混了不同的托管登录服务，无法统一换账号".to_string());
        }
        auth_provider = Some(provider.to_string());
        for binding in &supplier.bindings {
            let target = (binding.app_type.clone(), binding.provider_id.clone());
            if !targets.contains(&target) {
                targets.push(target);
            }
        }
    }
    if matched == 0 {
        return Err("找不到这家供应商（可能已被删除）".to_string());
    }
    let auth_provider = auth_provider.ok_or_else(|| "这家供应商不是托管登录".to_string())?;
    if targets.is_empty() {
        return Err("这家供应商还没有接入，无法设置账号".to_string());
    }
    Ok((auth_provider, targets))
}

/// 把一条 provider 的托管账号绑定改成 `account_id`；返回是否有改动。
fn apply_managed_account_binding(
    provider: &mut crate::provider::Provider,
    auth_provider: &str,
    account_id: &str,
) -> bool {
    let meta = provider.meta.get_or_insert_with(Default::default);
    let binding = meta.auth_binding.get_or_insert_with(Default::default);
    let unchanged = binding.source == crate::provider::AuthBindingSource::ManagedAccount
        && binding.auth_provider.as_deref() == Some(auth_provider)
        && binding.account_id.as_deref() == Some(account_id);
    if unchanged {
        return false;
    }
    binding.source = crate::provider::AuthBindingSource::ManagedAccount;
    binding.auth_provider = Some(auth_provider.to_string());
    binding.account_id = Some(account_id.to_string());
    true
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
/// 目前支持 `claude` / `claude-desktop`（`env` 形状）、`codex`（auth + TOML）、
/// `gemini`（`env` 形状）；opencode / kimi 这类要逐家维护 models 列表的 app
/// 仍然明确拒绝，而不是给个半成品。
#[tauri::command]
pub async fn adopt_supplier(
    state: tauri::State<'_, AppState>,
    app_type: String,
    supplier_id: String,
) -> Result<AdoptedProvider, String> {
    adopt_into(&state.db, &app_type, &supplier_id)
}

/// 直连模式下没有代理做转换：下游协议必须等于上游协议，所以每种 app 只吃特定的上游协议。
fn adoptable_protocols(app_type: &str) -> Option<&'static [&'static str]> {
    match app_type {
        "claude" | "claude-desktop" => Some(&["anthropic"]),
        "codex" => Some(&["openai_responses", "openai_chat"]),
        "gemini" => Some(&["gemini_native"]),
        _ => None,
    }
}

/// 把 (app, 供应商) 渲染成该 app 的 `settings_config`。密钥只写进配置，不落别处。
fn render_adopted_config(
    app_type: &str,
    supplier_name: &str,
    base_url: &str,
    protocol: &str,
    secret: &str,
) -> serde_json::Value {
    let toml_string = |value: &str| serde_json::Value::from(value).to_string();
    match app_type {
        "codex" => {
            let wire_api = if protocol == "openai_chat" {
                "chat"
            } else {
                "responses"
            };
            let toml = format!(
                "model_provider = \"custom\"\n\n[model_providers.custom]\nname = {name}\nbase_url = {url}\nwire_api = \"{wire_api}\"\nrequires_openai_auth = true\n",
                name = toml_string(supplier_name),
                url = toml_string(base_url),
            );
            serde_json::json!({
                "auth": { "OPENAI_API_KEY": secret },
                "config": toml,
            })
        }
        "gemini" => serde_json::json!({
            "env": {
                "GOOGLE_GEMINI_BASE_URL": base_url,
                "GEMINI_API_KEY": secret,
            },
            "config": {},
        }),
        _ => serde_json::json!({
            "env": {
                "ANTHROPIC_BASE_URL": base_url,
                "ANTHROPIC_AUTH_TOKEN": secret,
            }
        }),
    }
}

fn adopt_into(
    db: &crate::database::Database,
    app_type: &str,
    supplier_id: &str,
) -> Result<AdoptedProvider, String> {
    let Some(accepted) = adoptable_protocols(app_type) else {
        return Err(format!(
            "{app_type} 暂不支持一键采用：它的配置需要按供应商维护模型列表/认证，无法凭空生成。请在应用页手动添加。"
        ));
    };

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

    let mut target: Option<(String, String, String, String)> = None;
    for endpoint_id in &endpoint_ids {
        let protocol = protocols.get(endpoint_id).cloned().unwrap_or_default();
        if !accepted.contains(&protocol.as_str()) {
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
        target = Some((endpoint_id.clone(), row.0, row.1, protocol));
        break;
    }
    let Some((endpoint_id, name, base_url, protocol)) = target else {
        let found: Vec<String> = endpoint_ids
            .iter()
            .map(|id| {
                protocols
                    .get(id)
                    .cloned()
                    .unwrap_or_else(|| "unknown".to_string())
            })
            .collect();
        return Err(format!(
            "这家供应商的上游协议是 {}，直连模式的 {app_type} 只吃 {} 协议，无法直接采用（要走它得开代理路由）。",
            found.join("、"),
            accepted.join(" / ")
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
    let settings = render_adopted_config(app_type, &name, &base_url, &protocol, &secret);
    let provider =
        crate::provider::Provider::with_id(provider_id.clone(), name.clone(), settings, None);
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
    protocol: Option<String>,
) -> Result<Vec<Supplier>, String> {
    let protocol = protocol.unwrap_or_else(|| "anthropic".to_string());
    create_into_with_protocol(&state.db, &name, &base_url, &api_key, &apps, &protocol)
}

const CREATABLE_APPS: [&str; 2] = ["claude", "claude-desktop"];

/// 托管服务身份 → (显示名, 上游协议)。
///
/// 只收「官方订阅」这三家：它们的供应商卡片语义明确（一家订阅 = 一家供应商），
/// 且协议由服务决定。Copilot / xAI / agy 的卡片来自 app 页登录后派生的接入，
/// 没有独立订阅的供应商语义，因此不在这里凭空建。
fn managed_supplier_profile(auth_provider: &str) -> Option<(&'static str, &'static str)> {
    match auth_provider {
        "codex_oauth" => Some(("OpenAI Official", "openai_responses")),
        "claude_official" => Some(("Claude Official", "anthropic")),
        "gemini_official" => Some(("Google Official", "gemini_native")),
        _ => None,
    }
}

/// 供应商页「导入本机 Agent」：把已经登录的官方订阅直接建成一家供应商（不接任何 app）。
///
/// 幂等：同一身份重复调用只会返回同一张卡（`up-managed-<authProvider>`），不会多建。
#[tauri::command(async)]
pub async fn ensure_managed_supplier(
    state: tauri::State<'_, AppState>,
    auth_provider: String,
) -> Result<Vec<Supplier>, String> {
    ensure_managed_supplier_into(&state.db, &auth_provider)
}

fn ensure_managed_supplier_into(
    db: &crate::database::Database,
    auth_provider: &str,
) -> Result<Vec<Supplier>, String> {
    let auth_provider = auth_provider.trim();
    let (name, protocol) = managed_supplier_profile(auth_provider)
        .ok_or_else(|| format!("{auth_provider} 暂不支持直接建供应商：请先在应用页登录该账号"))?;
    {
        let conn = lock_conn!(db.conn);
        crate::database::Database::ensure_managed_upstream(
            &conn,
            name,
            auth_provider,
            protocol,
            chrono::Utc::now().timestamp_millis(),
        )
        .map_err(|error| error.to_string())?;
    }
    collect(db).map_err(|error| error.to_string())
}

/// 测试用简写：默认按 anthropic 协议创建。
#[cfg(test)]
fn create_into(
    db: &crate::database::Database,
    name: &str,
    base_url: &str,
    api_key: &str,
    apps: &[String],
) -> Result<Vec<Supplier>, String> {
    create_into_with_protocol(db, name, base_url, api_key, apps, "anthropic")
}

fn create_into_with_protocol(
    db: &crate::database::Database,
    name: &str,
    base_url: &str,
    api_key: &str,
    apps: &[String],
    protocol: &str,
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
    // 创建时只渲染 anthropic 的 claude 配置；其它协议交给卡片上的「接入」按协议渲染。
    if !targets.is_empty() && protocol != "anthropic" {
        return Err(
            "这个协议暂时只能在创建后从供应商卡片接入应用；先在创建向导里不选应用。".to_string(),
        );
    }
    // 接入是可选项：创建供应商与接入 app 解耦，不勾任何 app 时只建供应商本体。
    if let Some(bad) = targets
        .iter()
        .find(|app| !CREATABLE_APPS.contains(&app.as_str()))
    {
        return Err(format!(
            "{bad} 暂不支持从网关新建：它的配置需要按供应商维护模型列表/认证，请在应用页手动添加。"
        ));
    }

    // 供应商本体先落库（上游 + 端点 + 密钥），与 app 无关；之后接入时派生逻辑会复用
    // 这行上游和端点，而不是另起一份。
    {
        let conn = lock_conn!(db.conn);
        crate::database::Database::create_manual_upstream(
            &conn,
            name,
            &base_url,
            protocol,
            Some(api_key),
            chrono::Utc::now().timestamp_millis(),
        )
        .map_err(|error| error.to_string())?;
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

/// 找出这家供应商下某个凭据指纹对应的接入：`(endpoint, app, provider, secret)`。
fn credential_targets(
    conn: &rusqlite::Connection,
    endpoint_ids: &[String],
    key_hash: &str,
) -> Result<Vec<(String, String, String, String)>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT endpoint_id, app_type, provider_id, COALESCE(secret, '')
             FROM upstream_credentials
             WHERE endpoint_id = ?1 AND COALESCE(secret_kind, 'none') = 'literal'
               AND secret IS NOT NULL AND secret <> ''
             ORDER BY app_type, provider_id",
        )
        .map_err(|error| error.to_string())?;
    let mut targets = Vec::new();
    for endpoint_id in endpoint_ids {
        let rows = stmt
            .query_map(rusqlite::params![endpoint_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })
            .map_err(|error| error.to_string())?;
        for row in rows {
            let (endpoint, app, provider, secret) = row.map_err(|error| error.to_string())?;
            if crate::database::Database::p1_stable_hash(&secret) == key_hash {
                targets.push((endpoint, app, provider, secret));
            }
        }
    }
    Ok(targets)
}

/// 只替换这一把 Key：同一家供应商下的其它 Key 原样保留。
#[tauri::command]
pub async fn update_supplier_credential(
    state: tauri::State<'_, AppState>,
    supplier_id: String,
    key_hash: String,
    secret: String,
) -> Result<Vec<Supplier>, String> {
    update_supplier_credential_into(&state.db, supplier_id, key_hash, secret)
}

fn update_supplier_credential_into(
    db: &crate::database::Database,
    supplier_id: String,
    key_hash: String,
    secret: String,
) -> Result<Vec<Supplier>, String> {
    let secret = secret.trim().to_string();
    if secret.is_empty() {
        return Err("密钥不能为空".to_string());
    }

    {
        let mut conn = lock_conn!(db.conn);
        let endpoint_ids = endpoint_ids_of(&conn, &supplier_id)?;
        if endpoint_ids.is_empty() {
            return Err("找不到这家供应商（可能已被删除）".to_string());
        }
        let targets = credential_targets(&conn, &endpoint_ids, &key_hash)?;
        if targets.is_empty() {
            return Err("找不到这把密钥（可能已被删除或已被替换）".to_string());
        }

        let tx = conn.transaction().map_err(|error| error.to_string())?;
        for (_, app, provider, _) in &targets {
            let config: String = tx
                .query_row(
                    "SELECT settings_config FROM providers WHERE app_type = ?1 AND id = ?2",
                    rusqlite::params![app, provider],
                    |row| row.get(0),
                )
                .map_err(|error| error.to_string())?;
            let mut config: serde_json::Value =
                serde_json::from_str(&config).unwrap_or(serde_json::Value::Null);
            if !crate::database::Database::p1_set_secret(&mut config, &secret) {
                return Err(format!(
                    "{app} 的这条配置里没有可写的密钥字段，无法只替换这一把 Key"
                ));
            }
            tx.execute(
                "UPDATE providers SET settings_config = ?1 WHERE app_type = ?2 AND id = ?3",
                rusqlite::params![config.to_string(), app, provider],
            )
            .map_err(|error| error.to_string())?;
        }
        tx.commit().map_err(|error| error.to_string())?;

        let apps: Vec<String> = {
            let mut list = Vec::new();
            for (_, app, _, _) in &targets {
                if !list.contains(app) {
                    list.push(app.clone());
                }
            }
            list
        };
        for app in apps {
            crate::database::Database::reconcile_provider_bindings(&mut conn, &app)
                .map_err(|error| error.to_string())?;
        }
    }

    collect(&db).map_err(|error| error.to_string())
}

/// 给指定 app 再加一把 Key：复制它在同一端点上的接入配置，换 Key，并加入故障转移队列。

#[tauri::command]
pub async fn add_supplier_credential(
    state: tauri::State<'_, AppState>,
    supplier_id: String,
    app_type: String,
    secret: String,
) -> Result<Vec<Supplier>, String> {
    add_supplier_credential_into(&state.db, supplier_id, app_type, secret)
}

fn add_supplier_credential_into(
    db: &crate::database::Database,
    supplier_id: String,
    app_type: String,
    secret: String,
) -> Result<Vec<Supplier>, String> {
    let secret = secret.trim().to_string();
    if secret.is_empty() {
        return Err("密钥不能为空".to_string());
    }

    {
        let conn = lock_conn!(db.conn);
        let endpoint_ids = endpoint_ids_of(&conn, &supplier_id)?;
        if endpoint_ids.is_empty() {
            return Err("找不到这家供应商（可能已被删除）".to_string());
        }
        // 同一家供应商可能有多个协议端点；要选这个 app 实际接入的那个
        let mut target: Option<(String, String, String)> = None;
        for endpoint_id in &endpoint_ids {
            let row = conn
                .query_row(
                    "SELECT b.provider_id, COALESCE(c.secret, '')
                     FROM provider_bindings b
                     LEFT JOIN upstream_credentials c
                       ON c.app_type = b.app_type AND c.provider_id = b.provider_id
                     WHERE b.endpoint_id = ?1 AND b.app_type = ?2
                       AND COALESCE(c.secret_kind, 'none') = 'literal'
                     ORDER BY b.provider_id LIMIT 1",
                    rusqlite::params![endpoint_id, app_type],
                    |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
                )
                .ok();
            if let Some((provider_id, secret)) = row {
                target = Some((endpoint_id.clone(), provider_id, secret));
                break;
            }
        }
        let Some((endpoint_id, base_provider_id, base_secret)) = target else {
            return Err(format!(
                "{app_type} 还没有接入这家供应商，无法在它的池子里加 Key"
            ));
        };

        if base_secret == secret {
            return Err("这把 Key 已经在这家供应商上了".to_string());
        }
        let existing_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM upstream_credentials
                 WHERE endpoint_id = ?1 AND app_type = ?2 AND COALESCE(secret_kind,'none')='literal'",
                rusqlite::params![endpoint_id, app_type],
                |row| row.get(0),
            )
            .unwrap_or(0);

        drop(conn);
        let provider = db
            .get_provider_by_id(&base_provider_id, &app_type)
            .map_err(|error| error.to_string())?
            .ok_or_else(|| "找不到要复制的接入配置".to_string())?;

        let digest = crate::database::Database::p1_stable_hash(&format!(
            "credential|{app_type}|{base_provider_id}|{secret}"
        ));
        let mut config = provider.settings_config.clone();
        if !crate::database::Database::p1_set_secret(&mut config, &secret) {
            return Err(format!(
                "{app_type} 的这条配置里没有可写的密钥字段，无法添加 Key"
            ));
        }
        let mut clone = provider.clone();
        clone.id = format!("sup-key-{digest}");
        clone.name = format!("{}（Key {}）", provider.name, existing_count + 1);
        clone.settings_config = config;
        clone.in_failover_queue = true;
        db.save_provider(&app_type, &clone)
            .map_err(|error| error.to_string())?;
        db.add_to_failover_queue(&app_type, &clone.id)
            .map_err(|error| error.to_string())?;

        let mut conn = lock_conn!(db.conn);
        crate::database::Database::reconcile_provider_bindings(&mut conn, &app_type)
            .map_err(|error| error.to_string())?;
    }

    collect(&db).map_err(|error| error.to_string())
}

/// 删除一把 Key：使用它的接入一起删掉；如果某个 app 只剩这一把，拒绝删除。

#[tauri::command]
pub async fn delete_supplier_credential(
    state: tauri::State<'_, AppState>,
    supplier_id: String,
    key_hash: String,
) -> Result<Vec<Supplier>, String> {
    delete_supplier_credential_into(&state.db, supplier_id, key_hash)
}

fn delete_supplier_credential_into(
    db: &crate::database::Database,
    supplier_id: String,
    key_hash: String,
) -> Result<Vec<Supplier>, String> {
    // 查询阶段单独持锁；后面的 set_current / delete 自己会拿锁，不能嵌套
    let (targets, blocked) = {
        let conn = lock_conn!(db.conn);
        let endpoint_ids = endpoint_ids_of(&conn, &supplier_id)?;
        if endpoint_ids.is_empty() {
            return Err("找不到这家供应商（可能已被删除）".to_string());
        }
        let targets = credential_targets(&conn, &endpoint_ids, &key_hash)?;
        if targets.is_empty() {
            return Err("找不到这把密钥（可能已被删除）".to_string());
        }

        let mut blocked: Vec<String> = Vec::new();
        for (endpoint, app, provider, _) in &targets {
            let others: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM provider_bindings b
                     LEFT JOIN upstream_credentials c
                       ON c.app_type = b.app_type AND c.provider_id = b.provider_id
                     WHERE b.endpoint_id = ?1 AND b.app_type = ?2 AND b.provider_id <> ?3
                       AND COALESCE(c.secret_kind,'none') = 'literal'",
                    rusqlite::params![endpoint, app, provider],
                    |row| row.get(0),
                )
                .unwrap_or(0);
            if others == 0 && !blocked.contains(app) {
                blocked.push(app.clone());
            }
        }
        (targets, blocked)
    };

    if !blocked.is_empty() {
        let mut blocked = blocked;
        blocked.sort();
        return Err(format!(
            "不能删除：{} 只剩这一把 Key。请先给它添加另一把 Key，或到应用页删除这条接入。",
            blocked.join("、")
        ));
    }

    for (endpoint, app, provider, _) in &targets {
        let (is_current, replacement) = {
            let conn = lock_conn!(db.conn);
            let is_current: bool = conn
                .query_row(
                    "SELECT is_current FROM providers WHERE app_type = ?1 AND id = ?2",
                    rusqlite::params![app, provider],
                    |row| row.get(0),
                )
                .unwrap_or(false);
            let replacement: Option<String> = if is_current {
                conn.query_row(
                    "SELECT b.provider_id FROM provider_bindings b
                     JOIN providers p ON p.app_type = b.app_type AND p.id = b.provider_id
                     LEFT JOIN upstream_credentials c
                       ON c.app_type = b.app_type AND c.provider_id = b.provider_id
                     WHERE b.app_type = ?1 AND b.endpoint_id = ?2
                       AND b.provider_id <> ?3 AND p.is_current = 0
                       AND COALESCE(c.secret_kind, 'none') = 'literal'
                     ORDER BY b.provider_id LIMIT 1",
                    rusqlite::params![app, endpoint, provider],
                    |row| row.get(0),
                )
                .ok()
            } else {
                None
            };
            (is_current, replacement)
        };
        if is_current {
            if let Some(replacement) = replacement {
                activate_current_account(db, app, &replacement)?;
            }
        }
        db.delete_provider(app, provider)
            .map_err(|error| error.to_string())?;
    }

    let apps: std::collections::BTreeSet<String> =
        targets.iter().map(|(_, app, _, _)| app.clone()).collect();
    let mut conn = lock_conn!(db.conn);
    for app in apps {
        crate::database::Database::reconcile_provider_bindings(&mut conn, &app)
            .map_err(|error| error.to_string())?;
    }
    drop(conn);

    collect(&db).map_err(|error| error.to_string())
}

/// 池内 Key 的启用/停用 = 是否进入该 app 的故障转移队列。

#[tauri::command]
pub async fn set_supplier_credential_enabled(
    state: tauri::State<'_, AppState>,
    supplier_id: String,
    key_hash: String,
    enabled: bool,
) -> Result<Vec<Supplier>, String> {
    set_supplier_credential_enabled_into(&state.db, supplier_id, key_hash, enabled)
}

fn set_supplier_credential_enabled_into(
    db: &crate::database::Database,
    supplier_id: String,
    key_hash: String,
    enabled: bool,
) -> Result<Vec<Supplier>, String> {
    let targets = {
        let conn = lock_conn!(db.conn);
        let endpoint_ids = endpoint_ids_of(&conn, &supplier_id)?;
        let targets = credential_targets(&conn, &endpoint_ids, &key_hash)?;
        if targets.is_empty() {
            return Err("找不到这把密钥".to_string());
        }
        targets
    };

    for (_, app, provider, _) in &targets {
        if enabled {
            db.add_to_failover_queue(app, provider)
                .map_err(|error| error.to_string())?;
        } else {
            let others: i64 = {
                let conn = lock_conn!(db.conn);
                conn.query_row(
                    "SELECT COUNT(*) FROM upstream_credentials c
                     JOIN providers p ON p.app_type = c.app_type AND p.id = c.provider_id
                     WHERE c.endpoint_id IN (SELECT endpoint_id FROM provider_bindings WHERE app_type = ?1 AND provider_id = ?2)
                       AND c.app_type = ?1 AND c.provider_id <> ?3 AND p.in_failover_queue = 1
                       AND COALESCE(c.secret_kind,'none') = 'literal'",
                    rusqlite::params![app, provider, provider],
                    |row| row.get(0),
                )
                .unwrap_or(0)
            };
            if others == 0 {
                return Err(format!("{app} 没有别的已启用 Key，不能停用最后一把"));
            }
            db.remove_from_failover_queue(app, provider)
                .map_err(|error| error.to_string())?;
        }
    }

    collect(&db).map_err(|error| error.to_string())
}

/// 一张卡片可能由多条 supplier 行组成（Codex + OpenAI Official），账号池按卡片操作。
fn supplier_endpoint_ids(
    conn: &rusqlite::Connection,
    supplier_ids: &[String],
) -> Result<Vec<String>, String> {
    let mut ids: Vec<String> = Vec::new();
    for supplier_id in supplier_ids {
        for endpoint_id in endpoint_ids_of(conn, supplier_id)? {
            if !ids.contains(&endpoint_id) {
                ids.push(endpoint_id);
            }
        }
    }
    if ids.is_empty() {
        return Err("找不到这家供应商（可能已被删除）".to_string());
    }
    Ok(ids)
}

/// 托管账号池：列出一家供应商下绑定某个账号的 provider 接入。
fn managed_account_targets(
    conn: &rusqlite::Connection,
    endpoint_ids: &[String],
    account_id: &str,
) -> Result<Vec<(String, String, String)>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT b.endpoint_id, b.app_type, b.provider_id
             FROM provider_bindings b
             JOIN providers p ON p.app_type = b.app_type AND p.id = b.provider_id
             WHERE b.endpoint_id = ?1
               AND json_extract(COALESCE(p.meta, '{}'), '$.authBinding.accountId') = ?2
             ORDER BY b.app_type, b.provider_id",
        )
        .map_err(|error| error.to_string())?;
    let mut targets = Vec::new();
    for endpoint_id in endpoint_ids {
        let rows = stmt
            .query_map(rusqlite::params![endpoint_id, account_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .map_err(|error| error.to_string())?;
        for row in rows {
            targets.push(row.map_err(|error| error.to_string())?);
        }
    }
    Ok(targets)
}

/// 队列状态写在 providers 表；绑定表是派生视图，要同步一下，否则 collect 读到的还是旧值。
fn sync_binding_queue(
    db: &crate::database::Database,
    app_type: &str,
    provider_id: &str,
    enabled: bool,
) -> Result<(), String> {
    let conn = lock_conn!(db.conn);
    conn.execute(
        "UPDATE provider_bindings SET in_failover_queue = ?1
         WHERE app_type = ?2 AND provider_id = ?3",
        rusqlite::params![if enabled { 1i64 } else { 0i64 }, app_type, provider_id],
    )
    .map_err(|error| error.to_string())?;
    Ok(())
}

/// 卡片下所有接入（不按端点过滤）：托管账号没有共享 base_url 时，每个账号可能是
/// 独立 supplier/端点，判断「还有没有别的账号」必须按整张卡看。
fn group_provider_bindings(
    conn: &rusqlite::Connection,
    endpoint_ids: &[String],
) -> Result<Vec<(String, String, String, bool, bool)>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT b.endpoint_id, b.app_type, b.provider_id, b.in_failover_queue,
                    COALESCE(p.is_current, 0)
             FROM provider_bindings b
             LEFT JOIN providers p ON p.app_type = b.app_type AND p.id = b.provider_id
             WHERE b.endpoint_id = ?1
             ORDER BY b.app_type, b.provider_id",
        )
        .map_err(|error| error.to_string())?;
    let mut rows_out = Vec::new();
    for endpoint_id in endpoint_ids {
        let rows = stmt
            .query_map(rusqlite::params![endpoint_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3).unwrap_or(0) != 0,
                    row.get::<_, i64>(4).unwrap_or(0) != 0,
                ))
            })
            .map_err(|error| error.to_string())?;
        for row in rows {
            rows_out.push(row.map_err(|error| error.to_string())?);
        }
    }
    Ok(rows_out)
}

/// 卡片内某个 app 当前的 provider（只认这张卡覆盖的端点）。
fn group_current_provider(
    conn: &rusqlite::Connection,
    endpoint_ids: &[String],
    app: &str,
) -> Option<String> {
    for endpoint_id in endpoint_ids {
        let found: Option<String> = conn
            .query_row(
                "SELECT b.provider_id
                 FROM provider_bindings b
                 JOIN providers p ON p.app_type = b.app_type AND p.id = b.provider_id
                 WHERE b.endpoint_id = ?1 AND b.app_type = ?2 AND p.is_current = 1
                 LIMIT 1",
                rusqlite::params![endpoint_id, app],
                |row| row.get(0),
            )
            .ok();
        if found.is_some() {
            return found;
        }
    }
    None
}

/// 一条接入行成为当前接入后，从故障转移队列里摘掉；队列只放「备用账号」。
fn activate_current_account(
    db: &crate::database::Database,
    app: &str,
    provider: &str,
) -> Result<(), String> {
    db.set_current_provider(app, provider)
        .map_err(|error| error.to_string())?;
    db.remove_from_failover_queue(app, provider)
        .map_err(|error| error.to_string())?;
    sync_binding_queue(db, app, provider, false)
}

/// 当前供应商被换下时，如果它还在账号池里，就回到故障转移队列，别在切号时失效。
fn park_previous_current(
    db: &crate::database::Database,
    app: &str,
    provider: &str,
) -> Result<(), String> {
    db.add_to_failover_queue(app, provider)
        .map_err(|error| error.to_string())?;
    sync_binding_queue(db, app, provider, true)
}

/// 添加一个托管账号：给卡片里每一条 supplier 行、每一个 app 复制一条 provider，绑定到该账号。
///
/// 托管账号没有 gateway 地址，账号池的“一行”就是一组 `provider.meta.authBinding.
/// accountId` 相同的 provider。复制成独立 provider 而不是改原 provider，是为了让
/// 账号之间能同时留在故障转移队列里；真正的选路仍由网关的 provider 队列完成。
#[tauri::command]
pub async fn add_supplier_account(
    state: tauri::State<'_, AppState>,
    supplier_ids: Vec<String>,
    account_id: String,
    name: Option<String>,
) -> Result<Vec<Supplier>, String> {
    add_supplier_account_into(&state.db, &supplier_ids, account_id, name)
}

fn add_supplier_account_into(
    db: &crate::database::Database,
    supplier_ids: &[String],
    account_id: String,
    name: Option<String>,
) -> Result<Vec<Supplier>, String> {
    let account_id = account_id.trim().to_string();
    if account_id.is_empty() {
        return Err("账号不能为空".to_string());
    }
    let label = name.unwrap_or_default().trim().to_string();

    let matched: Vec<Supplier> = collect(db)
        .map_err(|error| error.to_string())?
        .into_iter()
        .filter(|supplier| supplier_ids.iter().any(|id| id == &supplier.id))
        .collect();
    if matched.is_empty() {
        return Err("找不到这家供应商（可能已被删除）".to_string());
    }
    let mut auth_provider: Option<String> = None;
    for supplier in &matched {
        let Some(provider) = supplier.managed_auth_provider.as_deref() else {
            return Err("这家供应商不是托管登录，不能添加托管账号".to_string());
        };
        if auth_provider
            .as_deref()
            .is_some_and(|current| current != provider)
        {
            return Err("这张卡片里混了不同的托管登录服务，无法统一添加账号".to_string());
        }
        auth_provider = Some(provider.to_string());
    }
    let auth_provider = auth_provider.expect("matched is not empty");

    let mut bases: Vec<(String, String, String)> = Vec::new();
    let mut base_by_app: Vec<(String, String)> = Vec::new();
    let mut seen_apps: Vec<String> = Vec::new();
    {
        let conn = lock_conn!(db.conn);
        let endpoint_ids = supplier_endpoint_ids(&conn, supplier_ids)?;
        for endpoint_id in &endpoint_ids {
            let mut stmt = conn
                .prepare(
                    "SELECT DISTINCT b.app_type
                     FROM provider_bindings b
                     WHERE b.endpoint_id = ?1 ORDER BY b.app_type",
                )
                .map_err(|error| error.to_string())?;
            let apps: Vec<String> = stmt
                .query_map(rusqlite::params![endpoint_id], |row| row.get(0))
                .map_err(|error| error.to_string())?
                .collect::<Result<_, _>>()
                .map_err(|error| error.to_string())?;
            for app_type in apps {
                if seen_apps.contains(&app_type) {
                    continue;
                }
                let already: i64 = conn
                    .query_row(
                        "SELECT COUNT(*)
                         FROM provider_bindings b
                         JOIN providers p ON p.app_type = b.app_type AND p.id = b.provider_id
                         WHERE b.endpoint_id = ?1 AND b.app_type = ?2
                           AND json_extract(COALESCE(p.meta, '{}'), '$.authBinding.accountId') = ?3",
                        rusqlite::params![endpoint_id, app_type, account_id],
                        |row| row.get(0),
                    )
                    .unwrap_or(0);
                if already > 0 {
                    continue;
                }
                let base: Option<String> = conn
                    .query_row(
                        "SELECT b.provider_id
                         FROM provider_bindings b
                         JOIN providers p ON p.app_type = b.app_type AND p.id = b.provider_id
                         WHERE b.endpoint_id = ?1 AND b.app_type = ?2
                         ORDER BY p.is_current DESC, b.provider_id LIMIT 1",
                        rusqlite::params![endpoint_id, app_type],
                        |row| row.get(0),
                    )
                    .ok();
                if let Some(base) = base {
                    seen_apps.push(app_type.clone());
                    base_by_app.push((app_type.clone(), base.clone()));
                    bases.push((endpoint_id.clone(), app_type, base));
                }
            }
        }
    }

    if bases.is_empty() {
        return Err("这个账号已经在这家供应商上了".to_string());
    }

    let mut cloned_apps: Vec<String> = Vec::new();
    for (endpoint_id, app_type, base_provider_id) in bases {
        let provider = db
            .get_provider_by_id(&base_provider_id, &app_type)
            .map_err(|error| error.to_string())?
            .ok_or_else(|| "找不到要复制的接入配置".to_string())?;
        let digest = crate::database::Database::p1_stable_hash(&format!(
            "account|{endpoint_id}|{app_type}|{account_id}"
        ));
        let mut clone = provider.clone();
        clone.id = format!("sup-acct-{digest}");
        clone.name = if label.is_empty() {
            format!("{}（账号）", provider.name)
        } else {
            format!("{} · {}", provider.name, label)
        };
        let meta = clone.meta.get_or_insert_with(Default::default);
        let binding = meta.auth_binding.get_or_insert_with(Default::default);
        binding.source = crate::provider::AuthBindingSource::ManagedAccount;
        binding.auth_provider = Some(auth_provider.clone());
        binding.account_id = Some(account_id.clone());
        clone.in_failover_queue = true;
        db.save_provider(&app_type, &clone)
            .map_err(|error| error.to_string())?;
        db.add_to_failover_queue(&app_type, &clone.id)
            .map_err(|error| error.to_string())?;
        if !cloned_apps.contains(&app_type) {
            cloned_apps.push(app_type);
        }
    }

    {
        let mut conn = lock_conn!(db.conn);
        for app in &cloned_apps {
            crate::database::Database::reconcile_provider_bindings(&mut conn, app)
                .map_err(|error| error.to_string())?;
        }
    }

    // 新代码/新库上这个 app 可能还没有 current provider（账号池是后加的）。
    // 把复制时的原始 provider 设为当前，账号池才不会一加就「全部停用」。
    for (app, base) in &base_by_app {
        let needs_current = {
            let conn = lock_conn!(db.conn);
            let endpoint_ids = supplier_endpoint_ids(&conn, supplier_ids)?;
            group_current_provider(&conn, &endpoint_ids, app).is_none()
        };
        if needs_current {
            activate_current_account(db, app, base)?;
        }
    }

    collect(db).map_err(|error| error.to_string())
}

/// 把账号池里的某个账号设为这家供应商当前使用的账号（不合并、不删其它账号）。
///
/// 卡片上的「设为默认」在账号池存在时必须走这里：只改 current，账号池里其它 provider
/// 仍留在故障转移队列里，否则点一次就把多账号池压回单账号了。
#[tauri::command]
pub async fn set_supplier_current_account(
    state: tauri::State<'_, AppState>,
    supplier_ids: Vec<String>,
    account_id: String,
) -> Result<Vec<Supplier>, String> {
    set_supplier_current_account_into(&state.db, &supplier_ids, account_id)
}

fn set_supplier_current_account_into(
    db: &crate::database::Database,
    supplier_ids: &[String],
    account_id: String,
) -> Result<Vec<Supplier>, String> {
    let targets = {
        let conn = lock_conn!(db.conn);
        let endpoint_ids = supplier_endpoint_ids(&conn, supplier_ids)?;
        let targets = managed_account_targets(&conn, &endpoint_ids, &account_id)?;
        if targets.is_empty() {
            return Err("这个账号不在凭据池里".to_string());
        }
        targets
    };

    let mut apps: Vec<String> = Vec::new();
    for (_, app, provider) in &targets {
        if apps.contains(app) {
            continue;
        }
        apps.push(app.clone());
        let previous = {
            let conn = lock_conn!(db.conn);
            let endpoint_ids = supplier_endpoint_ids(&conn, supplier_ids)?;
            group_current_provider(&conn, &endpoint_ids, app)
        };
        if let Some(previous) = previous.filter(|previous| previous != provider) {
            park_previous_current(db, app, &previous)?;
        }
        activate_current_account(db, app, provider)?;
    }

    collect(db).map_err(|error| error.to_string())
}

/// 删除一个托管账号：使用它的接入一起删掉；如果某个 app 只剩这一个账号，拒绝删除。
#[tauri::command]
pub async fn delete_supplier_account(
    state: tauri::State<'_, AppState>,
    supplier_ids: Vec<String>,
    account_id: String,
) -> Result<Vec<Supplier>, String> {
    delete_supplier_account_into(&state.db, &supplier_ids, account_id)
}

fn delete_supplier_account_into(
    db: &crate::database::Database,
    supplier_ids: &[String],
    account_id: String,
) -> Result<Vec<Supplier>, String> {
    let (targets, blocked) = {
        let conn = lock_conn!(db.conn);
        let endpoint_ids = supplier_endpoint_ids(&conn, supplier_ids)?;
        let targets = managed_account_targets(&conn, &endpoint_ids, &account_id)?;
        if targets.is_empty() {
            return Err("找不到这个账号（可能已被删除）".to_string());
        }
        let all = group_provider_bindings(&conn, &endpoint_ids)?;
        let mut blocked: Vec<String> = Vec::new();
        for (_, app, provider) in &targets {
            let others = all
                .iter()
                .filter(|(_, other_app, other_provider, _, _)| {
                    other_app == app && other_provider != provider
                })
                .count();
            if others == 0 && !blocked.contains(app) {
                blocked.push(app.clone());
            }
        }
        (targets, blocked)
    };

    if !blocked.is_empty() {
        let mut blocked = blocked;
        blocked.sort();
        return Err(format!(
            "不能删除：{} 只剩这一个账号。请先添加另一个账号，或到应用页删除这条接入。",
            blocked.join("、")
        ));
    }

    for (_endpoint, app, provider) in &targets {
        let (is_current, replacement) = {
            let conn = lock_conn!(db.conn);
            let is_current: bool = conn
                .query_row(
                    "SELECT is_current FROM providers WHERE app_type = ?1 AND id = ?2",
                    rusqlite::params![app, provider],
                    |row| row.get(0),
                )
                .unwrap_or(false);
            let replacement: Option<String> = if is_current {
                let endpoint_ids = supplier_endpoint_ids(&conn, supplier_ids)?;
                let all = group_provider_bindings(&conn, &endpoint_ids)?;
                all.iter()
                    .find(|(_, other_app, other_provider, _, _)| {
                        other_app == app && other_provider != provider
                    })
                    .map(|(_, _, other_provider, _, _)| other_provider.clone())
            } else {
                None
            };
            (is_current, replacement)
        };
        if is_current {
            if let Some(replacement) = replacement {
                activate_current_account(db, app, &replacement)?;
            }
        }
        db.delete_provider(app, provider)
            .map_err(|error| error.to_string())?;
    }

    let apps: std::collections::BTreeSet<String> =
        targets.iter().map(|(_, app, _)| app.clone()).collect();
    let mut conn = lock_conn!(db.conn);
    for app in apps {
        crate::database::Database::reconcile_provider_bindings(&mut conn, &app)
            .map_err(|error| error.to_string())?;
    }
    drop(conn);

    collect(db).map_err(|error| error.to_string())
}

/// 托管账号的启用/停用 = 该账号的全部 provider 是否进入故障转移队列。
#[tauri::command]
pub async fn set_supplier_account_enabled(
    state: tauri::State<'_, AppState>,
    supplier_ids: Vec<String>,
    account_id: String,
    enabled: bool,
) -> Result<Vec<Supplier>, String> {
    set_supplier_account_enabled_into(&state.db, &supplier_ids, account_id, enabled)
}

fn set_supplier_account_enabled_into(
    db: &crate::database::Database,
    supplier_ids: &[String],
    account_id: String,
    enabled: bool,
) -> Result<Vec<Supplier>, String> {
    let targets = {
        let conn = lock_conn!(db.conn);
        let endpoint_ids = supplier_endpoint_ids(&conn, supplier_ids)?;
        let targets = managed_account_targets(&conn, &endpoint_ids, &account_id)?;
        if targets.is_empty() {
            return Err("找不到这个账号".to_string());
        }
        targets
    };

    for (_endpoint, app, provider) in &targets {
        if enabled {
            db.add_to_failover_queue(app, provider)
                .map_err(|error| error.to_string())?;
            sync_binding_queue(db, app, provider, true)?;
            continue;
        }

        let (others, replacement) = {
            let conn = lock_conn!(db.conn);
            let endpoint_ids = supplier_endpoint_ids(&conn, supplier_ids)?;
            let all = group_provider_bindings(&conn, &endpoint_ids)?;
            let others = all
                .iter()
                .filter(|(_, other_app, other_provider, in_queue, is_current)| {
                    other_app == app && other_provider != provider && (*in_queue || *is_current)
                })
                .count();
            let replacement = all
                .iter()
                .find(|(_, other_app, other_provider, in_queue, is_current)| {
                    other_app == app && other_provider != provider && (*in_queue || *is_current)
                })
                .map(|(_, _, other_provider, _, _)| other_provider.clone());
            (others, replacement)
        };
        if others == 0 {
            return Err(format!("{app} 没有别的已启用账号，不能停用最后一个"));
        }
        let is_current: bool = {
            let conn = lock_conn!(db.conn);
            conn.query_row(
                "SELECT is_current FROM providers WHERE app_type = ?1 AND id = ?2",
                rusqlite::params![app, provider],
                |row| row.get(0),
            )
            .unwrap_or(false)
        };
        if is_current {
            if let Some(replacement) = replacement {
                activate_current_account(db, app, &replacement)?;
            }
        }
        db.remove_from_failover_queue(app, provider)
            .map_err(|error| error.to_string())?;
        sync_binding_queue(db, app, provider, false)?;
    }

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
            .find(|supplier| supplier.managed_auth_provider.as_deref() == Some("codex_oauth"))
            .expect("codex 供应商");
        assert_eq!(
            supplier.managed_auth_provider.as_deref(),
            Some("codex_oauth")
        );
        assert_eq!(supplier.bound_account_ids, vec!["acc-default".to_string()]);
        assert!(supplier.credential_kinds.contains(&"managed".to_string()));
    }

    /// 托管账号池：同一个账号在多个 app 的 provider 合成一行；添加/启停/删除都按账号走。
    #[test]
    fn managed_account_pool_add_enable_delete() {
        let db = Database::memory().expect("内存库");
        let meta = |account: &str| {
            json!({"authBinding": {
                "source": "managed_account",
                "authProvider": "codex_oauth",
                "accountId": account
            }})
        };
        insert_provider_with_meta(
            &db,
            "claude-desktop",
            "codex-link",
            json!({"env": {
                "ANTHROPIC_BASE_URL": "https://chatgpt.com/backend-api/codex",
                "ANTHROPIC_AUTH_TOKEN": ""
            }}),
            meta("acc-a"),
        );
        insert_provider_with_meta(
            &db,
            "codex",
            "codex-official",
            json!({"auth": {}, "config": ""}),
            meta("acc-a"),
        );
        // 真实库里每家 app 都有一个 current provider；托管卡原本的账号就是当前账号。
        db.set_current_provider("claude-desktop", "codex-link")
            .expect("设置 claude-desktop 当前供应商");
        db.set_current_provider("codex", "codex-official")
            .expect("设置 codex 当前供应商");

        let suppliers = collect(&db).expect("总览");
        let ids: Vec<String> = suppliers
            .iter()
            .filter(|supplier| supplier.managed_auth_provider.as_deref() == Some("codex_oauth"))
            .map(|supplier| supplier.id.clone())
            .collect();
        assert_eq!(ids.len(), 2);
        let initial: Vec<&SupplierCredential> = suppliers
            .iter()
            .flat_map(|supplier| supplier.credentials.iter())
            .filter(|credential| credential.secret_kind == "managed")
            .collect();
        assert_eq!(initial.len(), 2, "每条 supplier 行各一行账号凭据");
        assert!(initial
            .iter()
            .all(|credential| credential.account_id.as_deref() == Some("acc-a")));
        assert!(initial.iter().all(|credential| credential.apps.len() == 1));

        let after_add =
            add_supplier_account_into(&db, &ids, "acc-b".to_string(), Some("B 账号".to_string()))
                .expect("添加账号");
        let ids: Vec<String> = after_add
            .iter()
            .filter(|supplier| supplier.managed_auth_provider.as_deref() == Some("codex_oauth"))
            .map(|supplier| supplier.id.clone())
            .collect();
        let managed: Vec<&SupplierCredential> = after_add
            .iter()
            .flat_map(|supplier| supplier.credentials.iter())
            .filter(|credential| credential.secret_kind == "managed")
            .collect();
        assert_eq!(managed.len(), 4, "两个账号 × 两条 supplier 行：{managed:?}");
        let b = managed
            .iter()
            .find(|credential| credential.account_id.as_deref() == Some("acc-b"))
            .expect("新账号");
        assert!(b.apps.iter().all(|app| app.in_failover_queue));

        let after_current = set_supplier_current_account_into(&db, &ids, "acc-b".to_string())
            .expect("切换当前账号");
        let b = after_current
            .iter()
            .flat_map(|supplier| supplier.credentials.iter())
            .find(|credential| credential.account_id.as_deref() == Some("acc-b"))
            .expect("当前账号");
        assert!(
            b.apps.iter().all(|app| app.is_current),
            "切换后当前账号应生效: {b:?}"
        );

        let after_disable =
            set_supplier_account_enabled_into(&db, &ids, "acc-b".to_string(), false)
                .expect("停用账号");
        let b = after_disable
            .iter()
            .flat_map(|supplier| supplier.credentials.iter())
            .find(|credential| credential.account_id.as_deref() == Some("acc-b"))
            .expect("停用后的账号");
        assert!(
            b.apps.iter().all(|app| !app.in_failover_queue),
            "停用后仍有队列行: {b:?}"
        );
        let a = after_disable
            .iter()
            .flat_map(|supplier| supplier.credentials.iter())
            .find(|credential| credential.account_id.as_deref() == Some("acc-a"))
            .expect("剩余账号");
        assert!(
            a.apps.iter().all(|app| app.is_current),
            "停用当前账号后应切回另一个已启用账号: {a:?}"
        );

        let error = set_supplier_account_enabled_into(&db, &ids, "acc-a".to_string(), false)
            .expect_err("不能停用最后一个已启用账号");
        assert!(error.contains("不能停用最后一个"), "{error}");

        let after_delete =
            delete_supplier_account_into(&db, &ids, "acc-a".to_string()).expect("删除账号");
        let managed: Vec<&SupplierCredential> = after_delete
            .iter()
            .flat_map(|supplier| supplier.credentials.iter())
            .filter(|credential| credential.secret_kind == "managed")
            .collect();
        assert_eq!(managed.len(), 2);
        assert!(managed
            .iter()
            .all(|credential| credential.account_id.as_deref() == Some("acc-b")));

        let error = delete_supplier_account_into(&db, &ids, "acc-b".to_string())
            .expect_err("最后一个账号不能删除");
        assert!(error.contains("只剩这一个账号"), "{error}");
    }

    /// 供应商卡片里的「设为默认」要改掉这张卡上**所有**接入的绑定账号，否则只改一半
    /// （比如只改 Codex 官方卡、漏掉 Claude Desktop）用户点完还是没反应。
    #[test]
    fn managed_binding_targets_covers_every_consumer_of_the_card() {
        let db = Database::memory().expect("内存库");
        let binding = |account: &str| {
            json!({"authBinding": {
                "source": "managed_account",
                "authProvider": "codex_oauth",
                "accountId": account
            }})
        };
        insert_provider_with_meta(
            &db,
            "claude-desktop",
            "codex-link",
            json!({"env": {
                "ANTHROPIC_BASE_URL": "https://chatgpt.com/backend-api/codex",
                "ANTHROPIC_AUTH_TOKEN": ""
            }}),
            binding("acc-old"),
        );
        insert_provider_with_meta(
            &db,
            "codex",
            "codex-official",
            json!({"auth": {}, "config": ""}),
            binding("acc-old"),
        );

        let suppliers = collect(&db).expect("总览");
        // 卡片是按托管登录服务合成的，可能跨多条 supplier 行（不同协议各一行），
        // 页面上一次换账号要把这一组里的接入全改掉。
        let ids: Vec<String> = suppliers
            .iter()
            .filter(|supplier| supplier.managed_auth_provider.as_deref() == Some("codex_oauth"))
            .map(|supplier| supplier.id.clone())
            .collect();
        assert_eq!(
            ids.len(),
            2,
            "两个 app 的托管行各自成一条 supplier：{ids:?}"
        );
        let (auth_provider, targets) = managed_binding_targets(&suppliers, &ids).expect("接入清单");

        assert_eq!(auth_provider, "codex_oauth");
        assert_eq!(targets.len(), 2, "两个 app 的接入都要换：{targets:?}");
        assert!(targets.contains(&("claude-desktop".to_string(), "codex-link".to_string())));
        assert!(targets.contains(&("codex".to_string(), "codex-official".to_string())));
    }

    /// 改绑只动 `authBinding`，没绑账号 / 不是这家服务时不瞎猜。
    #[test]
    fn apply_managed_account_binding_only_touches_the_binding() {
        let mut provider = crate::provider::Provider::with_id(
            "codex-official".to_string(),
            "OpenAI Official".to_string(),
            json!({"auth": {}, "config": "model = \"gpt-5\""}),
            None,
        );
        provider.meta = Some(
            serde_json::from_value(json!({"authBinding": {
                "source": "managed_account",
                "authProvider": "codex_oauth",
                "accountId": "acc-old"
            }}))
            .expect("provider meta"),
        );

        assert!(apply_managed_account_binding(
            &mut provider,
            "codex_oauth",
            "acc-new"
        ));
        let binding = provider
            .meta
            .as_ref()
            .and_then(|meta| meta.auth_binding.as_ref())
            .expect("托管账号绑定");
        assert_eq!(
            binding.source,
            crate::provider::AuthBindingSource::ManagedAccount
        );
        assert_eq!(binding.auth_provider.as_deref(), Some("codex_oauth"));
        assert_eq!(binding.account_id.as_deref(), Some("acc-new"));
        assert_eq!(
            provider.settings_config,
            json!({"auth": {}, "config": "model = \"gpt-5\""}),
            "只动绑定，凭据配置原样保留"
        );

        // 已经是这个账号就不用再写一遍（避免白重写 live / 代理契约）
        assert!(!apply_managed_account_binding(
            &mut provider,
            "codex_oauth",
            "acc-new"
        ));
    }

    /// 凭据明细：同一把 Key 跨 app 合并，不同 Key 各占一行；只返回掩码。
    #[test]
    fn credential_details_group_keys_and_never_expose_plaintext() {
        let db = Database::memory().expect("内存库");
        let config = |key: &str| {
            json!({"env": {
                "ANTHROPIC_BASE_URL": "https://pool.example.com/v1",
                "ANTHROPIC_AUTH_TOKEN": key
            }})
        };
        insert_provider(&db, "claude", "alpha", config("sk-alpha-123456"));
        insert_provider(&db, "claude", "beta", config("sk-beta-654321"));
        insert_provider(&db, "claude-desktop", "alpha2", config("sk-alpha-123456"));

        let suppliers = collect(&db).expect("总览");
        let supplier = suppliers
            .iter()
            .find(|supplier| supplier.host == "pool.example.com")
            .expect("供应商");
        assert_eq!(supplier.credentials.len(), 2, "两把不同的 Key");
        let alpha = supplier
            .credentials
            .iter()
            .find(|credential| credential.apps.len() == 2)
            .expect("跨 app 的同一把 Key 应合并");
        assert_eq!(alpha.secret_kind, "literal");
        assert!(
            !alpha.masked_secret.contains("alpha-123456"),
            "掩码不能泄露明文"
        );
        assert!(alpha.masked_secret.starts_with("sk-"));
        let beta = supplier
            .credentials
            .iter()
            .find(|credential| credential.apps.len() == 1)
            .expect("单独一把 Key");
        assert_eq!(beta.apps[0].provider_id, "beta");
        assert!(beta.apps[0].in_failover_queue == false);
    }

    fn secret_of(db: &Database, app: &str, id: &str) -> Result<String, AppError> {
        let conn = lock_conn!(db.conn);
        let config: String = conn
            .query_row(
                "SELECT settings_config FROM providers WHERE app_type = ?1 AND id = ?2",
                rusqlite::params![app, id],
                |row| row.get(0),
            )
            .expect("provider");
        let config: serde_json::Value = serde_json::from_str(&config).expect("json");
        Ok(config["env"]["ANTHROPIC_AUTH_TOKEN"]
            .as_str()
            .unwrap_or_default()
            .to_string())
    }

    fn provider_count(db: &Database, app: &str) -> Result<i64, AppError> {
        let conn = lock_conn!(db.conn);
        conn.query_row(
            "SELECT COUNT(*) FROM providers WHERE app_type = ?1",
            rusqlite::params![app],
            |row| row.get(0),
        )
        .map_err(|error| AppError::Database(error.to_string()))
    }

    #[test]
    fn replacing_one_credential_leaves_the_other_key_untouched() {
        let db = Database::memory().expect("内存库");
        let config = |key: &str| {
            json!({"env": {
                "ANTHROPIC_BASE_URL": "https://pool.example.com/v1",
                "ANTHROPIC_AUTH_TOKEN": key
            }})
        };
        insert_provider(&db, "claude", "a", config("sk-a-111111"));
        insert_provider(&db, "claude", "b", config("sk-b-222222"));

        let suppliers = collect(&db).expect("总览");
        let supplier = suppliers
            .iter()
            .find(|supplier| supplier.host == "pool.example.com")
            .expect("供应商");
        let hash_a = supplier
            .credentials
            .iter()
            .find(|credential| credential.apps.iter().any(|app| app.provider_id == "a"))
            .expect("a 的凭据")
            .key_hash
            .clone();

        update_supplier_credential_into(
            &db,
            supplier.id.clone(),
            hash_a,
            "sk-a-new-333333".to_string(),
        )
        .expect("替换一把");

        assert_eq!(
            secret_of(&db, "claude", "a").expect("读密钥"),
            "sk-a-new-333333"
        );
        assert_eq!(
            secret_of(&db, "claude", "b").expect("读密钥"),
            "sk-b-222222",
            "另一把 Key 必须原样保留"
        );
    }

    #[test]
    fn adding_a_credential_clones_binding_and_joins_failover_queue() {
        let db = Database::memory().expect("内存库");
        insert_provider(
            &db,
            "claude",
            "a",
            json!({"env": {
                "ANTHROPIC_BASE_URL": "https://pool.example.com/v1",
                "ANTHROPIC_AUTH_TOKEN": "sk-a-111111"
            }}),
        );
        let suppliers = collect(&db).expect("总览");
        let supplier = suppliers
            .iter()
            .find(|supplier| supplier.host == "pool.example.com")
            .expect("供应商");

        add_supplier_credential_into(
            &db,
            supplier.id.clone(),
            "claude".to_string(),
            "sk-added-222222".to_string(),
        )
        .expect("加 Key");

        assert_eq!(provider_count(&db, "claude").expect("计数"), 2);
        let suppliers = collect(&db).expect("总览2");
        let supplier = suppliers
            .iter()
            .find(|supplier| supplier.host == "pool.example.com")
            .expect("供应商2");
        assert_eq!(supplier.credentials.len(), 2);
        let added = supplier
            .credentials
            .iter()
            .find(|credential| {
                credential
                    .apps
                    .iter()
                    .any(|app| app.provider_id.starts_with("sup-key-"))
            })
            .expect("新增凭据");
        assert!(added.apps[0].in_failover_queue, "新 Key 应进故障转移队列");
    }

    #[test]
    fn deleting_the_only_credential_is_rejected() {
        let db = Database::memory().expect("内存库");
        insert_provider(
            &db,
            "claude",
            "a",
            json!({"env": {
                "ANTHROPIC_BASE_URL": "https://pool.example.com/v1",
                "ANTHROPIC_AUTH_TOKEN": "sk-only-111111"
            }}),
        );
        let suppliers = collect(&db).expect("总览");
        let supplier = suppliers
            .iter()
            .find(|supplier| supplier.host == "pool.example.com")
            .expect("供应商");
        let hash = supplier.credentials[0].key_hash.clone();

        let error = delete_supplier_credential_into(&db, supplier.id.clone(), hash)
            .expect_err("只剩一把时不能删");
        assert!(error.contains("只剩这一把"), "错误原因：{error}");
        assert_eq!(provider_count(&db, "claude").expect("计数"), 1);
    }

    #[test]
    fn deleting_one_of_two_credentials_removes_only_that_binding() {
        let db = Database::memory().expect("内存库");
        let config = |key: &str| {
            json!({"env": {
                "ANTHROPIC_BASE_URL": "https://pool.example.com/v1",
                "ANTHROPIC_AUTH_TOKEN": key
            }})
        };
        insert_provider(&db, "claude", "a", config("sk-a-111111"));
        insert_provider(&db, "claude", "b", config("sk-b-222222"));
        let suppliers = collect(&db).expect("总览");
        let supplier = suppliers
            .iter()
            .find(|supplier| supplier.host == "pool.example.com")
            .expect("供应商");
        let hash_a = supplier
            .credentials
            .iter()
            .find(|credential| credential.apps.iter().any(|app| app.provider_id == "a"))
            .expect("a 的凭据")
            .key_hash
            .clone();

        delete_supplier_credential_into(&db, supplier.id.clone(), hash_a).expect("删一把");

        assert_eq!(provider_count(&db, "claude").expect("计数"), 1);
        assert_eq!(
            secret_of(&db, "claude", "b").expect("读密钥"),
            "sk-b-222222"
        );
    }

    #[test]
    fn mask_secret_keeps_only_prefix_and_suffix() {
        assert_eq!(mask_secret("sk-1234567890abcd"), "sk-…abcd");
        assert_eq!(mask_secret("short"), "••••");
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

    /// 采用 codex：openai_responses 上游写 wire_api = responses，密钥进 auth。
    #[test]
    fn adopt_renders_codex_config() -> Result<(), AppError> {
        let db = Database::memory()?;
        {
            let conn = lock_conn!(db.conn);
            Database::create_manual_upstream(
                &conn,
                "Relay",
                "https://relay.dev/v1",
                "openai_responses",
                Some("sk-relay"),
                1,
            )?;
        }
        let suppliers = collect(&db)?;
        let supplier = suppliers
            .iter()
            .find(|supplier| supplier.host == "relay.dev")
            .expect("供应商");
        adopt_into(&db, "codex", &supplier.id).map_err(AppError::Message)?;

        let stored: String = {
            let conn = lock_conn!(db.conn);
            conn.query_row(
                "SELECT settings_config FROM providers WHERE app_type = 'codex'",
                [],
                |row| row.get(0),
            )?
        };
        let stored: serde_json::Value = serde_json::from_str(&stored).expect("json");
        assert_eq!(stored["auth"]["OPENAI_API_KEY"], json!("sk-relay"));
        let toml = stored["config"].as_str().expect("config 应为 TOML 文本");
        assert!(toml.contains("base_url = \"https://relay.dev/v1\""));
        assert!(toml.contains("wire_api = \"responses\""));
        Ok(())
    }

    /// 采用 gemini：地址与密钥写进 env，且要求上游就是 gemini_native。
    #[test]
    fn adopt_renders_gemini_config() -> Result<(), AppError> {
        let db = Database::memory()?;
        {
            let conn = lock_conn!(db.conn);
            Database::create_manual_upstream(
                &conn,
                "Gem",
                "https://gem.example.dev",
                "gemini_native",
                Some("gm-key"),
                1,
            )?;
        }
        let suppliers = collect(&db)?;
        let supplier = suppliers
            .iter()
            .find(|supplier| supplier.host == "gem.example.dev")
            .expect("供应商");
        adopt_into(&db, "gemini", &supplier.id).map_err(AppError::Message)?;

        let stored: String = {
            let conn = lock_conn!(db.conn);
            conn.query_row(
                "SELECT settings_config FROM providers WHERE app_type = 'gemini'",
                [],
                |row| row.get(0),
            )?
        };
        let stored: serde_json::Value = serde_json::from_str(&stored).expect("json");
        assert_eq!(
            stored["env"]["GOOGLE_GEMINI_BASE_URL"],
            json!("https://gem.example.dev")
        );
        assert_eq!(stored["env"]["GEMINI_API_KEY"], json!("gm-key"));
        Ok(())
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
        Ok(())
    }

    /// 创建供应商与接入 app 解耦：不勾任何 app 也能建，且跨对账（含孤儿清理）存活。
    #[test]
    fn create_supplier_without_apps_creates_standalone_supplier() -> Result<(), AppError> {
        let db = Database::memory().expect("内存库");

        let suppliers = create_into(&db, "Acme", "https://acme.dev/v1", "sk-standalone", &[])
            .expect("不接入 app 也应创建成功");
        let supplier = suppliers
            .iter()
            .find(|supplier| supplier.name == "Acme")
            .expect("供应商应在总览里");
        assert_eq!(supplier.host, "acme.dev");
        assert!(supplier.apps.is_empty(), "还没接入任何 app");
        assert_eq!(supplier.credential_count, 1, "密钥应已入池");
        assert_eq!(supplier.credentials.len(), 1, "凭据池应能读出这把密钥");

        // collect() 每次读之前都会对账 + 孤儿清理；手动建的供应商不能因此消失。
        let again = collect(&db).expect("再次总览");
        assert!(
            again.iter().any(|supplier| supplier.name == "Acme"),
            "零接入的手动供应商必须跨对账存活"
        );

        // 之后接入 app 时复用同一行上游，而不是再造一家。
        let with_app = create_into(
            &db,
            "Acme",
            "https://acme.dev/v1",
            "sk-standalone",
            &["claude".to_string()],
        )
        .expect("再接入 claude");
        let acme: Vec<&Supplier> = with_app
            .iter()
            .filter(|supplier| supplier.name == "Acme")
            .collect();
        assert_eq!(acme.len(), 1, "同一家供应商不该被拆成两条");
        assert_eq!(acme[0].apps, vec!["claude".to_string()]);
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

    /// P3：一份 Codex 订阅在两个 app 里各有 provider 行（Claude Desktop 的 Codex 代理 +
    /// Codex 官方占位），总览必须只出现一家供应商、两条接入——否则模型目录会按两家上游
    /// 各存一套（页面上就是「16 个模型」）。
    #[test]
    fn managed_codex_subscription_merges_into_one_supplier() -> Result<(), AppError> {
        let db = Database::memory().expect("内存库");
        insert_provider_with_meta(
            &db,
            "claude-desktop",
            "9a07b5f3",
            json!({"env": {
                "ANTHROPIC_BASE_URL": "https://chatgpt.com/backend-api/codex"
            }}),
            json!({"claudeDesktopMode": "proxy"}),
        );
        insert_provider_with_meta(
            &db,
            "codex",
            "codex-official",
            json!({}),
            json!({"authBinding": {
                "source": "managed_account",
                "authProvider": "codex_oauth",
                "accountId": "acc-1"
            }}),
        );

        let suppliers = collect(&db).expect("总览");
        let codex: Vec<_> = suppliers
            .iter()
            .filter(|supplier| supplier.managed_auth_provider.as_deref() == Some("codex_oauth"))
            .collect();
        // 后端一个供应商 = 一条上游端点（两个接入两个端点），前端按托管身份合成一张卡；
        // 关键是它们必须认成同一家上游、两个 app 都在。
        assert!(!codex.is_empty(), "同一份订阅只能是同一家上游");
        for supplier in &codex {
            assert_eq!(
                supplier.upstream_ids,
                vec!["up-managed-codex_oauth".to_string()],
                "同一份订阅的每个端点都要挂在同一个上游 id 上"
            );
        }
        let mut apps: Vec<String> = codex
            .iter()
            .flat_map(|supplier| supplier.apps.clone())
            .collect();
        apps.sort();
        apps.dedup();
        assert_eq!(
            apps,
            vec!["claude-desktop".to_string(), "codex".to_string()]
        );
        let mut protocols: Vec<String> = codex
            .iter()
            .flat_map(|supplier| {
                supplier
                    .bindings
                    .iter()
                    .map(|binding| binding.downstream_protocol.clone())
            })
            .collect();
        protocols.sort();
        protocols.dedup();
        assert_eq!(
            protocols,
            vec!["anthropic".to_string(), "openai_responses".to_string()]
        );

        Ok(())
    }

    /// P4：导入本机登录态 = 建一家托管供应商，不接任何 app，也不写明文密钥。
    /// 重复导入必须幂等：同一份订阅只能有一家上游。
    #[test]
    fn ensure_managed_supplier_is_idempotent() -> Result<(), AppError> {
        let db = Database::memory().expect("内存库");
        let first = ensure_managed_supplier_into(&db, "codex_oauth").expect("导入");
        let second = ensure_managed_supplier_into(&db, "codex_oauth").expect("重复导入");

        for list in [&first, &second] {
            let codex: Vec<_> = list
                .iter()
                .filter(|supplier| supplier.managed_auth_provider.as_deref() == Some("codex_oauth"))
                .collect();
            assert_eq!(codex.len(), 1, "一张卡");
            assert_eq!(
                codex[0].upstream_ids,
                vec!["up-managed-codex_oauth".to_string()]
            );
            assert!(codex[0].apps.is_empty(), "导入不应接入任何 app");
            // 托管订阅的凭据是账号池里的 OAuth 账号，不是 upstream_credentials 行
            assert!(codex[0].credential_kinds.is_empty());
            assert_eq!(codex[0].credential_count, 0);
        }
        assert_eq!(first[0].id, second[0].id);

        let upstreams: i64 = {
            let conn = crate::database::lock_conn!(db.conn);
            conn.query_row(
                "SELECT COUNT(*) FROM upstreams WHERE id LIKE 'up-managed-%'",
                [],
                |row| row.get(0),
            )?
        };
        assert_eq!(upstreams, 1, "重复导入不能堆出第二条上游");
        Ok(())
    }

    /// 创建向导里「预设」可能带 openai_responses / gemini_native 协议：这类供应商不能在
    /// 创建时直接接 claude 系（会写成 anthropic 配置），必须拒绝并提示创建后再接入。
    #[test]
    fn create_rejects_apps_for_non_anthropic_protocol() {
        let db = Database::memory().expect("内存库");
        let error = create_into_with_protocol(
            &db,
            "Codex 网关",
            "https://api.example.com/v1",
            "sk-1",
            &["claude".to_string()],
            "openai_responses",
        )
        .expect_err("必须拒绝");
        assert!(error.contains("创建后"), "{error}");
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
