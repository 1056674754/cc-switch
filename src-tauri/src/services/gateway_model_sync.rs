//! 上游 `/v1/models` 的发现与落库（供应商编辑弹窗里「供应商模型」的刷新）。
//!
//! 目录原本只有两个来源：provider 行派生 + 人工覆盖。这里补第三个来源——直接问上游
//! 要模型列表，落到 `gateway_upstream_models` 当**候选**。候选默认写一条 `hidden = true`
//! 的覆盖，所以刷新一次不会把上游几十个模型铺进每个 app 的模型选择器；用户点「加入」
//! 才真正放开（`hidden = false`），此时它和派生条目一样可路由、可发布。
//!
//! 密钥只在 Rust 侧读：前端拿不到明文（凭证是只写的），所以这一步必须在这里做。

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use rusqlite::OptionalExtension;

use crate::database::Database;
use crate::error::AppError;

/// 一次「刷新供应商模型」的结果，直接回给前端弹 toast。
///
/// 计数按**真实上游**算：同一家上游在库里可能对应多行，只发一次请求、只算一次模型数。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SupplierModelFetch {
    /// 这次真的刷新过的 upstream 行。
    pub upstream_ids: Vec<String>,
    /// 上游返回、去重去空之后的模型数。
    pub fetched: usize,
    /// 这次新出现、被默认隐藏（未启用）的模型数（按模型名去重）。
    pub added: usize,
}

/// 拉 `/v1/models` 需要的上游端点信息。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FetchTarget {
    pub base_url: String,
    pub secret: String,
    /// `model_fetch` 的 apiFormat：Anthropic / Gemini 与 OpenAI 兼容在鉴权和路径上不同。
    pub api_format: Option<&'static str>,
    /// baseURL 是不是写成了整条请求地址（`.../v1/chat/completions`），是则取列表要先回推根路径。
    pub is_full_url: bool,
}

/// 这次刷新实际怎么拿到模型列表。
///
/// 目录里的上游有两种凭证形态：自定义网关是明文 key（OpenAI 兼容 `/v1/models`）；
/// 官方 / 托管登录是 OAuth 账号，Codex 走的是 `chatgpt.com/backend-api/codex/models`，
/// 根本不是 OpenAI 兼容端点。两种都要能刷新，否则托管登录的供应商按钮永远报错。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FetchSource {
    /// 明文密钥 → OpenAI 兼容端点。
    Literal(FetchTarget),
    /// 托管 Codex OAuth 账号 → ChatGPT Codex 后端的模型列表端点。
    CodexOAuth { account_id: String },
}

/// 上游端点行的取证形状：`(端点 id, 协议, baseURL, 明文密钥)`。
type EndpointRow = (String, String, String, String);

/// baseURL 被填成整条请求地址的常见形态。供应商表单允许这么填（`isFullUrl` 开关），
/// 库里只存了字符串，取模型列表前得先认出来，否则会拼成 `.../completions/v1/models`。
fn looks_like_endpoint_url(base_url: &str) -> bool {
    const ENDPOINT_SUFFIXES: &[&str] = &[
        "/chat/completions",
        "/completions",
        "/responses",
        "/messages",
        "/v1beta/models",
    ];
    let trimmed = base_url.trim().trim_end_matches('/');
    ENDPOINT_SUFFIXES
        .iter()
        .any(|suffix| trimmed.ends_with(suffix))
}

/// 上游协议 → `model_fetch` 的 apiFormat。
///
/// 上游协议与目录里的 `Provider.meta.apiFormat` 是同一套词表；`model_fetch` 只对
/// Anthropic（`x-api-key`）和 Gemini（`/v1beta/models` + `x-goog-api-key`）特判，
/// 其余按 OpenAI 兼容的 `Bearer` + `/v1/models` 试。
fn api_format_for_protocol(protocol: &str) -> Option<&'static str> {
    match protocol {
        "anthropic" | "anthropic_messages" | "anthropic-messages" | "claude" | "messages" => {
            Some("anthropic-messages")
        }
        "gemini_native" | "gemini" | "google-generative-ai" => Some("google-generative-ai"),
        _ => None,
    }
}

/// 同一家上游可能同时挂 anthropic / openai_chat 两端点，优先用 OpenAI 系要列表。
fn protocol_rank(protocol: &str) -> u8 {
    match protocol {
        "openai_chat" => 0,
        "openai_responses" => 1,
        "openai" => 2,
        "anthropic" | "anthropic_messages" | "anthropic-messages" | "claude" | "messages" => 3,
        "gemini_native" | "gemini" | "google-generative-ai" => 4,
        _ => 5,
    }
}

/// 这家上游启用的端点 + 每条端点能取到的第一个明文密钥。
fn endpoint_rows(db: &Database, upstream_id: &str) -> Result<Vec<EndpointRow>, AppError> {
    let conn = crate::database::lock_conn!(db.conn);
    let mut stmt = conn.prepare(
        "SELECT e.id,
                e.protocol,
                e.base_url,
                COALESCE((
                    SELECT c.secret FROM upstream_credentials c
                    WHERE c.endpoint_id = e.id
                      AND COALESCE(c.secret_kind, 'none') = 'literal'
                      AND c.secret IS NOT NULL
                      AND c.secret <> ''
                    ORDER BY c.created_at
                    LIMIT 1
                ), '')
         FROM upstream_endpoints e
         WHERE e.upstream_id = ?1 AND COALESCE(e.enabled, 1) <> 0",
    )?;
    let rows = stmt.query_map([upstream_id], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
        ))
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

/// 这家上游绑定的托管 Codex OAuth 账号（provider meta 的 `authBinding` 是唯一真源）。
fn codex_oauth_account(db: &Database, upstream_id: &str) -> Result<Option<String>, AppError> {
    let conn = crate::database::lock_conn!(db.conn);
    let mut stmt = conn.prepare(
        "SELECT json_extract(p.meta, '$.authBinding.accountId')
         FROM provider_bindings b
         JOIN upstream_endpoints e ON e.id = b.endpoint_id
         JOIN providers p ON p.app_type = b.app_type AND p.id = b.provider_id
         WHERE e.upstream_id = ?1
           AND json_extract(p.meta, '$.authBinding.source') = 'managed_account'
           AND json_extract(p.meta, '$.authBinding.authProvider') = 'codex_oauth'
         ORDER BY json_extract(p.meta, '$.authBinding.accountId')
         LIMIT 1",
    )?;
    let account: Option<Option<String>> = stmt
        .query_row([upstream_id], |row| row.get::<_, Option<String>>(0))
        .optional()?;
    Ok(account.flatten().filter(|id| !id.trim().is_empty()))
}

/// 拿不到模型列表时给用户一句能行动的话：是没端点、只有托管登录、还是只有环境变量引用。
fn unsupported_reason(db: &Database, upstream_id: &str) -> Result<String, AppError> {
    let (managed_providers, kinds): (Vec<String>, Vec<String>) = {
        let conn = crate::database::lock_conn!(db.conn);
        let mut stmt = conn.prepare(
            "SELECT DISTINCT json_extract(p.meta, '$.authBinding.authProvider')
             FROM provider_bindings b
             JOIN upstream_endpoints e ON e.id = b.endpoint_id
             JOIN providers p ON p.app_type = b.app_type AND p.id = b.provider_id
             WHERE e.upstream_id = ?1
               AND json_extract(p.meta, '$.authBinding.source') = 'managed_account'
               AND json_extract(p.meta, '$.authBinding.authProvider') IS NOT NULL",
        )?;
        let managed_providers = stmt
            .query_map([upstream_id], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        let mut stmt = conn.prepare(
            "SELECT DISTINCT COALESCE(c.secret_kind, 'none')
             FROM upstream_credentials c
             JOIN upstream_endpoints e ON e.id = c.endpoint_id
             WHERE e.upstream_id = ?1 AND COALESCE(c.secret_kind, 'none') <> 'none'",
        )?;
        let kinds = stmt
            .query_map([upstream_id], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        (managed_providers, kinds)
    };
    if !managed_providers.is_empty() {
        return Ok(format!(
            "这家上游用的是托管登录（{}），暂不支持从上游获取模型列表",
            managed_providers.join(" / ")
        ));
    }
    if kinds.iter().any(|kind| kind == "env_ref") {
        return Ok("这家上游的密钥是环境变量引用，无法从上游获取模型列表".to_string());
    }
    Ok("这家上游没有可用的明文密钥或托管账号，无法从上游获取模型列表".to_string())
}

/// 找这家上游能用来拉模型列表的来源：优先明文密钥，其次托管 Codex 账号。
///
/// 明文优先是因为它适用范围最广（OpenAI / Anthropic / Gemini 三种协议都能取）；
/// 只有完全没有明文密钥时才退到 Codex 托管账号——那条路会真的去刷新 OAuth token。
pub fn resolve_source(db: &Database, upstream_id: &str) -> Result<FetchSource, AppError> {
    let rows = endpoint_rows(db, upstream_id)?;
    if rows.is_empty() {
        return Err(AppError::InvalidInput(
            "这家上游没有可用端点（可能已停用），无法从上游获取模型列表".to_string(),
        ));
    }
    // 稳定挑一个：先看协议偏好，再按端点 id，避免同一个库两次刷新选中不同端点。
    let chosen = rows
        .iter()
        .filter(|row| !row.3.is_empty() && !row.2.trim().is_empty())
        .min_by_key(|row| (protocol_rank(&row.1), row.0.clone()));
    if let Some(chosen) = chosen {
        let base_url = chosen.2.trim().to_string();
        return Ok(FetchSource::Literal(FetchTarget {
            is_full_url: looks_like_endpoint_url(&base_url),
            base_url,
            secret: chosen.3.clone(),
            api_format: api_format_for_protocol(&chosen.1),
        }));
    }
    if let Some(account_id) = codex_oauth_account(db, upstream_id)? {
        return Ok(FetchSource::CodexOAuth { account_id });
    }
    Err(AppError::InvalidInput(unsupported_reason(db, upstream_id)?))
}

/// 一次刷新里「一家真实上游」的计划：同一家上游只发一次请求，结果写进它对应的每一行。
///
/// 供应商页上「供应商就是上游，app 是下游消费者」。老库里同一个上游会按 app 派生成多行
/// （Codex 订阅同时有 `up-chatgpt.com` 与官方托管上游行），如果按行各拉一遍，就是对着同一家
/// 上游重复请求，还会把同一份模型在两个 app 里各存一套。这里按真实上游身份归并。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefreshPlan {
    /// 真实上游的身份：同一家 = 只发一次请求。
    pub service: String,
    pub source: FetchSource,
    /// 这家上游在库里对应的 upstream 行（至少一行）。
    pub upstream_ids: Vec<String>,
}

/// 把一组 upstream 行归并成「按真实上游去重」的刷新计划；解析不了的行走 errors 返回。
pub fn plan_refresh(
    db: &Database,
    upstream_ids: &[String],
) -> Result<(Vec<RefreshPlan>, Vec<String>), AppError> {
    let mut plans: Vec<RefreshPlan> = Vec::new();
    let mut errors: Vec<String> = Vec::new();
    for upstream_id in upstream_ids {
        let source = match resolve_source(db, upstream_id) {
            Ok(source) => source,
            Err(error) => {
                errors.push(error.to_string());
                continue;
            }
        };
        let service = match &source {
            // 同一个 baseURL = 同一家上游；密钥不同也只算一家。
            FetchSource::Literal(target) => {
                format!("literal:{}", target.base_url.trim().trim_end_matches('/'))
            }
            FetchSource::CodexOAuth { account_id } => format!("codex_oauth:{account_id}"),
        };
        match plans.iter_mut().find(|plan| plan.service == service) {
            Some(plan) => plan.upstream_ids.push(upstream_id.clone()),
            None => plans.push(RefreshPlan {
                service,
                source,
                upstream_ids: vec![upstream_id.clone()],
            }),
        }
    }
    Ok((plans, errors))
}

/// 清掉并重写这家上游的发现结果，返回 `(清洗后的模型数, 新增的候选模型 id)`。
///
/// 「新增」= 这次才出现、且不是 provider 行已派生的模型；它们各自写一条 `hidden = true`
/// 覆盖。已经存在的候选与用户改过状态的模型一律不动——刷新只增不减语义上的启用状态。
pub fn store_discovered_models(
    db: &Database,
    upstream_id: &str,
    models: &[String],
) -> Result<(usize, Vec<String>), AppError> {
    let cleaned: Vec<String> = models
        .iter()
        .map(|model| model.trim().to_string())
        .filter(|model| !model.is_empty() && !model.chars().any(|ch| ch.is_control()))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();

    // 派生出来的模型（provider 行里已有）不算候选：它们本来就在目录里，状态由用户管。
    let derived: BTreeSet<String> = crate::services::gateway_catalog::derive_entries(db)?
        .into_iter()
        .filter(|entry| entry.upstream_id == upstream_id)
        .map(|entry| entry.model_id)
        .collect();
    let existing: BTreeSet<String> = {
        let conn = crate::database::lock_conn!(db.conn);
        let mut stmt =
            conn.prepare("SELECT model_id FROM gateway_upstream_models WHERE upstream_id = ?1")?;
        let rows = stmt.query_map([upstream_id], |row| row.get::<_, String>(0))?;
        rows.collect::<Result<BTreeSet<_>, _>>()?
    };
    let overrides: BTreeMap<String, bool> = db
        .list_model_overrides()?
        .into_iter()
        .filter(|over| over.upstream_id == upstream_id)
        .map(|over| (over.model_id, over.hidden))
        .collect();

    let now = chrono::Utc::now().timestamp();
    {
        let conn = crate::database::lock_conn!(db.conn);
        conn.execute(
            "DELETE FROM gateway_upstream_models WHERE upstream_id = ?1",
            [upstream_id],
        )?;
        let mut stmt = conn.prepare(
            "INSERT INTO gateway_upstream_models (upstream_id, model_id, display_name, fetched_at)
             VALUES (?1, ?2, NULL, ?3)",
        )?;
        for model in &cleaned {
            stmt.execute(rusqlite::params![upstream_id, model, now])?;
        }
    }

    let mut added: Vec<String> = Vec::new();
    for model in &cleaned {
        if derived.contains(model) || existing.contains(model) || overrides.contains_key(model) {
            continue;
        }
        db.set_model_override(upstream_id, model, None, true)?;
        added.push(model.clone());
    }
    // 发现表本身不在覆盖表的写路径上，这里补一次失效，否则弹窗刷新后读到的还是旧目录。
    crate::services::gateway_catalog::invalidate_catalog_cache();
    Ok((cleaned.len(), added))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protocol_maps_to_the_fetch_api_format() {
        assert_eq!(
            api_format_for_protocol("anthropic"),
            Some("anthropic-messages")
        );
        assert_eq!(
            api_format_for_protocol("gemini_native"),
            Some("google-generative-ai")
        );
        assert_eq!(api_format_for_protocol("openai_chat"), None);
        assert_eq!(api_format_for_protocol("openai_responses"), None);
    }

    fn temp_sync_db(tag: &str) -> (std::path::PathBuf, Database) {
        let path = std::env::temp_dir().join(format!(
            "ccs-model-sync-{tag}-{}-{}.db",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        let db = Database::open_and_migrate_for_test(&path).expect("open");
        (path, db)
    }

    fn insert_endpoint(
        db: &Database,
        id: &str,
        protocol: &str,
        base_url: &str,
        enabled: bool,
        secret: Option<(&str, &str)>,
    ) -> Result<(), AppError> {
        let conn = crate::database::lock_conn!(db.conn);
        conn.execute(
            "INSERT INTO upstream_endpoints (id, upstream_id, protocol, base_url, enabled, created_at)
             VALUES (?1, 'up-1', ?2, ?3, ?4, 0)",
            rusqlite::params![id, protocol, base_url, enabled as i64],
        )
        .expect("endpoint");
        if let Some((secret, kind)) = secret {
            conn.execute(
                "INSERT INTO upstream_credentials
                     (id, endpoint_id, app_type, provider_id, secret, secret_kind, created_at)
                 VALUES (?1, ?2, 'claude', 'p-1', ?3, ?4, 0)",
                rusqlite::params![format!("cred-{id}"), id, secret, kind],
            )
            .expect("credential");
        }
        Ok(())
    }

    /// 同一个 Codex 订阅在库里的两行：claude-desktop 的 chatgpt.com 行 + codex 官方行。
    fn seed_two_codex_upstream_rows(db: &Database) -> Result<(), AppError> {
        let conn = crate::database::lock_conn!(db.conn);
        conn.execute(
            "INSERT INTO upstreams (id, name, host, created_at)
             VALUES ('up-a', 'Codex', 'chatgpt.com', 0), ('up-b', 'OpenAI Official', '', 0)",
            [],
        )?;
        conn.execute(
            "INSERT INTO upstream_endpoints
                 (id, upstream_id, protocol, base_url, enabled, created_at)
             VALUES ('ep-a', 'up-a', 'openai_responses', 'https://chatgpt.com/backend-api/codex', 1, 0),
                    ('ep-b', 'up-b', 'openai_responses', '', 1, 0)",
            [],
        )?;
        Ok(())
    }

    /// 绑一条托管账号的 provider：`meta.authBinding` 是 Codex 账号的唯一真源。
    fn bind_managed_provider(
        db: &Database,
        provider_id: &str,
        endpoint_id: &str,
        auth_provider: &str,
        account_id: &str,
    ) -> Result<(), AppError> {
        let conn = crate::database::lock_conn!(db.conn);
        let meta = serde_json::json!({
            "authBinding": {
                "source": "managed_account",
                "authProvider": auth_provider,
                "accountId": account_id,
            }
        })
        .to_string();
        conn.execute(
            "INSERT INTO providers
                 (id, app_type, name, settings_config, meta, category, created_at, sort_index)
             VALUES (?1, 'codex', 'OpenAI Official', '{\"auth\":{}}', ?2, 'official', 0, 0)",
            rusqlite::params![provider_id, meta],
        )
        .expect("provider");
        conn.execute(
            "INSERT INTO provider_bindings
                 (app_type, provider_id, endpoint_id, is_current, in_failover_queue, sort_index, model_map, created_at)
             VALUES ('codex', ?1, ?2, 1, 0, 0, '', 0)",
            rusqlite::params![provider_id, endpoint_id],
        )
        .expect("binding");
        Ok(())
    }

    /// 测试里绝大多数用例只关心明文路径，单独包一层免得每处都 match。
    fn literal_target(db: &Database, upstream_id: &str) -> FetchTarget {
        match resolve_source(db, upstream_id).expect("source") {
            FetchSource::Literal(target) => target,
            other => panic!("期望明文密钥路径，实际：{other:?}"),
        }
    }

    fn seed_upstream(db: &Database) -> Result<(), AppError> {
        let conn = crate::database::lock_conn!(db.conn);
        conn.execute(
            "INSERT INTO upstreams (id, name, host, created_at)
             VALUES ('up-1', 'CTYun', 'ctyun.example', 0)",
            [],
        )
        .expect("upstream");
        Ok(())
    }

    #[test]
    fn fetch_target_prefers_the_openai_compatible_endpoint() {
        let (path, db) = temp_sync_db("prefer-openai");
        seed_upstream(&db).expect("seed");
        insert_endpoint(
            &db,
            "ep-anthropic",
            "anthropic",
            "https://ctyun.example/anthropic",
            true,
            Some(("sk-anthropic", "literal")),
        )
        .expect("endpoint");
        insert_endpoint(
            &db,
            "ep-openai",
            "openai_chat",
            "https://ctyun.example/v1",
            true,
            Some(("sk-openai", "literal")),
        )
        .expect("endpoint");
        let target = literal_target(&db, "up-1");
        assert_eq!(target.base_url, "https://ctyun.example/v1");
        assert_eq!(target.secret, "sk-openai");
        assert_eq!(target.api_format, None);
        assert!(!target.is_full_url, "根路径不该按整条请求地址处理");
        drop(db);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn fetch_target_falls_back_to_anthropic_and_skips_disabled_endpoints() {
        let (path, db) = temp_sync_db("anthropic-only");
        seed_upstream(&db).expect("seed");
        insert_endpoint(
            &db,
            "ep-openai",
            "openai_chat",
            "https://ctyun.example/v1",
            false,
            Some(("sk-openai", "literal")),
        )
        .expect("endpoint");
        insert_endpoint(
            &db,
            "ep-anthropic",
            "anthropic",
            "https://ctyun.example/anthropic",
            true,
            Some(("sk-anthropic", "literal")),
        )
        .expect("endpoint");
        let target = literal_target(&db, "up-1");
        assert_eq!(target.base_url, "https://ctyun.example/anthropic");
        assert_eq!(target.api_format, Some("anthropic-messages"));
        drop(db);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn fetch_target_recognises_a_base_url_that_is_a_full_endpoint() {
        let (path, db) = temp_sync_db("full-url");
        seed_upstream(&db).expect("seed");
        insert_endpoint(
            &db,
            "ep-full",
            "openai_chat",
            "https://ctyun.example/v1/chat/completions",
            true,
            Some(("sk-openai", "literal")),
        )
        .expect("endpoint");
        let target = literal_target(&db, "up-1");
        assert!(target.is_full_url, "整条请求地址要按 full URL 处理");
        drop(db);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn endpoint_url_detection_only_matches_known_terminals() {
        assert!(looks_like_endpoint_url(
            "https://x.example/v1/chat/completions"
        ));
        assert!(looks_like_endpoint_url("https://x.example/v1/messages/"));
        assert!(looks_like_endpoint_url("https://x.example/v1beta/models"));
        assert!(!looks_like_endpoint_url("https://x.example/v1"));
        assert!(!looks_like_endpoint_url("https://x.example/anthropic"));
        assert!(!looks_like_endpoint_url("https://x.example"));
    }

    #[test]
    fn fetch_target_refuses_without_a_literal_secret() {
        let (path, db) = temp_sync_db("no-secret");
        seed_upstream(&db).expect("seed");
        insert_endpoint(
            &db,
            "ep-env",
            "openai_chat",
            "https://ctyun.example/v1",
            true,
            Some(("env:MY_KEY", "env_ref")),
        )
        .expect("endpoint");
        let error = resolve_source(&db, "up-1").expect_err("没有明文密钥应该报错");
        assert!(
            error.to_string().contains("环境变量引用"),
            "错误要说清楚为什么拿不到，实际：{error}"
        );
        drop(db);
        let _ = std::fs::remove_file(&path);
    }

    /// 手动验收：对着**真实库副本 + 真实托管账号**跑一遍，打印上游真正返回的模型。
    ///
    /// 需要环境变量：`CCS_VERIFY_DB`（库副本）、`CCS_VERIFY_UPSTREAM`（上游 id），
    /// 托管账号还要 `CCS_VERIFY_OAUTH_DIR`（`codex_oauth_auth.json` 所在目录）。
    /// 运行：`CCS_VERIFY_DB=... CCS_VERIFY_UPSTREAM=... cargo test --lib manual_fetch -- --ignored --nocapture`
    #[test]
    #[ignore = "手动验收：需要真实库副本与真实凭证"]
    fn manual_fetch_against_real_db_copy() {
        let db_path = std::env::var("CCS_VERIFY_DB").expect("CCS_VERIFY_DB");
        let upstream_id = std::env::var("CCS_VERIFY_UPSTREAM").expect("CCS_VERIFY_UPSTREAM");
        let db = Database::open_and_migrate_for_test(std::path::Path::new(&db_path))
            .expect("打开库副本失败");
        let source = resolve_source(&db, &upstream_id).expect("解析来源失败");
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime");
        let models = runtime.block_on(async {
            match source {
                FetchSource::Literal(target) => {
                    println!("来源：明文密钥 → {}/v1/models", target.base_url);
                    crate::services::model_fetch::fetch_models(
                        &target.base_url,
                        &target.secret,
                        target.is_full_url,
                        None,
                        None,
                        target.api_format,
                        None,
                    )
                    .await
                    .expect("拉取失败")
                }
                FetchSource::CodexOAuth { account_id } => {
                    println!("来源：托管 Codex 账号 {account_id}");
                    let dir = std::env::var("CCS_VERIFY_OAUTH_DIR").expect("CCS_VERIFY_OAUTH_DIR");
                    let manager = crate::proxy::providers::codex_oauth_auth::CodexOAuthManager::new(
                        std::path::PathBuf::from(dir),
                    );
                    let token = manager
                        .get_valid_token_for_account(&account_id)
                        .await
                        .expect("token");
                    let chatgpt_account_id = manager
                        .chatgpt_account_id_for_account(&account_id)
                        .await
                        .expect("workspace");
                    crate::services::codex_oauth_models::fetch_models_with_token(
                        &token,
                        &chatgpt_account_id,
                    )
                    .await
                    .expect("拉取失败")
                }
            }
        });
        println!("上游返回 {} 个模型：", models.len());
        for model in &models {
            println!("  {}", model.id);
        }
        assert!(!models.is_empty(), "上游没有返回任何模型");

        // 想在库副本上验证「刷新之后目录里到底能看到几个」时加 CCS_VERIFY_STORE=1。
        if std::env::var("CCS_VERIFY_STORE").as_deref() == Ok("1") {
            let ids: Vec<String> = models.into_iter().map(|model| model.id).collect();
            let (fetched, added) =
                store_discovered_models(&db, &upstream_id, &ids).expect("写发现结果失败");
            let visible = crate::services::gateway_catalog::catalog_view(&db)
                .expect("目录")
                .into_iter()
                .filter(|entry| entry.upstream_id == upstream_id)
                .count();
            let with_hidden = crate::services::gateway_catalog::catalog_view_including_hidden(&db)
                .expect("目录")
                .into_iter()
                .filter(|entry| entry.upstream_id == upstream_id)
                .count();
            println!(
                "写库：fetched={fetched} added={}（新增默认未启用）",
                added.len()
            );
            println!("目录：可见 {visible} 条，含未启用 {with_hidden} 条");
        }
    }

    #[test]
    fn refresh_requests_each_real_upstream_only_once() {
        // Codex 订阅在库里被按 app 派生成两行：claude-desktop 的 chatgpt.com 行与
        // codex 官方行。它们绑同一个托管账号 = 同一家上游，刷新只能请求一次，
        // 再把结果写进这两行；否则就是对着同一家上游拉两遍、各存一套。
        let (path, db) = temp_sync_db("plan-collapse");
        seed_two_codex_upstream_rows(&db).expect("seed rows");
        bind_managed_provider(&db, "codex-desktop", "ep-a", "codex_oauth", "acct-42")
            .expect("provider a");
        bind_managed_provider(&db, "codex-official", "ep-b", "codex_oauth", "acct-42")
            .expect("provider b");

        let ids = vec!["up-a".to_string(), "up-b".to_string()];
        let (plans, errors) = plan_refresh(&db, &ids).expect("plan");
        assert!(errors.is_empty(), "不该有解析失败：{errors:?}");
        assert_eq!(plans.len(), 1, "同一家上游只能有一个请求计划");
        assert_eq!(plans[0].service, "codex_oauth:acct-42");
        assert_eq!(
            plans[0].upstream_ids,
            vec!["up-a".to_string(), "up-b".to_string()],
            "结果要写回这两行，否则另一个 app 的目录里没有这些模型"
        );
        drop(db);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn managed_codex_account_is_used_when_the_upstream_has_no_literal_key() {
        let (path, db) = temp_sync_db("managed-codex");
        seed_upstream(&db).expect("seed");
        // 官方 / 托管登录的端点和自定义网关一样有行，但没有明文密钥。
        insert_endpoint(
            &db,
            "ep-managed",
            "openai_responses",
            "",
            true,
            Some(("", "managed")),
        )
        .expect("endpoint");
        bind_managed_provider(
            &db,
            "codex-official",
            "ep-managed",
            "codex_oauth",
            "acct-42",
        )
        .expect("provider");

        let source = resolve_source(&db, "up-1").expect("source");
        assert_eq!(
            source,
            FetchSource::CodexOAuth {
                account_id: "acct-42".to_string()
            }
        );
        drop(db);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn managed_accounts_other_than_codex_report_which_login_blocked_the_fetch() {
        let (path, db) = temp_sync_db("managed-kimi");
        seed_upstream(&db).expect("seed");
        insert_endpoint(&db, "ep-managed", "openai_chat", "", true, None).expect("endpoint");
        bind_managed_provider(&db, "kimi-code", "ep-managed", "kimi_oauth", "acct-7")
            .expect("provider");

        let error = resolve_source(&db, "up-1").expect_err("kimi 托管登录还不支持");
        let message = error.to_string();
        assert!(
            message.contains("kimi_oauth"),
            "错误要指出是哪种托管登录，实际：{message}"
        );
        drop(db);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn fetch_target_reports_an_upstream_without_endpoints() {
        let (path, db) = temp_sync_db("no-endpoint");
        seed_upstream(&db).expect("seed");
        let error = resolve_source(&db, "up-1").expect_err("没有端点应该报错");
        assert!(error.to_string().contains("端点"), "实际：{error}");
        drop(db);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn protocols_are_ranked_openai_first() {
        assert!(protocol_rank("openai_chat") < protocol_rank("openai_responses"));
        assert!(protocol_rank("openai_responses") < protocol_rank("anthropic"));
        assert!(protocol_rank("anthropic") < protocol_rank("gemini_native"));
        assert!(protocol_rank("gemini_native") < protocol_rank("weird"));
    }
}
