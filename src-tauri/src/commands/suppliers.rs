//! 供应商（网关级上游）总览。
//!
//! 这是「以供应商为一级公民」的读入口：前端不再从「某个 app 的 provider 列表」
//! 出发，而是从网关出发看——一家服务商有几个端点、几把凭据、被哪些 app 使用。
//!
//! 当前只读，不改任何既有写入路径；后续的供应商编辑 / 接入选择器都建立在这层之上。

use crate::database::lock_conn;
use crate::error::AppError;
use crate::store::AppState;
use serde::Serialize;
use std::collections::BTreeMap;

/// 一个端点，以及它在各 agent 侧被谁使用。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SupplierEndpoint {
    pub id: String,
    pub protocol: String,
    pub base_url: String,
    /// 使用该端点的 app（去重、有序）
    pub apps: Vec<String>,
    /// `app:providerId` 明细，方便前端跳回原配置
    pub consumers: Vec<String>,
    /// 挂在这个端点上的凭据数
    pub credential_count: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Supplier {
    pub id: String,
    pub name: String,
    pub host: String,
    pub endpoints: Vec<SupplierEndpoint>,
    pub app_count: i64,
    /// 该供应商下出现过的凭证形态：literal / env_ref / managed / none
    pub credential_kinds: Vec<String>,
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

    let mut suppliers: BTreeMap<String, Supplier> = BTreeMap::new();
    {
        let mut stmt = conn.prepare(
            "SELECT id, name, host FROM upstreams
             ORDER BY name COLLATE NOCASE, id",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(Supplier {
                id: row.get(0)?,
                name: row.get(1)?,
                host: row.get(2)?,
                endpoints: Vec::new(),
                app_count: 0,
                credential_kinds: Vec::new(),
            })
        })?;
        for row in rows {
            let supplier = row?;
            suppliers.insert(supplier.id.clone(), supplier);
        }
    }

    // endpoint_id → 该端点上的使用情况
    let mut consumers: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
    {
        let mut stmt = conn.prepare(
            "SELECT endpoint_id, app_type, provider_id FROM provider_bindings
             ORDER BY endpoint_id, app_type, provider_id",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?;
        for row in rows {
            let (endpoint_id, app_type, provider_id) = row?;
            consumers
                .entry(endpoint_id)
                .or_default()
                .push((app_type, provider_id));
        }
    }

    // 每个端点的凭据数 + 形态
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

    {
        let mut stmt = conn.prepare(
            "SELECT id, upstream_id, protocol, base_url FROM upstream_endpoints
             ORDER BY upstream_id, protocol, base_url",
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
            let (endpoint_id, upstream_id, protocol, base_url) = row?;
            let Some(supplier) = suppliers.get_mut(&upstream_id) else {
                continue;
            };

            let mut apps: Vec<String> = Vec::new();
            let mut details: Vec<String> = Vec::new();
            for (app_type, provider_id) in consumers.get(&endpoint_id).into_iter().flatten() {
                if !apps.contains(app_type) {
                    apps.push(app_type.clone());
                }
                details.push(format!("{app_type}:{provider_id}"));
            }

            let (credential_count, kinds) = per_endpoint
                .get(&endpoint_id)
                .cloned()
                .unwrap_or((0, Vec::new()));
            for kind in kinds {
                if !supplier.credential_kinds.contains(&kind) {
                    supplier.credential_kinds.push(kind);
                }
            }
            supplier.app_count += apps.len() as i64;
            supplier.endpoints.push(SupplierEndpoint {
                id: endpoint_id,
                protocol,
                base_url,
                apps,
                consumers: details,
                credential_count,
            });
        }
    }

    Ok(suppliers.into_values().collect())
}

/// 供应商总览：网关级视角的只读聚合。
#[tauri::command]
pub async fn get_suppliers_overview(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<Supplier>, String> {
    collect(&state.db).map_err(|error| error.to_string())
}

/// 改一个网关端点的地址：所有使用它的 app 一起跟着变。
///
/// 返回更新后的总览，前端可以直接替换列表，不必再拉一次。
#[tauri::command]
pub async fn update_supplier_base_url(
    state: tauri::State<'_, AppState>,
    endpoint_id: String,
    base_url: String,
) -> Result<Vec<Supplier>, String> {
    let normalized = crate::database::Database::p1_normalize_url(&base_url)
        .map(|(_, url)| url)
        .ok_or_else(|| "base_url 格式无效（需要完整的 http(s) 地址）".to_string())?;

    {
        let mut conn = lock_conn!(state.db.conn);
        let outcome = crate::database::Database::rewrite_endpoint_base_url(
            &mut conn,
            &endpoint_id,
            &normalized,
        )
        .map_err(|error| error.to_string())?;

        if outcome.changed == 0 {
            return Err(if outcome.skipped.is_empty() {
                "该端点没有任何接入".to_string()
            } else {
                format!(
                    "这些接入的地址不在配置里，无法自动改写：{}",
                    outcome.skipped.join("、")
                )
            });
        }
        if !outcome.skipped.is_empty() {
            log::warn!(
                "[suppliers] 端点 {endpoint_id} 改址时跳过 {} 条接入：{}",
                outcome.skipped.len(),
                outcome.skipped.join("、")
            );
        }
        // 改完立即对账，让绑定指向新端点、旧端点被清理
        for app in &outcome.apps {
            crate::database::Database::reconcile_provider_bindings(&mut conn, app)
                .map_err(|error| error.to_string())?;
        }
    }

    collect(&state.db).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;
    use serde_json::json;

    fn insert_provider(db: &Database, app: &str, id: &str, config: serde_json::Value) {
        let provider = crate::provider::Provider::with_id(id.to_string(), id.to_string(), config, None);
        db.save_provider(app, &provider).expect("插入 provider");
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
        assert_eq!(supplier.endpoints.len(), 1);
        let endpoint = &supplier.endpoints[0];
        assert_eq!(endpoint.protocol, "anthropic");
        assert_eq!(endpoint.base_url, "https://ai.ctaigw.cn/v1");
        assert_eq!(endpoint.apps, vec!["claude", "claude-desktop"]);
        assert_eq!(endpoint.credential_count, 2);
        assert!(endpoint.consumers.contains(&"claude:a".to_string()));
    }

    /// 没有显式 URL 的托管 provider 自成上游，不能被误并入别的网关。
    #[test]
    fn managed_provider_becomes_its_own_supplier() {
        let db = Database::memory().expect("内存库");
        insert_provider(&db, "claude", "claude-official", json!({"env": {}}));

        let suppliers = collect(&db).expect("总览");
        let official = suppliers
            .iter()
            .find(|supplier| supplier.id.contains("claude-official"))
            .expect("托管 provider 必须自成一条供应商");
        assert_eq!(official.host, "");
        assert_eq!(official.endpoints.len(), 1);
        assert_eq!(official.endpoints[0].base_url, "");
    }
}
