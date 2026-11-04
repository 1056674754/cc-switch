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
    /// 接入明细（含模型映射），前端编辑映射时用来预填
    pub bindings: Vec<BindingDetail>,
}

/// 一条接入：哪个 app、哪个 provider、当前模型映射。
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BindingDetail {
    pub app_type: String,
    pub provider_id: String,
    /// 槽位化 JSON（haiku/sonnet/opus/fable/subagent/default），空对象表示没配
    pub model_map: serde_json::Value,
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
            let mut binding_details: Vec<BindingDetail> = Vec::new();
            for (app_type, provider_id, model_map) in consumers.get(&endpoint_id).into_iter().flatten()
            {
                if !apps.contains(app_type) {
                    apps.push(app_type.clone());
                }
                details.push(format!("{app_type}:{provider_id}"));
                binding_details.push(BindingDetail {
                    app_type: app_type.clone(),
                    provider_id: provider_id.clone(),
                    model_map: serde_json::from_str(model_map)
                        .unwrap_or_else(|_| serde_json::json!({})),
                });
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
                bindings: binding_details,
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
    endpoint_id: String,
) -> Result<AdoptedProvider, String> {
    adopt_into(&state.db, &app_type, &endpoint_id)
}

fn adopt_into(
    db: &crate::database::Database,
    app_type: &str,
    endpoint_id: &str,
) -> Result<AdoptedProvider, String> {
    if !matches!(app_type, "claude" | "claude-desktop") {
        return Err(format!(
            "{app_type} 暂不支持一键采用：它的配置需要按供应商维护模型列表/认证，无法凭空生成。请在应用页手动添加。"
        ));
    }

    let (name, base_url, protocol, secret) = {
        let conn = lock_conn!(db.conn);
        let row = conn
            .query_row(
                "SELECT u.name, e.base_url, e.protocol
                 FROM upstream_endpoints e JOIN upstreams u ON u.id = e.upstream_id
                 WHERE e.id = ?1",
                rusqlite::params![endpoint_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            )
            .map_err(|_| "找不到这个端点".to_string())?;

        let credential: Option<(Option<String>, String)> = conn
            .query_row(
                "SELECT secret, COALESCE(secret_kind, 'none') FROM upstream_credentials
                 WHERE endpoint_id = ?1 ORDER BY app_type LIMIT 1",
                rusqlite::params![endpoint_id],
                |row| Ok((row.get::<_, Option<String>>(0)?, row.get::<_, String>(1)?)),
            )
            .ok();

        match credential {
            Some((Some(secret), kind)) if kind == "literal" && !secret.trim().is_empty() => {
                (row.0, row.1, row.2, secret)
            }
            Some((_, kind)) if kind == "env_ref" => {
                return Err(
                    "这家供应商的密钥只存了环境变量名，库里没有明文，无法一键采用".to_string()
                )
            }
            Some((_, kind)) if kind == "managed" => {
                return Err("这家供应商用的是托管登录态，无法一键采用".to_string())
            }
            _ => return Err("这家供应商还没有可用的密钥，请先补上".to_string()),
        }
    };

    if protocol != "anthropic" {
        return Err(format!(
            "这个端点是 {protocol} 协议，{app_type} 只吃 anthropic 协议，无法直接采用"
        ));
    }

    let provider_id = format!("sup-{endpoint_id}");
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

/// 换一把密钥：所有使用这个端点的 app 一起换。
#[tauri::command]
pub async fn update_supplier_secret(
    state: tauri::State<'_, AppState>,
    endpoint_id: String,
    secret: String,
) -> Result<Vec<Supplier>, String> {
    let secret = secret.trim().to_string();
    if secret.is_empty() {
        return Err("密钥不能为空".to_string());
    }

    {
        let mut conn = lock_conn!(state.db.conn);
        let outcome = crate::database::Database::rewrite_endpoint_secret(
            &mut conn,
            &endpoint_id,
            &secret,
        )
        .map_err(|error| error.to_string())?;

        if outcome.changed == 0 {
            return Err(if outcome.skipped.is_empty() {
                "该端点没有任何接入".to_string()
            } else {
                format!(
                    "这些接入的密钥存在环境变量里，库里没有明文可换：{}",
                    outcome.skipped.join("、")
                )
            });
        }
        if !outcome.skipped.is_empty() {
            log::warn!(
                "[suppliers] 端点 {endpoint_id} 换密钥时跳过 {} 条接入：{}",
                outcome.skipped.len(),
                outcome.skipped.join("、")
            );
        }
        for app in &outcome.apps {
            crate::database::Database::reconcile_provider_bindings(&mut conn, app)
                .map_err(|error| error.to_string())?;
        }
    }

    collect(&state.db).map_err(|error| error.to_string())
}

/// 改某个接入的模型映射（`app + provider` 粒度）。
///
/// 同一个网关，不同 app 可以映射成不同模型——这正是「映射归接入」的含义。
/// 在网关侧新建一家供应商，并直接接进选中的 app。
///
/// 这是"一次配齐"的入口：以前只能先在某个 app 里建一条 provider，再「采用」到别的
/// app；现在反过来——先在网关填名字/地址/密钥，勾选要吃这条路由的 agent。
/// 每个 app 落一条**确定性 id**（同一名字+地址+app 永远同一条），重复提交不会出重复行。
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
        let digest =
            crate::database::Database::p1_stable_hash(&format!("{name}|{base_url}|{app}"));
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
        let endpoint = suppliers
            .iter()
            .flat_map(|supplier| supplier.endpoints.iter())
            .find(|endpoint| endpoint.base_url == "https://ai.ctaigw.cn/v1")
            .expect("端点");

        let adopted = adopt_into(&db, "claude-desktop", &endpoint.id).expect("应能采用");
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
        assert_eq!(
            stored["env"]["ANTHROPIC_AUTH_TOKEN"],
            json!("sk-secret")
        );

        // 采用后两个 app 应落在同一个网关上
        let suppliers = collect(&db).expect("总览2");
        let ctaigw: Vec<_> = suppliers
            .iter()
            .filter(|supplier| supplier.host == "ai.ctaigw.cn")
            .collect();
        assert_eq!(ctaigw.len(), 1, "同一网关只能出现一条供应商");
        assert_eq!(ctaigw[0].app_count, 2, "claude 与 claude-desktop 都应算上");
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
        let endpoint_id = suppliers[0].endpoints[0].id.clone();
        let error = adopt_into(&db, "opencode", &endpoint_id).expect_err("应拒绝");
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
        let endpoint = &suppliers[0].endpoints[0];
        // opencode 的协议由 npm 包决定，派生不出 anthropic —— 正是要拒绝的情况
        assert_ne!(endpoint.protocol, "anthropic");
        let error = adopt_into(&db, "claude", &endpoint.id).expect_err("应拒绝");
        assert!(error.contains("协议"), "错误应说明协议不匹配: {error}");
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
            "命中：name={} host={} apps={} endpoints={}",
            supplier.name,
            supplier.host,
            supplier.app_count,
            supplier.endpoints.len()
        );
        for endpoint in &supplier.endpoints {
            println!(
                "  端点 {} [{}] base_url={} apps={:?} creds={}",
                endpoint.id, endpoint.protocol, endpoint.base_url, endpoint.apps, endpoint.credential_count
            );
        }
        assert_eq!(supplier.host, "verify.example.com");
        assert_eq!(supplier.app_count, 2);
        assert_eq!(supplier.endpoints.len(), 1);
        assert_eq!(supplier.endpoints[0].base_url, "https://verify.example.com/v1");

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
        assert_eq!(supplier.app_count, 2, "两个 app 都该接上");
        assert_eq!(supplier.endpoints.len(), 1, "同一地址只应有一个端点");
        let endpoint = &supplier.endpoints[0];
        assert_eq!(endpoint.base_url, "https://ai.ctaigw.cn/v1", "尾部斜杠应被规范化");
        assert_eq!(endpoint.protocol, "anthropic");
        assert_eq!(endpoint.apps.len(), 2);

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
            assert_eq!(config["env"]["ANTHROPIC_BASE_URL"], "https://ai.ctaigw.cn/v1");
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
        let suppliers = create_into(&db, "Acme", "https://acme.dev/v1", "sk-2", &apps)
            .expect("第二次");

        assert_eq!(suppliers.len(), 1, "同一家只应有一条供应商");
        assert_eq!(suppliers[0].app_count, 1);
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
            .find(|supplier| supplier.id.contains("claude-official"))
            .expect("托管 provider 必须自成一条供应商");
        assert_eq!(official.host, "");
        assert_eq!(official.endpoints.len(), 1);
        assert_eq!(official.endpoints[0].base_url, "");
    }
}
