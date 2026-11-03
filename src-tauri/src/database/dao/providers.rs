use crate::database::{lock_conn, Database};
use crate::error::AppError;
use crate::provider::{Provider, ProviderMeta};
use indexmap::IndexMap;
use rusqlite::{params, OptionalExtension};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};

/// provider 到网关级上游的绑定结果。
///
/// 熔断需要两个粒度：**凭证级**（`circuit_key`，管 401/429/配额）和
/// **host 级**（`host`，管 DNS/连接/TLS/5xx/超时）。
/// 改写端点地址的结果。
#[derive(Debug, Default)]
pub struct EndpointRewrite {
    /// 被改写的 provider 数
    pub changed: usize,
    /// 受影响的 app（去重）
    pub apps: Vec<String>,
    /// 改不了的 `app:provider`（URL 不在配置里，或配置不可解析）
    pub skipped: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct UpstreamBinding {
    /// `up:{upstream_id}:{protocol}:{密钥指纹}`
    pub circuit_key: String,
    /// 上游 host；无显式 URL（官方托管 / 本地未填）时为空串，此时不参与 host 级熔断
    pub host: String,
    /// 端点主键
    pub endpoint_id: String,
    /// 端点协议（anthropic / openai_chat / ...）
    pub protocol: String,
    /// 端点 base_url，已归一化（去尾斜杠、保留 query）
    pub base_url: String,
    /// 该接入的模型映射（JSON 文本，可能为空）
    pub model_map: Option<String>,
}

/// 鉴权字段名判定：`env` / `options` 里命中这些后缀的才参与指纹计算。
fn is_credential_field(name: &str) -> bool {
    let upper = name.to_ascii_uppercase();
    ["TOKEN", "API_KEY", "APIKEY", "AUTH", "KEY"]
        .iter()
        .any(|suffix| upper.ends_with(suffix))
}

/// 由 provider 的 `settings_config` 计算密钥指纹。
///
/// 为什么需要：同一个上游下不同 provider 可能用**不同**的 key（实测 `api.kimi.com`
/// 有 2 把、`www.omnai.xyz` 有 3 把）。若熔断键只看 `(上游, 协议)`，一把 key 配额耗尽
/// 会把用另一把 key 的 agent 一起熔断——属于误伤。
///
/// 只输出指纹：不落库、不打印、不可逆。
/// 收集配置里所有凭证字段的**值**（不含字段名）。
///
/// 同一把 key 在不同 app 下可能分别叫 `ANTHROPIC_AUTH_TOKEN` / `OPENAI_API_KEY` /
/// `options.apiKey`，它们是同一个凭证，不该因为字段名不同而被拆开。
pub(crate) fn credential_values(config: &serde_json::Value) -> Vec<String> {
    let mut parts: Vec<String> = Vec::new();
    for scope in ["env", "options"] {
        let Some(object) = config.get(scope).and_then(|value| value.as_object()) else {
            continue;
        };
        for (name, value) in object {
            if !is_credential_field(name) {
                continue;
            }
            if let Some(text) = value.as_str() {
                let text = text.trim();
                if !text.is_empty() {
                    // 只收「值」不收字段名：同一把 key 在不同 app 里可能分别存在
                    // `ANTHROPIC_AUTH_TOKEN` / `OPENAI_API_KEY` / `options.apiKey`，
                    // 它们是同一个凭证，不该因为字段名不同被拆成两个熔断器。
                    parts.push(text.to_string());
                }
            }
        }
    }
    for name in ["apiKey", "api_key"] {
        if let Some(text) = config.get(name).and_then(|value| value.as_str()) {
            let text = text.trim();
            if !text.is_empty() {
                parts.push(text.to_string());
            }
        }
    }
    parts
}

/// 由 provider 的 `settings_config` 计算密钥指纹。**只输出指纹：不落库、不打印、不可逆。**
fn credential_fingerprint(config: &serde_json::Value) -> String {
    let mut parts = credential_values(config);
    if parts.is_empty() {
        // 本地推理服务（LM Studio 等）通常无鉴权；同 host 同协议本就该归一组
        return "nokey".to_string();
    }
    parts.sort();
    let digest = Sha256::digest(parts.join("\u{1f}").as_bytes());
    digest[..4].iter().map(|b| format!("{b:02x}")).collect()
}

/// 搬进 `upstream_credentials.secret` 用的主密钥。
///
/// 一个 provider 理论上只有一把；多把时取排序后第一把（与指纹的排序口径一致），
/// 保证同一配置永远得到同一把，迁移可重跑。
pub(crate) fn primary_credential_value(config: &serde_json::Value) -> Option<String> {
    let mut parts = credential_values(config);
    parts.sort();
    parts.into_iter().next()
}

type OmoProviderRow = (
    String,
    String,
    String,
    Option<String>,
    Option<i64>,
    Option<usize>,
    Option<String>,
    String,
);

impl Database {
    /// Detach Codex providers bound to these deleted local accounts, atomically.
    /// Only this device's account IDs are passed in: provider rows sync across
    /// devices, and a row bound to another device's account must stay bound.
    /// Keep raw JSON so unrelated and future metadata survives the update.
    pub(crate) fn unbind_codex_managed_accounts(
        &self,
        account_ids: &[String],
    ) -> Result<Vec<String>, AppError> {
        if account_ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut conn = lock_conn!(self.conn);
        let tx = conn
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|e| AppError::Database(e.to_string()))?;
        let rows = {
            let mut stmt = tx
                .prepare(
                    "SELECT id, meta, settings_config, category,
                        trim(json_extract(meta, '$.authBinding.accountId')) FROM providers
                 WHERE app_type = 'codex'
                   AND json_extract(meta, '$.authBinding.source') = 'managed_account'
                   AND json_extract(meta, '$.authBinding.authProvider') = 'codex_oauth'
                 ORDER BY id",
                )
                .map_err(|e| AppError::Database(e.to_string()))?;
            let rows = stmt
                .query_map([], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, Option<String>>(3)?,
                        row.get::<_, Option<String>>(4)?,
                    ))
                })
                .map_err(|e| AppError::Database(e.to_string()))?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|e| AppError::Database(e.to_string()))?
        };
        let mut affected = Vec::new();
        for (id, meta_text, settings_text, category, bound) in rows {
            if !bound.is_some_and(|bound| account_ids.iter().any(|id| id.trim() == bound)) {
                continue;
            }
            let parse_error = |error: serde_json::Error| {
                AppError::Database(format!("供应商 {id} 配置无效: {error}"))
            };
            let mut meta: serde_json::Value =
                serde_json::from_str(&meta_text).map_err(parse_error)?;
            let mut settings: serde_json::Value =
                serde_json::from_str(&settings_text).map_err(parse_error)?;
            let mut provider = Provider::with_id(id.clone(), String::new(), settings.clone(), None);
            provider.category = category;
            provider.meta = Some(serde_json::from_value(meta.clone()).map_err(parse_error)?);
            if provider.category.is_none()
                && crate::proxy::providers::is_codex_official_provider(&provider)
            {
                provider.category = Some("official".to_string());
            }
            meta.as_object_mut()
                .ok_or_else(|| AppError::Config(format!("供应商 {id} 的 meta 必须是对象")))?
                .remove("authBinding");
            settings
                .as_object_mut()
                .ok_or_else(|| {
                    AppError::Config(format!("供应商 {id} 的 settings_config 必须是对象"))
                })?
                .insert("auth".to_string(), serde_json::json!({}));
            tx.execute(
                "UPDATE providers SET meta = ?1, settings_config = ?2, category = ?3
                 WHERE app_type = 'codex' AND id = ?4",
                params![
                    meta.to_string(),
                    settings.to_string(),
                    provider.category,
                    id
                ],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
            affected.push(id);
        }
        tx.commit().map_err(|e| AppError::Database(e.to_string()))?;
        Ok(affected)
    }

    pub fn get_all_providers(
        &self,
        app_type: &str,
    ) -> Result<IndexMap<String, Provider>, AppError> {
        let conn = lock_conn!(self.conn);
        let mut stmt = conn.prepare(
            "SELECT id, name, settings_config, website_url, category, created_at, sort_index, notes, icon, icon_color, meta, in_failover_queue
             FROM providers WHERE app_type = ?1
             ORDER BY COALESCE(sort_index, 999999), created_at ASC, id ASC"
        ).map_err(|e| AppError::Database(e.to_string()))?;

        let provider_iter = stmt
            .query_map(params![app_type], |row| {
                let id: String = row.get(0)?;
                let name: String = row.get(1)?;
                let settings_config_str: String = row.get(2)?;
                let website_url: Option<String> = row.get(3)?;
                let category: Option<String> = row.get(4)?;
                let created_at: Option<i64> = row.get(5)?;
                let sort_index: Option<usize> = row.get(6)?;
                let notes: Option<String> = row.get(7)?;
                let icon: Option<String> = row.get(8)?;
                let icon_color: Option<String> = row.get(9)?;
                let meta_str: String = row.get(10)?;
                let in_failover_queue: bool = row.get(11)?;

                let settings_config =
                    serde_json::from_str(&settings_config_str).unwrap_or(serde_json::Value::Null);
                let meta: ProviderMeta = serde_json::from_str(&meta_str).unwrap_or_default();

                Ok((
                    id,
                    Provider {
                        id: "".to_string(), // Placeholder, set below
                        name,
                        settings_config,
                        website_url,
                        category,
                        created_at,
                        sort_index,
                        notes,
                        meta: Some(meta),
                        icon,
                        icon_color,
                        in_failover_queue,
                    },
                ))
            })
            .map_err(|e| AppError::Database(e.to_string()))?;

        let mut providers = IndexMap::new();
        for provider_res in provider_iter {
            let (id, mut provider) = provider_res.map_err(|e| AppError::Database(e.to_string()))?;
            provider.id = id.clone();

            let mut stmt_endpoints = conn.prepare(
                "SELECT url, added_at FROM provider_endpoints WHERE provider_id = ?1 AND app_type = ?2 ORDER BY added_at ASC, url ASC"
            ).map_err(|e| AppError::Database(e.to_string()))?;

            let endpoints_iter = stmt_endpoints
                .query_map(params![id, app_type], |row| {
                    let url: String = row.get(0)?;
                    let added_at: Option<i64> = row.get(1)?;
                    Ok((
                        url,
                        crate::settings::CustomEndpoint {
                            url: "".to_string(),
                            added_at: added_at.unwrap_or(0),
                            last_used: None,
                        },
                    ))
                })
                .map_err(|e| AppError::Database(e.to_string()))?;

            let mut custom_endpoints = HashMap::new();
            for ep_res in endpoints_iter {
                let (url, mut ep) = ep_res.map_err(|e| AppError::Database(e.to_string()))?;
                ep.url = url.clone();
                custom_endpoints.insert(url, ep);
            }

            if let Some(meta) = &mut provider.meta {
                meta.custom_endpoints = custom_endpoints;
            }

            providers.insert(id, provider);
        }

        Ok(providers)
    }

    pub fn get_current_provider(&self, app_type: &str) -> Result<Option<String>, AppError> {
        let conn = lock_conn!(self.conn);
        let mut stmt = conn
            .prepare("SELECT id FROM providers WHERE app_type = ?1 AND is_current = 1 LIMIT 1")
            .map_err(|e| AppError::Database(e.to_string()))?;

        let mut rows = stmt
            .query(params![app_type])
            .map_err(|e| AppError::Database(e.to_string()))?;

        if let Some(row) = rows.next().map_err(|e| AppError::Database(e.to_string()))? {
            Ok(Some(
                row.get(0).map_err(|e| AppError::Database(e.to_string()))?,
            ))
        } else {
            Ok(None)
        }
    }

    pub fn get_provider_by_id(
        &self,
        id: &str,
        app_type: &str,
    ) -> Result<Option<Provider>, AppError> {
        let conn = lock_conn!(self.conn);
        let result = conn.query_row(
            "SELECT name, settings_config, website_url, category, created_at, sort_index, notes, icon, icon_color, meta, in_failover_queue
             FROM providers WHERE id = ?1 AND app_type = ?2",
            params![id, app_type],
            |row| {
                let name: String = row.get(0)?;
                let settings_config_str: String = row.get(1)?;
                let website_url: Option<String> = row.get(2)?;
                let category: Option<String> = row.get(3)?;
                let created_at: Option<i64> = row.get(4)?;
                let sort_index: Option<usize> = row.get(5)?;
                let notes: Option<String> = row.get(6)?;
                let icon: Option<String> = row.get(7)?;
                let icon_color: Option<String> = row.get(8)?;
                let meta_str: String = row.get(9)?;
                let in_failover_queue: bool = row.get(10)?;

                let settings_config = serde_json::from_str(&settings_config_str).unwrap_or(serde_json::Value::Null);
                let meta: ProviderMeta = serde_json::from_str(&meta_str).unwrap_or_default();

                Ok(Provider {
                    id: id.to_string(),
                    name,
                    settings_config,
                    website_url,
                    category,
                    created_at,
                    sort_index,
                    notes,
                    meta: Some(meta),
                    icon,
                    icon_color,
                    in_failover_queue,
                })
            },
        );

        match result {
            Ok(provider) => Ok(Some(provider)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AppError::Database(e.to_string())),
        }
    }

    /// 只改排序：一个事务里按 id 更新 `sort_index`，不存在的行跳过。不经整行快照保存，
    /// 和删除、编辑并发时既不会复活删掉的行，也不会盖掉别处刚存的字段。
    pub fn update_provider_sort_indexes(
        &self,
        app_type: &str,
        updates: &[(String, usize)],
    ) -> Result<(), AppError> {
        let mut conn = lock_conn!(self.conn);
        let tx = conn
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|e| AppError::Database(e.to_string()))?;
        for (id, sort_index) in updates {
            tx.execute(
                "UPDATE providers SET sort_index = ?1 WHERE id = ?2 AND app_type = ?3",
                params![sort_index, id, app_type],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        }
        tx.commit().map_err(|e| AppError::Database(e.to_string()))
    }

    pub fn save_provider(&self, app_type: &str, provider: &Provider) -> Result<(), AppError> {
        let mut conn = lock_conn!(self.conn);
        let tx = conn
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|e| AppError::Database(e.to_string()))?;

        let mut meta_clone = provider.meta.clone().unwrap_or_default();
        let endpoints = std::mem::take(&mut meta_clone.custom_endpoints);

        let existing: Option<(bool, bool)> = tx
            .query_row(
                "SELECT is_current, in_failover_queue FROM providers WHERE id = ?1 AND app_type = ?2",
                params![provider.id, app_type],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .ok();

        let is_update = existing.is_some();
        let (is_current, in_failover_queue) =
            existing.unwrap_or((false, provider.in_failover_queue));

        if is_update {
            tx.execute(
                "UPDATE providers SET
                    name = ?1,
                    settings_config = ?2,
                    website_url = ?3,
                    category = ?4,
                    created_at = ?5,
                    sort_index = ?6,
                    notes = ?7,
                    icon = ?8,
                    icon_color = ?9,
                    meta = ?10,
                    is_current = ?11,
                    in_failover_queue = ?12
                WHERE id = ?13 AND app_type = ?14",
                params![
                    provider.name,
                    serde_json::to_string(&provider.settings_config).map_err(|e| {
                        AppError::Database(format!("Failed to serialize settings_config: {e}"))
                    })?,
                    provider.website_url,
                    provider.category,
                    provider.created_at,
                    provider.sort_index,
                    provider.notes,
                    provider.icon,
                    provider.icon_color,
                    serde_json::to_string(&meta_clone).map_err(|e| AppError::Database(format!(
                        "Failed to serialize meta: {e}"
                    )))?,
                    is_current,
                    in_failover_queue,
                    provider.id,
                    app_type,
                ],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        } else {
            tx.execute(
                "INSERT INTO providers (
                    id, app_type, name, settings_config, website_url, category,
                    created_at, sort_index, notes, icon, icon_color, meta, is_current, in_failover_queue
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
                params![
                    provider.id,
                    app_type,
                    provider.name,
                    serde_json::to_string(&provider.settings_config)
                        .map_err(|e| AppError::Database(format!("Failed to serialize settings_config: {e}")))?,
                    provider.website_url,
                    provider.category,
                    provider.created_at,
                    provider.sort_index,
                    provider.notes,
                    provider.icon,
                    provider.icon_color,
                    serde_json::to_string(&meta_clone)
                        .map_err(|e| AppError::Database(format!("Failed to serialize meta: {e}")))?,
                    is_current,
                    in_failover_queue,
                ],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

            for (url, endpoint) in endpoints {
                tx.execute(
                    "INSERT INTO provider_endpoints (provider_id, app_type, url, added_at)
                     VALUES (?1, ?2, ?3, ?4)",
                    params![provider.id, app_type, url, endpoint.added_at],
                )
                .map_err(|e| AppError::Database(e.to_string()))?;
            }
        }

        tx.commit().map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    /// Replace a provider row under a new ID without exposing an intermediate
    /// duplicate or missing row. Existing endpoint and health references move
    /// with the provider, while its current-state bit is preserved.
    pub fn replace_provider_id(
        &self,
        app_type: &str,
        original_id: &str,
        provider: &Provider,
    ) -> Result<(), AppError> {
        if original_id == provider.id {
            return self.save_provider(app_type, provider);
        }

        let mut conn = lock_conn!(self.conn);
        let tx = conn
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|e| AppError::Database(e.to_string()))?;

        let (is_current, in_failover_queue) = tx
            .query_row(
                "SELECT is_current, in_failover_queue FROM providers WHERE id = ?1 AND app_type = ?2",
                params![original_id, app_type],
                |row| Ok((row.get::<_, bool>(0)?, row.get::<_, bool>(1)?)),
            )
            .optional()
            .map_err(|e| AppError::Database(e.to_string()))?
            .ok_or_else(|| {
                AppError::Database(format!(
                    "Provider '{original_id}' does not exist in app '{app_type}'"
                ))
            })?;

        let target_exists = tx
            .query_row(
                "SELECT 1 FROM providers WHERE id = ?1 AND app_type = ?2",
                params![provider.id, app_type],
                |_| Ok(()),
            )
            .optional()
            .map_err(|e| AppError::Database(e.to_string()))?
            .is_some();
        if target_exists {
            return Err(AppError::Database(format!(
                "Provider '{}' already exists in app '{app_type}'",
                provider.id
            )));
        }

        let mut meta = provider.meta.clone().unwrap_or_default();
        meta.custom_endpoints.clear();
        tx.execute(
            "INSERT INTO providers (
                id, app_type, name, settings_config, website_url, category,
                created_at, sort_index, notes, icon, icon_color, meta,
                is_current, in_failover_queue
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                provider.id,
                app_type,
                provider.name,
                serde_json::to_string(&provider.settings_config).map_err(|e| {
                    AppError::Database(format!("Failed to serialize settings_config: {e}"))
                })?,
                provider.website_url,
                provider.category,
                provider.created_at,
                provider.sort_index,
                provider.notes,
                provider.icon,
                provider.icon_color,
                serde_json::to_string(&meta).map_err(|e| {
                    AppError::Database(format!("Failed to serialize meta: {e}"))
                })?,
                is_current,
                in_failover_queue,
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        tx.execute(
            "UPDATE provider_endpoints SET provider_id = ?1 WHERE provider_id = ?2 AND app_type = ?3",
            params![provider.id, original_id, app_type],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        tx.execute(
            "UPDATE provider_health SET provider_id = ?1 WHERE provider_id = ?2 AND app_type = ?3",
            params![provider.id, original_id, app_type],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        tx.execute(
            "DELETE FROM providers WHERE id = ?1 AND app_type = ?2",
            params![original_id, app_type],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        tx.commit().map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    pub fn delete_provider(&self, app_type: &str, id: &str) -> Result<(), AppError> {
        let mut conn = lock_conn!(self.conn);
        // 绑定要跟着删：否则 provider_bindings 留下孤儿行，`up-provider-{id}`
        // 这类上游也会永久堆积（读时对账只补不删，补不到这里）。
        let tx = conn
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|e| AppError::Database(e.to_string()))?;
        tx.execute(
            "DELETE FROM providers WHERE id = ?1 AND app_type = ?2",
            params![id, app_type],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        if Database::table_exists(&tx, "provider_bindings")? {
            tx.execute(
                "DELETE FROM provider_bindings WHERE provider_id = ?1 AND app_type = ?2",
                params![id, app_type],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
            Database::cleanup_orphan_upstreams(&tx)?;
        }
        tx.commit().map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    pub fn set_current_provider(&self, app_type: &str, id: &str) -> Result<(), AppError> {
        let mut conn = lock_conn!(self.conn);
        let tx = conn
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|e| AppError::Database(e.to_string()))?;

        tx.execute(
            "UPDATE providers SET is_current = 0 WHERE app_type = ?1",
            params![app_type],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        tx.execute(
            "UPDATE providers SET is_current = 1 WHERE id = ?1 AND app_type = ?2",
            params![id, app_type],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        tx.commit().map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    pub fn update_provider_settings_config(
        &self,
        app_type: &str,
        provider_id: &str,
        settings_config: &serde_json::Value,
    ) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        conn.execute(
            "UPDATE providers SET settings_config = ?1 WHERE id = ?2 AND app_type = ?3",
            params![
                serde_json::to_string(settings_config).map_err(|e| AppError::Database(format!(
                    "Failed to serialize settings_config: {e}"
                )))?,
                provider_id,
                app_type
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    pub fn add_custom_endpoint(
        &self,
        app_type: &str,
        provider_id: &str,
        url: &str,
    ) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        let added_at = chrono::Utc::now().timestamp_millis();
        conn.execute(
            "INSERT INTO provider_endpoints (provider_id, app_type, url, added_at) VALUES (?1, ?2, ?3, ?4)",
            params![provider_id, app_type, url, added_at],
        ).map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    pub fn remove_custom_endpoint(
        &self,
        app_type: &str,
        provider_id: &str,
        url: &str,
    ) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        conn.execute(
            "DELETE FROM provider_endpoints WHERE provider_id = ?1 AND app_type = ?2 AND url = ?3",
            params![provider_id, app_type, url],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    pub fn set_omo_provider_current(
        &self,
        app_type: &str,
        provider_id: &str,
        category: &str,
    ) -> Result<(), AppError> {
        let mut conn = lock_conn!(self.conn);
        let tx = conn
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|e| AppError::Database(e.to_string()))?;
        tx.execute(
            "UPDATE providers SET is_current = 0 WHERE app_type = ?1 AND category = ?2",
            params![app_type, category],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        // OMO ↔ OMO Slim mutually exclusive: deactivate the opposite category
        let opposite = match category {
            "omo" => Some("omo-slim"),
            "omo-slim" => Some("omo"),
            _ => None,
        };
        if let Some(opp) = opposite {
            tx.execute(
                "UPDATE providers SET is_current = 0 WHERE app_type = ?1 AND category = ?2",
                params![app_type, opp],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        }
        let updated = tx
            .execute(
                "UPDATE providers SET is_current = 1 WHERE id = ?1 AND app_type = ?2 AND category = ?3",
                params![provider_id, app_type, category],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        if updated != 1 {
            return Err(AppError::Database(format!(
                "Failed to set {category} provider current: provider '{provider_id}' not found in app '{app_type}'"
            )));
        }
        tx.commit().map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    pub fn is_omo_provider_current(
        &self,
        app_type: &str,
        provider_id: &str,
        category: &str,
    ) -> Result<bool, AppError> {
        let conn = lock_conn!(self.conn);
        match conn.query_row(
            "SELECT is_current FROM providers
             WHERE id = ?1 AND app_type = ?2 AND category = ?3",
            params![provider_id, app_type, category],
            |row| row.get(0),
        ) {
            Ok(is_current) => Ok(is_current),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(false),
            Err(e) => Err(AppError::Database(e.to_string())),
        }
    }

    pub fn clear_omo_provider_current(
        &self,
        app_type: &str,
        provider_id: &str,
        category: &str,
    ) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        conn.execute(
            "UPDATE providers SET is_current = 0
             WHERE id = ?1 AND app_type = ?2 AND category = ?3",
            params![provider_id, app_type, category],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    pub fn get_current_omo_provider(
        &self,
        app_type: &str,
        category: &str,
    ) -> Result<Option<Provider>, AppError> {
        let conn = lock_conn!(self.conn);
        let row_data: Result<OmoProviderRow, rusqlite::Error> = conn.query_row(
            "SELECT id, name, settings_config, category, created_at, sort_index, notes, meta
             FROM providers
             WHERE app_type = ?1 AND category = ?2 AND is_current = 1
             LIMIT 1",
            params![app_type, category],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                ))
            },
        );

        let (id, name, settings_config_str, _row_category, created_at, sort_index, notes, meta_str) =
            match row_data {
                Ok(v) => v,
                Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(None),
                Err(e) => return Err(AppError::Database(e.to_string())),
            };

        let settings_config = serde_json::from_str(&settings_config_str).map_err(|e| {
            AppError::Database(format!(
                "Failed to parse {category} provider settings_config (provider_id={id}): {e}"
            ))
        })?;
        let meta: crate::provider::ProviderMeta = if meta_str.trim().is_empty() {
            crate::provider::ProviderMeta::default()
        } else {
            serde_json::from_str(&meta_str).map_err(|e| {
                AppError::Database(format!(
                    "Failed to parse {category} provider meta (provider_id={id}): {e}"
                ))
            })?
        };

        Ok(Some(Provider {
            id,
            name,
            settings_config,
            website_url: None,
            category: Some(category.to_string()),
            created_at,
            sort_index,
            notes,
            meta: Some(meta),
            icon: None,
            icon_color: None,
            in_failover_queue: false,
        }))
    }

    /// 判断 providers 表是否为空（全 app_type 一起算）。
    ///
    /// 用于区分"全新安装"和"升级用户"：在启动流程 import/seed 之前调用。
    /// 使用 `EXISTS` 短路查询，比 `COUNT(*)` 在将来表变大时更高效。
    pub fn is_providers_empty(&self) -> Result<bool, AppError> {
        let conn = lock_conn!(self.conn);
        let exists: bool = conn
            .query_row("SELECT EXISTS(SELECT 1 FROM providers)", [], |row| {
                row.get(0)
            })
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(!exists)
    }

    /// 仅获取指定 app 下所有 provider 的 id 集合。
    ///
    /// 比 `get_all_providers` 轻量得多：只读 id 列、无 endpoint 子查询。
    /// 用于只需要做存在性检查的场景（如 additive 模式的 live 同步去重）。
    pub fn get_provider_ids(&self, app_type: &str) -> Result<HashSet<String>, AppError> {
        let conn = lock_conn!(self.conn);
        let mut stmt = conn
            .prepare("SELECT id FROM providers WHERE app_type = ?1")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let rows = stmt
            .query_map(params![app_type], |row| row.get::<_, String>(0))
            .map_err(|e| AppError::Database(e.to_string()))?;
        let mut ids = HashSet::new();
        for row in rows {
            ids.insert(row.map_err(|e| AppError::Database(e.to_string()))?);
        }
        Ok(ids)
    }

    /// 按 app 取「provider_id → 网关级熔断键」映射。
    ///
    /// 键格式 `up:{upstream_id}:{protocol}:{密钥指纹}`，三段缺一不可：
    ///
    /// - `upstream_id`：同一网关在不同 app 下只算一个上游（改造的核心目的）；
    /// - `protocol`：实测同一 host 会分裂成 anthropic / openai_chat / openai_responses
    ///   多个端点，一条路径坏掉不代表其他路径坏掉；
    /// - **密钥指纹**：同一上游下的 provider 可能用不同的 key（实测 `api.kimi.com`
    ///   有 2 把、`www.omnai.xyz` 有 3 把）。少了这一段，一把 key 配额耗尽会把
    ///   用另一把 key 的 agent 一起熔断——那是误伤，不是共享。
    ///
    /// 没有绑定的 provider（迁移后新建、或迁移被跳过）不出现在返回值里，调用方回落到旧键。
    /// 把「provider 行 → 上游绑定」对齐到最新状态。
    ///
    /// 为什么不挂在 `save_provider` 上：直接写 `providers` 表的 SQL 有二十多处
    /// （导入、恢复备份、启动同步、故障转移开关……），漏掉任何一处就会重演
    /// 「`dsh` 那行没有绑定」的问题。挂在读路径上则天然覆盖所有写入方。
    ///
    /// 幂等且只在真有漂移时写库；无论调用多少次，结果都与整库迁移一致。
    /// 改一个网关端点，所有使用它的 app 一起跟着变。
    ///
    /// 这是「配置归网关、选择归 app」的第一次写操作：用户在上游改一次地址，
    /// 底下每个 app 的 `settings_config` 都被改写，下次派生自然指向新端点。
    ///
    /// 返回值里 `skipped` 不是错误——有些 provider 的 URL 根本不是配置里的字符串
    /// （托管 OAuth 由适配器按身份算），那些行必须原样保留并如实报回去。
    pub(crate) fn rewrite_endpoint_base_url(
        conn: &mut rusqlite::Connection,
        endpoint_id: &str,
        new_url: &str,
    ) -> Result<EndpointRewrite, AppError> {
        Self::rewrite_endpoint_configs(conn, endpoint_id, |config| {
            Database::p1_set_base_url(config, new_url)
        })
    }

    /// 改某个接入的模型映射（`app + provider` 粒度，不是网关粒度）。
    ///
    /// 模型映射归"这个 app 怎么用这条接入"：同一个网关，claude 里可以映射成
    /// kimi-k2，claude-desktop 里映射成别的。所以写的是该 app 自己的配置，
    /// 不动同端点的其它 app。
    ///
    /// 返回 false 表示这个 provider 的配置里没有可写的 `env`（例如 opencode）。
    pub(crate) fn update_binding_model_map(
        conn: &mut rusqlite::Connection,
        app_type: &str,
        provider_id: &str,
        map: &serde_json::Value,
    ) -> Result<bool, AppError> {
        let config_text: Option<String> = conn
            .query_row(
                "SELECT settings_config FROM providers WHERE app_type = ?1 AND id = ?2",
                params![app_type, provider_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| AppError::Database(e.to_string()))?;
        let Some(config_text) = config_text else {
            return Ok(false);
        };
        let mut config: serde_json::Value =
            serde_json::from_str(&config_text).unwrap_or(serde_json::Value::Null);
        if !Database::p1_set_model_map(&mut config, map) {
            return Ok(false);
        }
        conn.execute(
            "UPDATE providers SET settings_config = ?1 WHERE app_type = ?2 AND id = ?3",
            params![config.to_string(), app_type, provider_id],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(true)
    }

    /// 换一把密钥，所有使用这个端点的 app 一起换。
    ///
    /// 只改 `settings_config` 里的凭证值；`upstream_credentials` 由随后的对账
    /// 从配置重新派生，指纹跟着变——新 key 天然拿到一套干净的熔断状态。
    ///
    /// 返回的 `skipped` 里会有 `env_ref` 的 app：它配置里只有环境变量名，没有明文可换。
    pub(crate) fn rewrite_endpoint_secret(
        conn: &mut rusqlite::Connection,
        endpoint_id: &str,
        new_secret: &str,
    ) -> Result<EndpointRewrite, AppError> {
        Self::rewrite_endpoint_configs(conn, endpoint_id, |config| {
            Database::p1_set_secret(config, new_secret)
        })
    }

    /// 改地址与改密钥共用同一套「找接入 → 改配置 → 提交」的骨架。
    fn rewrite_endpoint_configs(
        conn: &mut rusqlite::Connection,
        endpoint_id: &str,
        apply: impl Fn(&mut serde_json::Value) -> bool,
    ) -> Result<EndpointRewrite, AppError> {
        let targets: Vec<(String, String)> = {
            let mut stmt = conn
                .prepare(
                    "SELECT app_type, provider_id FROM provider_bindings
                     WHERE endpoint_id = ?1 ORDER BY app_type, provider_id",
                )
                .map_err(|e| AppError::Database(e.to_string()))?;
            let mapped = stmt
                .query_map(params![endpoint_id], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })
                .map_err(|e| AppError::Database(e.to_string()))?;
            mapped
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| AppError::Database(e.to_string()))?
        };

        let mut outcome = EndpointRewrite::default();
        if targets.is_empty() {
            return Ok(outcome);
        }

        let tx = conn
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|e| AppError::Database(e.to_string()))?;

        for (app_type, provider_id) in &targets {
            let label = format!("{app_type}:{provider_id}");
            let config_text: Option<String> = tx
                .query_row(
                    "SELECT settings_config FROM providers WHERE app_type = ?1 AND id = ?2",
                    params![app_type, provider_id],
                    |row| row.get(0),
                )
                .optional()
                .map_err(|e| AppError::Database(e.to_string()))?;
            let Some(config_text) = config_text else {
                outcome.skipped.push(label);
                continue;
            };
            let mut config: serde_json::Value =
                serde_json::from_str(&config_text).unwrap_or(serde_json::Value::Null);
            if !apply(&mut config) {
                outcome.skipped.push(label);
                continue;
            }
            tx.execute(
                "UPDATE providers SET settings_config = ?1 WHERE app_type = ?2 AND id = ?3",
                params![config.to_string(), app_type, provider_id],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
            outcome.changed += 1;
            if !outcome.apps.contains(app_type) {
                outcome.apps.push(app_type.clone());
            }
        }

        tx.commit().map_err(|e| AppError::Database(e.to_string()))?;
        Ok(outcome)
    }

    pub(crate) fn reconcile_provider_bindings(
        conn: &mut rusqlite::Connection,
        app_type: &str,
    ) -> Result<(), AppError> {
        let rows = Database::p1_read_rows(conn, Some(app_type))?;
        if rows.is_empty() {
            return Ok(());
        }

        // 已存绑定：provider_id → (endpoint_id, model_map)
        let stored: HashMap<String, (String, String)> = {
            let mut stmt = conn
                .prepare(
                    "SELECT provider_id, endpoint_id, COALESCE(model_map, '')
                     FROM provider_bindings WHERE app_type = ?1",
                )
                .map_err(|e| AppError::Database(e.to_string()))?;
            let mapped = stmt
                .query_map(params![app_type], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        (row.get::<_, String>(1)?, row.get::<_, String>(2)?),
                    ))
                })
                .map_err(|e| AppError::Database(e.to_string()))?;
            mapped
                .collect::<Result<HashMap<_, _>, _>>()
                .map_err(|e| AppError::Database(e.to_string()))?
        };

        // 已存密钥：provider_id → (是否已有值, 形态)。
        // 密钥单向真源要求「配置里能解出、库里还没有」也算漂移，否则老库升级后
        // 永远补不上密钥（绑定没错，就没理由重搬）。
        let stored_secret: HashMap<String, (bool, String)> = {
            let mut stmt = conn
                .prepare(
                    "SELECT provider_id, COALESCE(secret, '') <> '', COALESCE(secret_kind, 'none')
                     FROM upstream_credentials WHERE app_type = ?1",
                )
                .map_err(|e| AppError::Database(e.to_string()))?;
            let mapped = stmt
                .query_map(params![app_type], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        (row.get::<_, bool>(1)?, row.get::<_, String>(2)?),
                    ))
                })
                .map_err(|e| AppError::Database(e.to_string()))?;
            mapped
                .collect::<Result<HashMap<_, _>, _>>()
                .map_err(|e| AppError::Database(e.to_string()))?
        };

        // 漂移 = 以下任一：
        //   1. 没有绑定，或端点与当前配置派生不一致（换了 base_url）
        //   2. 能解出密钥但库里没有（老库升级后必须补得上）
        //   3. 凭证形态变了（literal / env_ref / managed / none）
        //   4. 绑定的模型映射与配置派生不一致
        let mut drifted = Vec::new();
        for row in &rows {
            let lift = Database::p1_derive(row);
            let stored_binding = stored.get(&row.id);
            let endpoint_drifted = stored_binding.map(|(endpoint, _)| endpoint.as_str())
                != Some(lift.endpoint_id.as_str());
            let stored_secret_entry = stored_secret.get(&row.id);
            let secret_missing = lift.secret.is_some()
                && !stored_secret_entry.map(|(has, _)| *has).unwrap_or(false);
            let kind_drifted = stored_secret_entry
                .map(|(_, kind)| kind.as_str() != lift.secret_kind)
                .unwrap_or(false);
            let model_map_drifted = lift.model_map.is_some()
                && stored_binding
                    .map(|(_, model)| model.as_str() != lift.model_map.as_deref().unwrap_or(""))
                    .unwrap_or(true);
            if endpoint_drifted || secret_missing || kind_drifted || model_map_drifted {
                drifted.push((row, lift));
            }
        }
        if drifted.is_empty() {
            return Ok(());
        }

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|elapsed| elapsed.as_millis() as i64)
            .unwrap_or(0);
        // 规范名跨 app 统一：否则增量补出来的上游名会跟整库迁移不一致。
        // 整库扫一遍只发生在「有漂移」时，代价可以忽略。
        let canonical = Database::p1_canonical_names(conn)?;

        let tx = conn
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|e| AppError::Database(e.to_string()))?;
        for (row, lift) in &drifted {
            Database::lift_provider_row(&tx, row, lift, &canonical, now)?;
        }
        Database::cleanup_orphan_upstreams(&tx)?;
        tx.commit().map_err(|e| AppError::Database(e.to_string()))?;

        log::debug!(
            "[{app_type}] 上游绑定对账：补/换 {} 条",
            drifted.len()
        );
        Ok(())
    }

    pub fn get_provider_upstream_keys(
        &self,
        app_type: &str,
    ) -> Result<HashMap<String, UpstreamBinding>, AppError> {
        let mut conn = lock_conn!(self.conn);
        // 表不存在（未跑过 v25 迁移的库）时返回空映射，绝不因此让路由失败
        let has_bindings: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master
                               WHERE type = 'table' AND name = 'provider_bindings')",
                [],
                |row| row.get(0),
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        if !has_bindings {
            return Ok(HashMap::new());
        }

        // 读时对账：v25 迁移只跑一次，之后新建 / 改了 base_url 的 provider 全靠这里补齐。
        // 失败只记日志——对账坏了也不能让路由跟着坏（回落旧键即可）。
        if let Err(err) = Self::reconcile_provider_bindings(&mut conn, app_type) {
            log::warn!("[{app_type}] 上游绑定对账失败，本次沿用已有绑定: {err}");
        }

        let mut stmt = conn
            .prepare(
                "SELECT b.provider_id, e.upstream_id, e.protocol, u.host, p.settings_config,
                        e.id, e.base_url, b.model_map
                 FROM provider_bindings b
                 JOIN upstream_endpoints e ON e.id = b.endpoint_id
                 JOIN upstreams u ON u.id = e.upstream_id
                 LEFT JOIN providers p
                        ON p.id = b.provider_id AND p.app_type = b.app_type
                 WHERE b.app_type = ?1",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        let rows = stmt
            .query_map(params![app_type], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, Option<String>>(7)?,
                ))
            })
            .map_err(|e| AppError::Database(e.to_string()))?;

        let mut keys = HashMap::new();
        for row in rows {
            let (
                provider_id,
                upstream_id,
                protocol,
                host,
                settings_config,
                endpoint_id,
                base_url,
                model_map,
            ) = row.map_err(|e| AppError::Database(e.to_string()))?;
            let fingerprint = settings_config
                .as_deref()
                .and_then(|text| serde_json::from_str::<serde_json::Value>(text).ok())
                .map(|config| credential_fingerprint(&config))
                .unwrap_or_else(|| "nokey".to_string());
            keys.insert(
                provider_id,
                UpstreamBinding {
                    circuit_key: format!("up:{upstream_id}:{protocol}:{fingerprint}"),
                    host,
                    endpoint_id,
                    base_url,
                    protocol,
                    model_map,
                },
            );
        }
        Ok(keys)
    }

    /// 判断指定 app 下是否已存在任意 provider。
    ///
    /// 启动阶段的 live import 需要使用这个更严格的判断：
    /// 只要该 app 已经有任何 provider（包括官方 seed），就不应再自动导入 `default`。
    pub fn has_any_provider_for_app(&self, app_type: &str) -> Result<bool, AppError> {
        let conn = lock_conn!(self.conn);
        let exists: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM providers WHERE app_type = ?1)",
                params![app_type],
                |row| row.get(0),
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(exists)
    }

    /// 判断指定 app 下是否存在非官方种子的供应商。
    ///
    /// 比 `get_all_providers` 轻量得多：只读 id 列、无 endpoint 子查询、首条命中即返回。
    /// 用于 `import_default_config` 决定是否跳过 live 导入。
    pub fn has_non_official_seed_provider(&self, app_type: &str) -> Result<bool, AppError> {
        use crate::database::dao::providers_seed::is_official_seed_id;
        let conn = lock_conn!(self.conn);
        let mut stmt = conn
            .prepare("SELECT id FROM providers WHERE app_type = ?1")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let mut rows = stmt
            .query(params![app_type])
            .map_err(|e| AppError::Database(e.to_string()))?;
        while let Some(row) = rows.next().map_err(|e| AppError::Database(e.to_string()))? {
            let id: String = row.get(0).map_err(|e| AppError::Database(e.to_string()))?;
            if !is_official_seed_id(&id) {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// 计算指定 app 下一个可用的 sort_index（追加到末尾）。
    fn next_sort_index_for_app(&self, app_type: &str) -> Result<usize, AppError> {
        let conn = lock_conn!(self.conn);
        let max: Option<i64> = conn
            .query_row(
                "SELECT MAX(sort_index) FROM providers WHERE app_type = ?1",
                params![app_type],
                |row| row.get(0),
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(max.map(|v| (v + 1) as usize).unwrap_or(0))
    }

    /// 启动时调用：补齐缺失的官方预设供应商（Claude / Codex / Gemini）。
    ///
    /// 使用 settings flag `official_providers_seeded` 保证每个数据库只执行一次：
    /// - 全新用户：seed 三条官方预设
    /// - 老用户升级：同样会触发一次（flag 不存在），追加到末尾，不影响已有排序
    /// - 用户删除 seed 后：不再重建（flag 已为 true），尊重用户意图
    ///
    /// 与 `Database::save_provider` 的 UPSERT 语义配合，即使被意外重复调用
    /// 也不会覆盖用户当前激活的供应商（is_current 字段会被保留）。
    pub fn init_default_official_providers(&self) -> Result<usize, AppError> {
        use crate::database::dao::providers_seed::OFFICIAL_SEEDS;

        if self
            .get_bool_flag("official_providers_seeded")
            .unwrap_or(false)
        {
            return Ok(0);
        }

        let mut inserted = 0_usize;
        let now_ms = chrono::Utc::now().timestamp_millis();

        for seed in OFFICIAL_SEEDS {
            let app_type_str = seed.app_type.as_str();

            // 若该 id 已存在（极端情况：用户曾手动用过同 id），跳过
            if self.get_provider_by_id(seed.id, app_type_str)?.is_some() {
                continue;
            }

            let next_sort_index = self.next_sort_index_for_app(app_type_str)?;

            let settings_config: serde_json::Value =
                serde_json::from_str(seed.settings_config_json).map_err(|e| {
                    AppError::Database(format!("Seed JSON parse failed for {}: {e}", seed.id))
                })?;

            let mut provider = Provider::with_id(
                seed.id.to_string(),
                seed.name.to_string(),
                settings_config,
                Some(seed.website_url.to_string()),
            );
            provider.category = Some("official".to_string());
            provider.icon = Some(seed.icon.to_string());
            provider.icon_color = Some(seed.icon_color.to_string());
            provider.sort_index = Some(next_sort_index);
            provider.created_at = Some(now_ms);

            self.save_provider(app_type_str, &provider)?;
            inserted += 1;
            log::info!(
                "✓ Seeded official provider: {} ({})",
                seed.name,
                app_type_str
            );
        }

        // 即使 inserted=0（例如用户手动创建过同 id）也设置 flag 防止反复检查
        self.set_setting("official_providers_seeded", "true")?;

        Ok(inserted)
    }

    /// 按 id 兜底插入单条 official seed（仅当目标表中该 id 不存在时插入）。
    ///
    /// 与 `init_default_official_providers` 不同：
    /// - 不触碰 `official_providers_seeded` 全局 flag，是 on-demand 修复
    /// - 只处理一条 seed，由调用方决定 id + app_type
    /// - 已存在则尊重用户自定义，不覆盖
    ///
    /// 返回 Ok(true) 表示插入了新行，Ok(false) 表示已存在被跳过。
    pub fn ensure_official_seed_by_id(
        &self,
        seed_id: &str,
        app_type: crate::app_config::AppType,
    ) -> Result<bool, AppError> {
        use crate::database::dao::providers_seed::OFFICIAL_SEEDS;

        let seed = OFFICIAL_SEEDS
            .iter()
            .find(|s| s.id == seed_id && s.app_type == app_type)
            .ok_or_else(|| {
                AppError::Database(format!(
                    "unknown official seed: id={seed_id}, app_type={}",
                    app_type.as_str()
                ))
            })?;

        let app_type_str = seed.app_type.as_str();

        if self.get_provider_by_id(seed_id, app_type_str)?.is_some() {
            return Ok(false);
        }

        let settings_config: serde_json::Value = serde_json::from_str(seed.settings_config_json)
            .map_err(|e| {
                AppError::Database(format!("Seed JSON parse failed for {}: {e}", seed.id))
            })?;

        let next_sort_index = self.next_sort_index_for_app(app_type_str)?;
        let now_ms = chrono::Utc::now().timestamp_millis();

        let mut provider = Provider::with_id(
            seed.id.to_string(),
            seed.name.to_string(),
            settings_config,
            Some(seed.website_url.to_string()),
        );
        provider.category = Some("official".to_string());
        provider.icon = Some(seed.icon.to_string());
        provider.icon_color = Some(seed.icon_color.to_string());
        provider.sort_index = Some(next_sort_index);
        provider.created_at = Some(now_ms);

        self.save_provider(app_type_str, &provider)?;

        Ok(true)
    }
}

#[cfg(test)]
mod ensure_official_seed_tests {
    use crate::app_config::AppType;
    use crate::database::{
        Database, CLAUDE_DESKTOP_OFFICIAL_PROVIDER_ID, CODEX_OFFICIAL_PROVIDER_ID,
        GROKBUILD_OFFICIAL_PROVIDER_ID,
    };

    #[test]
    fn ensure_inserts_when_missing() {
        let db = Database::memory().expect("memory db");
        let inserted = db
            .ensure_official_seed_by_id(CLAUDE_DESKTOP_OFFICIAL_PROVIDER_ID, AppType::ClaudeDesktop)
            .expect("ensure ok");
        assert!(inserted, "should insert when missing");

        let provider = db
            .get_provider_by_id(
                CLAUDE_DESKTOP_OFFICIAL_PROVIDER_ID,
                AppType::ClaudeDesktop.as_str(),
            )
            .expect("query ok")
            .expect("provider exists after ensure");

        assert_eq!(provider.id, CLAUDE_DESKTOP_OFFICIAL_PROVIDER_ID);
        assert_eq!(provider.name, "Claude Desktop Official");
        assert_eq!(provider.category.as_deref(), Some("official"));
        assert_eq!(provider.icon.as_deref(), Some("anthropic"));
        assert_eq!(provider.icon_color.as_deref(), Some("#D4915D"));
    }

    #[test]
    fn ensure_skips_when_present_and_preserves_customization() {
        let db = Database::memory().expect("memory db");
        db.init_default_official_providers().expect("seed");

        let mut renamed = db
            .get_provider_by_id(
                CLAUDE_DESKTOP_OFFICIAL_PROVIDER_ID,
                AppType::ClaudeDesktop.as_str(),
            )
            .expect("query ok")
            .expect("seed present");
        renamed.name = "My Custom Backup".to_string();
        db.save_provider(AppType::ClaudeDesktop.as_str(), &renamed)
            .expect("save customization");

        let inserted = db
            .ensure_official_seed_by_id(CLAUDE_DESKTOP_OFFICIAL_PROVIDER_ID, AppType::ClaudeDesktop)
            .expect("ensure ok");
        assert!(!inserted, "should skip when present");

        let after = db
            .get_provider_by_id(
                CLAUDE_DESKTOP_OFFICIAL_PROVIDER_ID,
                AppType::ClaudeDesktop.as_str(),
            )
            .expect("query ok")
            .expect("still present");
        assert_eq!(
            after.name, "My Custom Backup",
            "customization must not be overwritten"
        );
    }

    #[test]
    fn ensure_recreates_codex_official_seed_after_deletion() {
        let db = Database::memory().expect("memory db");
        db.init_default_official_providers().expect("seed");
        db.delete_provider(AppType::Codex.as_str(), CODEX_OFFICIAL_PROVIDER_ID)
            .expect("delete Codex official");

        let inserted = db
            .ensure_official_seed_by_id(CODEX_OFFICIAL_PROVIDER_ID, AppType::Codex)
            .expect("ensure Codex official");
        assert!(inserted);
        let provider = db
            .get_provider_by_id(CODEX_OFFICIAL_PROVIDER_ID, AppType::Codex.as_str())
            .expect("query")
            .expect("Codex official restored");
        assert_eq!(provider.category.as_deref(), Some("official"));
        assert_eq!(provider.settings_config["auth"], serde_json::json!({}));
    }

    #[test]
    fn ensure_recreates_grokbuild_official_seed_after_deletion() {
        let db = Database::memory().expect("memory db");
        db.init_default_official_providers().expect("seed");
        db.delete_provider(AppType::GrokBuild.as_str(), GROKBUILD_OFFICIAL_PROVIDER_ID)
            .expect("delete Grok Build official");

        let inserted = db
            .ensure_official_seed_by_id(GROKBUILD_OFFICIAL_PROVIDER_ID, AppType::GrokBuild)
            .expect("ensure Grok Build official");
        assert!(inserted);
        let provider = db
            .get_provider_by_id(GROKBUILD_OFFICIAL_PROVIDER_ID, AppType::GrokBuild.as_str())
            .expect("query")
            .expect("Grok Build official restored");
        assert_eq!(provider.category.as_deref(), Some("official"));
        // 空 config：切换时不注入自定义模型表，Grok CLI 回落到自带 OAuth 登录
        assert_eq!(provider.settings_config["config"], serde_json::json!(""));
    }

    #[test]
    fn ensure_rejects_unknown_seed() {
        let db = Database::memory().expect("memory db");
        let result = db.ensure_official_seed_by_id("nonexistent-id", AppType::ClaudeDesktop);
        assert!(result.is_err(), "unknown seed id should be Err");
    }

    #[test]
    fn ensure_rejects_seed_app_type_mismatch() {
        let db = Database::memory().expect("memory db");
        let result =
            db.ensure_official_seed_by_id(CLAUDE_DESKTOP_OFFICIAL_PROVIDER_ID, AppType::Claude);
        assert!(result.is_err(), "(id, app_type) mismatch should be Err");
    }
}

/// 读时对账：迁移只跑一次，之后新建 / 改 base_url / 删除的 provider 靠这里与库对齐。
#[cfg(test)]
mod reconcile_binding_tests {
    use super::*;

    fn insert_provider(
        db: &Database,
        app_type: &str,
        id: &str,
        name: &str,
        config: &str,
    ) -> Result<(), AppError> {
        let conn = lock_conn!(db.conn);
        conn.execute(
            "INSERT INTO providers (id, app_type, name, settings_config, created_at, sort_index)
             VALUES (?1, ?2, ?3, ?4, 1, 0)",
            params![id, app_type, name, config],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    fn count(db: &Database, table: &str) -> Result<i64, AppError> {
        let conn = lock_conn!(db.conn);
        conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| row.get(0))
            .map_err(|e| AppError::Database(e.to_string()))
    }

    fn anthropic(name: &str, url: &str) -> String {
        format!(
            "{{\"env\":{{\"ANTHROPIC_BASE_URL\":\"{url}\"}},\"n\":\"{name}\"}}"
        )
    }

    /// 改一次网关地址，所有使用它的 app 都必须跟着变，且旧端点被清理。
    #[test]
    fn rewriting_endpoint_url_moves_all_consumers() -> Result<(), AppError> {
        let db = Database::memory()?;
        insert_provider(&db, "claude", "a", "CTYun", &anthropic("CTYun", "https://ai.ctaigw.cn/v1"))?;
        insert_provider(
            &db,
            "claude-desktop",
            "b",
            "CTYun",
            &anthropic("CTYun", "https://ai.ctaigw.cn/v1"),
        )?;
        db.get_provider_upstream_keys("claude")?;
        db.get_provider_upstream_keys("claude-desktop")?;

        let endpoint_id: String = {
            let conn = lock_conn!(db.conn);
            conn.query_row(
                "SELECT endpoint_id FROM provider_bindings
                 WHERE app_type = 'claude' AND provider_id = 'a'",
                [],
                |row| row.get(0),
            )
            .map_err(|e| AppError::Database(e.to_string()))?
        };

        let mut conn = lock_conn!(db.conn);
        let outcome = Database::rewrite_endpoint_base_url(
            &mut conn,
            &endpoint_id,
            "https://new.example.com/v1",
        )?;
        assert_eq!(outcome.changed, 2, "两个 app 都应被改写");
        assert_eq!(outcome.apps.len(), 2);
        assert!(outcome.skipped.is_empty());
        for app in &outcome.apps {
            Database::reconcile_provider_bindings(&mut conn, app)?;
        }
        drop(conn);

        // 两个 app 的 base_url 都变了
        let claude = db.get_provider_upstream_keys("claude")?;
        assert_eq!(claude["a"].base_url, "https://new.example.com/v1");
        assert_eq!(claude["a"].host, "new.example.com");
        let desktop = db.get_provider_upstream_keys("claude-desktop")?;
        assert_eq!(desktop["b"].base_url, "https://new.example.com/v1");

        // 旧端点已无绑定引用，应当被清掉
        let conn = lock_conn!(db.conn);
        let stale: i64 = conn.query_row(
            "SELECT COUNT(*) FROM upstream_endpoints WHERE base_url LIKE '%ai.ctaigw.cn%'",
            [],
            |row| row.get(0),
        )?;
        assert_eq!(stale, 0, "旧端点必须被清理");
        Ok(())
    }

    /// 换密钥：所有使用这个端点的 app 一起换，且熔断键跟着换。
    #[test]
    fn rewriting_endpoint_secret_updates_all_consumers() -> Result<(), AppError> {
        let db = Database::memory()?;
        let with_key = r#"{"env":{"ANTHROPIC_BASE_URL":"https://ai.ctaigw.cn/v1",
            "ANTHROPIC_AUTH_TOKEN":"sk-secret"}}"#;
        insert_provider(&db, "claude", "a", "CTYun", with_key)?;
        insert_provider(&db, "claude-desktop", "b", "CTYun", with_key)?;
        db.get_provider_upstream_keys("claude")?;
        db.get_provider_upstream_keys("claude-desktop")?;
        let before = db.get_provider_upstream_keys("claude")?;
        let old_key = before["a"].circuit_key.clone();

        let endpoint_id: String = {
            let conn = lock_conn!(db.conn);
            conn.query_row(
                "SELECT endpoint_id FROM provider_bindings
                 WHERE app_type = 'claude' AND provider_id = 'a'",
                [],
                |row| row.get(0),
            )
            .map_err(|e| AppError::Database(e.to_string()))?
        };

        let mut conn = lock_conn!(db.conn);
        let outcome =
            Database::rewrite_endpoint_secret(&mut conn, &endpoint_id, "sk-brand-new")?;
        assert_eq!(outcome.changed, 2, "两个 app 的密钥都要换");
        for app in &outcome.apps {
            Database::reconcile_provider_bindings(&mut conn, app)?;
        }
        drop(conn);

        let claude = db.get_provider_upstream_keys("claude")?;
        assert_eq!(claude["a"].base_url, "https://ai.ctaigw.cn/v1", "地址不该被改动");
        assert_ne!(
            claude["a"].circuit_key, old_key,
            "换密钥后熔断键必须变化，否则会沿用旧 key 的失败状态"
        );

        let stored: String = {
            let conn = lock_conn!(db.conn);
            conn.query_row(
                "SELECT settings_config FROM providers WHERE app_type='claude-desktop' AND id='b'",
                [],
                |row| row.get(0),
            )
            .map_err(|e| AppError::Database(e.to_string()))?
        };
        assert!(stored.contains("sk-brand-new"), "claude-desktop 也应换到新密钥");
        assert!(!stored.contains("sk-secret"), "旧密钥不能残留");
        Ok(())
    }

    /// 密钥只在环境变量里的接入（env_ref）换不了，必须如实 skipped。
    #[test]
    fn rewriting_secret_reports_env_ref_as_skipped() -> Result<(), AppError> {
        let db = Database::memory()?;
        insert_provider(
            &db,
            "hermes",
            "h",
            "Ctyun",
            r#"{"api":"https://ai.ctaigw.cn/v1","key_env":"CTYUN_API_KEY"}"#,
        )?;
        db.get_provider_upstream_keys("hermes")?;

        let endpoint_id: String = {
            let conn = lock_conn!(db.conn);
            conn.query_row(
                "SELECT endpoint_id FROM provider_bindings WHERE provider_id = 'h'",
                [],
                |row| row.get(0),
            )
            .map_err(|e| AppError::Database(e.to_string()))?
        };

        let mut conn = lock_conn!(db.conn);
        let outcome = Database::rewrite_endpoint_secret(&mut conn, &endpoint_id, "sk-x")?;
        assert_eq!(outcome.changed, 0);
        assert_eq!(outcome.skipped, vec!["hermes:h".to_string()]);
        Ok(())
    }

    /// 改写不认识的配置形状时必须如实报 skipped，不能假装成功。
    #[test]
    fn rewriting_unknown_shape_reports_skipped() -> Result<(), AppError> {
        let db = Database::memory()?;
        insert_provider(&db, "pi", "opaque", "Opaque", r#"{"something":"else"}"#)?;
        db.get_provider_upstream_keys("pi")?;

        let endpoint_id: String = {
            let conn = lock_conn!(db.conn);
            conn.query_row(
                "SELECT endpoint_id FROM provider_bindings WHERE provider_id = 'opaque'",
                [],
                |row| row.get(0),
            )
            .map_err(|e| AppError::Database(e.to_string()))?
        };

        let mut conn = lock_conn!(db.conn);
        let outcome =
            Database::rewrite_endpoint_base_url(&mut conn, &endpoint_id, "https://x.example.com")?;
        assert_eq!(outcome.changed, 0);
        assert_eq!(outcome.skipped, vec!["pi:opaque".to_string()]);
        Ok(())
    }

    /// 密钥单向真源：密钥要落进 `upstream_credentials.secret`；
    /// 解不出密钥的配置（托管 OAuth / 本地推理）不得把已存的值抹掉。
    #[test]
    fn credential_secret_is_backfilled_and_never_blanked() -> Result<(), AppError> {
        let db = Database::memory()?;
        insert_provider(
            &db,
            "claude",
            "p-key",
            "CTYun",
            r#"{"env":{"ANTHROPIC_BASE_URL":"https://ai.ctaigw.cn/v1","ANTHROPIC_AUTH_TOKEN":"sk-secret-a"}}"#,
        )?;
        insert_provider(&db, "claude", "p-managed", "Official", r#"{"env":{}}"#)?;

        db.get_provider_upstream_keys("claude")?;

        fn secret(db: &Database, provider_id: &str) -> Result<Option<String>, AppError> {
            let conn = lock_conn!(db.conn);
            conn.query_row(
                "SELECT secret FROM upstream_credentials
                 WHERE app_type = 'claude' AND provider_id = ?1",
                params![provider_id],
                |row| row.get::<_, Option<String>>(0),
            )
            .map_err(|e| AppError::Database(e.to_string()))
        }

        assert_eq!(secret(&db, "p-key")?.as_deref(), Some("sk-secret-a"));
        assert_eq!(secret(&db, "p-managed")?, None, "解不出密钥时应为 NULL");

        // 模拟以后接入托管账号：先塞一把 token，再对账，已存的不能被抹掉
        {
            let conn = lock_conn!(db.conn);
            conn.execute(
                "UPDATE upstream_credentials SET secret = 'managed-token'
                 WHERE provider_id = 'p-managed'",
                [],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        }
        db.get_provider_upstream_keys("claude")?;
        assert_eq!(
            secret(&db, "p-managed")?.as_deref(),
            Some("managed-token"),
            "重跑对账不得抹掉已存密钥"
        );
        Ok(())
    }

    #[test]
    fn backfills_provider_added_after_migration() -> Result<(), AppError> {
        let db = Database::memory()?;
        // 迁移之后才出现的 provider（启动同步 / 导入都会这样），没有任何绑定
        insert_provider(
            &db,
            "claude",
            "p-new",
            "CTYun",
            &anthropic("CTYun", "https://ai.ctaigw.cn/v1"),
        )?;
        assert_eq!(count(&db, "provider_bindings")?, 0);

        let keys = db.get_provider_upstream_keys("claude")?;
        let binding = keys.get("p-new").expect("缺绑定的 provider 必须被补齐");
        assert_eq!(binding.host, "ai.ctaigw.cn");
        assert!(
            binding
                .circuit_key
                .starts_with("up:up-ai.ctaigw.cn:anthropic:"),
            "熔断键应指向网关级上游，实际 {}",
            binding.circuit_key
        );
        assert_eq!(count(&db, "provider_bindings")?, 1);
        Ok(())
    }

    #[test]
    fn reconcile_is_idempotent_and_merges_same_host() -> Result<(), AppError> {
        let db = Database::memory()?;
        insert_provider(&db, "claude", "p1", "CTYun", &anthropic("CTYun", "https://ai.ctaigw.cn/v1"))?;
        insert_provider(&db, "claude", "p2", "Alias", &anthropic("Alias", "https://ai.ctaigw.cn/v1"))?;
        insert_provider(
            &db,
            "claude-desktop",
            "p3",
            "CTYun Desktop",
            &anthropic("CTYun Desktop", "https://ai.ctaigw.cn/v1"),
        )?;

        db.get_provider_upstream_keys("claude")?;
        let (bindings, endpoints, upstreams) = (
            count(&db, "provider_bindings")?,
            count(&db, "upstream_endpoints")?,
            count(&db, "upstreams")?,
        );
        assert_eq!((bindings, endpoints, upstreams), (2, 1, 1));

        // 重复读同一个 app 不能产生任何重复
        db.get_provider_upstream_keys("claude")?;
        assert_eq!(count(&db, "provider_bindings")?, bindings);
        assert_eq!(count(&db, "upstream_endpoints")?, endpoints);
        assert_eq!(count(&db, "upstreams")?, upstreams);

        // 换到另一个 app 读：只多一条绑定，上游与端点仍然只有一个
        db.get_provider_upstream_keys("claude-desktop")?;
        assert_eq!(count(&db, "provider_bindings")?, 3, "第三个 app 的那行也要补上");
        assert_eq!(count(&db, "upstream_endpoints")?, endpoints);
        assert_eq!(count(&db, "upstreams")?, upstreams, "同一网关只应有一个上游");
        Ok(())
    }

    #[test]
    fn base_url_change_moves_binding_instead_of_duplicating() -> Result<(), AppError> {
        let db = Database::memory()?;
        insert_provider(
            &db,
            "claude",
            "p1",
            "Old",
            &anthropic("Old", "https://old.example.com/v1"),
        )?;
        let before = db.get_provider_upstream_keys("claude")?;
        assert_eq!(before.get("p1").map(|b| b.host.as_str()), Some("old.example.com"));

        {
            let conn = lock_conn!(db.conn);
            conn.execute(
                "UPDATE providers SET settings_config = ?1 WHERE id = 'p1' AND app_type = 'claude'",
                params![anthropic("New", "https://new.example.com/v1")],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        }

        let after = db.get_provider_upstream_keys("claude")?;
        assert_eq!(
            after.get("p1").map(|b| b.host.as_str()),
            Some("new.example.com"),
            "改 base_url 后绑定必须跟着换"
        );
        assert_eq!(count(&db, "provider_bindings")?, 1, "换绑不是新增");
        assert_eq!(count(&db, "upstream_endpoints")?, 1, "旧端点应被清理");
        assert_eq!(count(&db, "upstreams")?, 1, "旧上游应被清理");
        assert_eq!(count(&db, "upstream_credentials")?, 1);
        Ok(())
    }

    #[test]
    fn delete_provider_removes_binding_and_orphans() -> Result<(), AppError> {
        let db = Database::memory()?;
        insert_provider(
            &db,
            "claude",
            "p1",
            "CTYun",
            &anthropic("CTYun", "https://ai.ctaigw.cn/v1"),
        )?;
        db.get_provider_upstream_keys("claude")?;
        assert_eq!(count(&db, "provider_bindings")?, 1);

        db.delete_provider("claude", "p1")?;

        assert_eq!(count(&db, "provider_bindings")?, 0, "绑定必须跟着删");
        assert_eq!(count(&db, "upstream_credentials")?, 0);
        assert_eq!(count(&db, "upstream_endpoints")?, 0);
        assert_eq!(count(&db, "upstreams")?, 0, "无人引用的上游必须清理");
        Ok(())
    }
}
