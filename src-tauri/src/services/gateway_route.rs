//! 按模型选上游（P2 的路由部分）。
//!
//! 客户端请求里的 `model` 就是路由键：
//! - 形如 `<供应商名>/<模型 id>` → 发往那家供应商，上游模型名换成 `/` 后面那段
//! - 其它（普通模型名）→ 交给现有的默认路由，不在这一层猜
//!
//! 解析直接复用目录（[`crate::services::gateway_catalog::catalog_view`]）和
//! [`crate::mode::stack::StackTarget`]：后者已经是「单家、不转移」的请求形状，
//! 代理管线里 `RequestContext` 对它的处理正是我们要的语义。

use crate::app_config::AppType;
use crate::database::Database;
use crate::error::AppError;
use crate::mode::stack::StackTarget;
use crate::services::gateway_catalog::{resolve_reference, CatalogViewEntry};

/// 把 `<供应商名>/<模型 id>` 解析成转发目标。不是引用形状时返回 `None`。
///
/// 引用形状但查不到（供应商不存在、模型不在目录、这家 app 没有绑定该上游）返回 `Err`：
/// 用户显式点名了模型，不能静默落到别的上游。
pub fn resolve_reference_target(
    db: &Database,
    app: &AppType,
    model: &str,
) -> Result<Option<StackTarget>, AppError> {
    let Some((supplier, upstream_model)) = model.split_once('/') else {
        return Ok(None);
    };
    let supplier = supplier.trim();
    let upstream_model =
        crate::proxy::model_mapper::strip_one_m_suffix_for_upstream(upstream_model.trim())
            .trim()
            .to_string();
    if supplier.is_empty() || upstream_model.is_empty() {
        return Ok(None);
    }

    let catalog = crate::services::gateway_catalog::catalog_view_cached(db)?;
    let reference = format!("{supplier}/{upstream_model}");
    let Some(entry) = crate::services::gateway_catalog::resolve_reference(&catalog, &reference)
    else {
        // 供应商名对得上、模型对不上 → 用户点名了一个不存在的模型，报错不回落。
        // 供应商名也对不上 → 可能是客户端自己的别名（如 `anthropic/claude-x`），走默认路由。
        let known_supplier = catalog.iter().any(|entry| entry.upstream_name == supplier);
        if !known_supplier {
            return Ok(None);
        }
        return Err(AppError::Config(format!(
            "模型 {reference} 不在目录里（供应商 {supplier} 没有发布这个模型）"
        )));
    };

    // 目录来源里有这一家的名字，用它反查 provider 行。按 app 过滤：同一个上游在
    // claude 与 claude-desktop 下是两条 provider 行，各自有自己的 settings。
    let source_prefix = format!("{}:", app.as_str());
    let provider_name = entry
        .sources
        .iter()
        .find_map(|source| source.strip_prefix(&source_prefix))
        .ok_or_else(|| {
            AppError::Config(format!(
                "模型 {} 不属于 {}：这家 app 没有绑定该上游",
                model,
                app.as_str()
            ))
        })?;

    let providers = db.get_all_providers(app.as_str())?;
    let mut candidates = providers
        .into_iter()
        .filter(|(_, provider)| provider.name == provider_name);
    let Some((_, provider)) = candidates.next() else {
        return Err(AppError::Config(format!(
            "模型 {} 的供应商 {provider_name} 在 {} 里已不存在",
            model,
            app.as_str()
        )));
    };
    Ok(Some(StackTarget {
        provider,
        upstream_model,
        original_model: model.to_string(),
    }))
}

/// 累加式 app（OpenCode 等）的客户端会把 provider 前缀吃掉，只发裸模型 id。
/// 在该 app 的网关模型配置里挑引用：同名模型唯一命中就选它；多家都发布时报歧义；
/// 一个都没配命中而目录里确实有这个模型时报「不在允许列表」；否则回落到默认模型。
pub fn pick_configured_reference(
    config: &crate::services::gateway_config::AppModelConfig,
    catalog: &[CatalogViewEntry],
    model: &str,
    app: &str,
) -> Result<Option<String>, AppError> {
    let bare = crate::proxy::model_mapper::strip_one_m_suffix_for_upstream(model.trim())
        .trim()
        .to_string();
    let mut references: Vec<&str> = Vec::new();
    for reference in std::iter::once(config.default.as_deref())
        .flatten()
        .chain(config.allowed.iter().map(String::as_str))
        .chain(config.slots.values().map(String::as_str))
    {
        if !references.contains(&reference) {
            references.push(reference);
        }
    }
    let configured: Vec<&CatalogViewEntry> = references
        .iter()
        .filter_map(|reference| resolve_reference(catalog, reference))
        .collect();
    let hits: Vec<&&CatalogViewEntry> = configured
        .iter()
        .filter(|entry| entry.model_id == bare)
        .collect();
    match hits.len() {
        1 => Ok(Some(hits[0].reference.clone())),
        n if n > 1 => Err(AppError::Config(format!(
            "模型 {model} 在多条引用里都存在，请用 <供应商>/<模型> 指定上游"
        ))),
        _ => {
            let published = catalog.iter().any(|entry| {
                entry.model_id == bare
                    && entry
                        .sources
                        .iter()
                        .any(|source| source.starts_with(&format!("{app}:")))
            });
            if published {
                return Err(AppError::Config(format!(
                    "模型 {model} 没有为此应用配置（不在默认模型 / 槽位 / 允许列表里）"
                )));
            }
            Ok(configured
                .iter()
                .find(|entry| config.default.as_deref() == Some(entry.reference.as_str()))
                .map(|entry| entry.reference.clone()))
        }
    }
}

/// 把裸模型 id 解析成 `<供应商>/<模型>` 再查上游（给累加式 app 的请求热路径用）。
pub fn resolve_configured_model(
    db: &Database,
    app: &AppType,
    model: &str,
) -> Result<Option<StackTarget>, AppError> {
    if model.trim().is_empty() {
        return Ok(None);
    }
    let Some(config) = db.get_app_model_config(app.as_str())? else {
        return Ok(None);
    };
    let catalog = crate::services::gateway_catalog::catalog_view_cached(db)?;
    let Some(reference) = pick_configured_reference(&config, &catalog, model, app.as_str())? else {
        return Ok(None);
    };
    resolve_reference_target(db, app, &reference)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::Provider;
    use serde_json::json;

    fn claude_provider(id: &str, name: &str, base_url: &str, models: &[&str]) -> Provider {
        let mut env = json!({
            "ANTHROPIC_BASE_URL": base_url,
            "ANTHROPIC_AUTH_TOKEN": "sk-test",
            "ANTHROPIC_MODEL": models[0],
        });
        for (index, slot) in ["OPUS", "SONNET", "HAIKU", "FABLE"].iter().enumerate() {
            if let Some(model) = models.get(index + 1) {
                env[format!("ANTHROPIC_DEFAULT_{slot}_MODEL")] = json!(model);
            }
        }
        Provider::with_id(
            id.to_string(),
            name.to_string(),
            json!({ "env": env }),
            None,
        )
    }

    fn catalog_entry(reference: &str, model_id: &str) -> CatalogViewEntry {
        CatalogViewEntry {
            reference: reference.to_string(),
            upstream_id: "u1".to_string(),
            upstream_name: reference.split_once('/').unwrap().0.to_string(),
            model_id: model_id.to_string(),
            display_name: Some(model_id.to_string()),
            one_m: false,
            window: None,
            endpoint_ids: Vec::new(),
            sources: vec!["opencode:p".to_string()],
        }
    }

    fn app_config(
        default: Option<&str>,
        allowed: &[&str],
        slots: &[(&str, &str)],
    ) -> crate::services::gateway_config::AppModelConfig {
        crate::services::gateway_config::AppModelConfig {
            default: default.map(str::to_string),
            slots: slots
                .iter()
                .map(|(key, value)| (key.to_string(), value.to_string()))
                .collect(),
            allowed: allowed.iter().map(|value| value.to_string()).collect(),
            routing: None,
        }
    }

    /// 真机烟测（默认忽略）：把真实 cc-switch.db 的**副本**放进临时 home，
    /// 让 `Database::init()` 走一遍真实迁移，再验证目录派生和引用解析。
    ///
    /// 跑法：`CC_SWITCH_REAL_DB=~/.cc-switch/cc-switch.db cargo test --lib
    /// real_db_copy_migrates -- --ignored --nocapture`
    #[test]
    #[ignore = "需要 CC_SWITCH_REAL_DB 指向真实 cc-switch.db 的副本"]
    #[serial_test::serial]
    fn real_db_copy_migrates_to_current_schema_and_derives_catalog() {
        let source = std::env::var("CC_SWITCH_REAL_DB").expect("CC_SWITCH_REAL_DB");
        let home = tempfile::tempdir().expect("temp home");
        let config_dir = home.path().join(".cc-switch");
        std::fs::create_dir_all(&config_dir).expect("config dir");
        let db_path = config_dir.join("cc-switch.db");
        std::fs::copy(&source, &db_path).expect("copy real db");

        let user_version = |path: &std::path::Path| -> i32 {
            let conn = rusqlite::Connection::open(path).expect("open copy");
            conn.query_row("PRAGMA user_version", [], |row| row.get(0))
                .expect("user_version")
        };
        let before = user_version(&db_path);
        std::env::set_var("CC_SWITCH_TEST_HOME", home.path());
        let db = Database::init().expect("migrate db copy");
        let after = user_version(&db_path);
        eprintln!("user_version: {before} -> {after}");
        assert_eq!(after, crate::database::SCHEMA_VERSION);
        assert!(after >= before, "迁移不能降版本");

        let conn = rusqlite::Connection::open(&db_path).expect("reopen copy");
        let table_count: i64 = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='gateway_app_profiles'",
                [],
                |row| row.get(0),
            )
            .expect("gateway_app_profiles");
        assert_eq!(table_count, 1, "迁移后应该有 gateway_app_profiles");
        let upstreams: i64 = conn
            .query_row("SELECT count(*) FROM upstreams", [], |row| row.get(0))
            .expect("upstreams");
        eprintln!("upstreams in real db: {upstreams}");
        assert!(upstreams > 0, "真实库应该有供应商");

        let catalog = crate::services::gateway_catalog::catalog_view_cached(&db)
            .expect("derive catalog from real db");
        eprintln!("catalog entries: {}", catalog.len());
        assert!(!catalog.is_empty(), "真实库的目录不该是空的");

        let entry = catalog
            .iter()
            .find(|entry| {
                entry
                    .sources
                    .iter()
                    .any(|source| source.starts_with("claude:"))
            })
            .expect("real db has a claude-bound supplier");
        let target = resolve_reference_target(&db, &AppType::Claude, &entry.reference)
            .expect("resolve against real catalog")
            .expect("hit");
        assert!(!target.provider.id.is_empty());
        eprintln!(
            "resolved {} -> provider {} model {}",
            entry.reference, target.provider.id, target.upstream_model
        );

        std::env::remove_var("CC_SWITCH_TEST_HOME");
    }

    #[test]
    fn bare_model_picks_the_unique_configured_reference() {
        let catalog = vec![
            catalog_entry("CTYun/kimi-k3", "kimi-k3"),
            catalog_entry("Kimi/gpt-5", "gpt-5"),
        ];
        let config = app_config(Some("Kimi/gpt-5"), &["CTYun/kimi-k3"], &[]);
        let picked =
            pick_configured_reference(&config, &catalog, "kimi-k3", "opencode").expect("pick");
        assert_eq!(picked.as_deref(), Some("CTYun/kimi-k3"));

        // 没配过的模型名回落到默认引用
        let picked = pick_configured_reference(&config, &catalog, "unknown-model", "opencode")
            .expect("fallback");
        assert_eq!(picked.as_deref(), Some("Kimi/gpt-5"));
    }

    #[test]
    fn bare_model_ambiguity_and_allowlist_errors_are_reported() {
        let catalog = vec![
            catalog_entry("CTYun/kimi-k3", "kimi-k3"),
            catalog_entry("Kimi/kimi-k3", "kimi-k3"),
            catalog_entry("Kimi/gpt-5", "gpt-5"),
        ];
        let config = app_config(None, &["CTYun/kimi-k3", "Kimi/kimi-k3"], &[]);
        let error =
            pick_configured_reference(&config, &catalog, "kimi-k3", "opencode").unwrap_err();
        assert!(error.to_string().contains("指定上游"), "{error}");

        // 目录里有、但应用没配 -> 报不在允许列表，不静默换模型
        let catalog = vec![
            catalog_entry("CTYun/kimi-k3", "kimi-k3"),
            catalog_entry("Kimi/gpt-5", "gpt-5"),
        ];
        let config = app_config(Some("Kimi/gpt-5"), &[], &[]);
        let error =
            pick_configured_reference(&config, &catalog, "kimi-k3", "opencode").unwrap_err();
        assert!(error.to_string().contains("没有为此应用配置"), "{error}");
    }

    #[test]
    fn slot_models_are_configured_references_too() {
        let catalog = vec![
            catalog_entry("CTYun/opus-x", "opus-x"),
            catalog_entry("Kimi/gpt-5", "gpt-5"),
        ];
        let config = app_config(Some("Kimi/gpt-5"), &[], &[("opus", "CTYun/opus-x")]);
        let picked =
            pick_configured_reference(&config, &catalog, "opus-x", "opencode").expect("pick");
        assert_eq!(picked.as_deref(), Some("CTYun/opus-x"));
    }

    #[test]
    fn reference_resolves_to_that_supplier_and_model() {
        let db = Database::memory().expect("memory db");
        db.save_provider(
            "claude",
            &claude_provider(
                "p-ctyun",
                "CTYun",
                "https://ai.ctaigw.cn/v1",
                &["deepseek-v4.1-flash-1017", "qwen3.8-max"],
            ),
        )
        .expect("save");
        db.save_provider(
            "claude",
            &claude_provider(
                "p-kimi",
                "Kimi",
                "https://api.kimi.com/coding",
                &["kimi-k3"],
            ),
        )
        .expect("save");

        let target =
            resolve_reference_target(&db, &AppType::Claude, "CTYun/deepseek-v4.1-flash-1017")
                .expect("resolve")
                .expect("hit");
        assert_eq!(target.provider.id, "p-ctyun");
        assert_eq!(target.upstream_model, "deepseek-v4.1-flash-1017");
        assert_eq!(target.original_model, "CTYun/deepseek-v4.1-flash-1017");

        // `[1M]` 标记在引用里要剥掉，仍能解析
        let target =
            resolve_reference_target(&db, &AppType::Claude, "CTYun/deepseek-v4.1-flash-1017[1M]")
                .expect("resolve")
                .expect("hit");
        assert_eq!(target.provider.id, "p-ctyun");
        assert_eq!(target.upstream_model, "deepseek-v4.1-flash-1017");
    }

    #[test]
    fn plain_model_names_are_left_to_the_default_route() {
        let db = Database::memory().expect("memory db");
        assert!(
            resolve_reference_target(&db, &AppType::Claude, "claude-sonnet-4-5")
                .expect("resolve")
                .is_none()
        );
    }

    #[test]
    fn unknown_model_under_a_known_supplier_is_an_error() {
        let db = Database::memory().expect("memory db");
        db.save_provider(
            "claude",
            &claude_provider(
                "p-ctyun",
                "CTYun",
                "https://ai.ctaigw.cn/v1",
                &["deepseek-v4.1-flash-1017"],
            ),
        )
        .expect("save");
        let result = resolve_reference_target(&db, &AppType::Claude, "CTYun/not-a-model")
            .expect_err("要报错");
        assert!(result.to_string().contains("不在目录里"), "{result}");
    }

    /// 真实库副本验证：
    /// `CCS_VERIFY_DB=/path/to/copy.db cargo test --lib gateway_route -- --ignored --nocapture`
    #[test]
    #[ignore = "手动：用真实库副本验证，需 CCS_VERIFY_DB"]
    fn manual_resolve_against_real_db_copy() -> Result<(), AppError> {
        let Ok(path) = std::env::var("CCS_VERIFY_DB") else {
            return Ok(());
        };
        let db = Database::open_and_migrate_for_test(std::path::Path::new(&path))?;
        let catalog = crate::services::gateway_catalog::catalog_view(&db)?;
        for entry in catalog
            .iter()
            .filter(|entry| {
                entry
                    .sources
                    .iter()
                    .any(|source| source.starts_with("claude:"))
            })
            .take(4)
        {
            let target = resolve_reference_target(&db, &AppType::Claude, &entry.reference)?;
            println!(
                "{} -> {:?}",
                entry.reference,
                target.as_ref().map(|t| (&t.provider.id, &t.upstream_model))
            );
            assert!(target.is_some(), "{} 应能解析", entry.reference);
        }
        // 客户端自己的别名（供应商名不存在）→ 交给默认路由
        assert!(resolve_reference_target(&db, &AppType::Claude, "anthropic/claude-x")?.is_none());
        Ok(())
    }

    #[test]
    fn reference_from_another_app_is_rejected_or_missing() {
        let db = Database::memory().expect("memory db");
        db.save_provider(
            "claude-desktop",
            &claude_provider(
                "p-ctyun",
                "CTYun",
                "https://ai.ctaigw.cn/v1",
                &["deepseek-v4.1-flash-1017"],
            ),
        )
        .expect("save");

        // 目录里没有 model 来源是 claude，故不是引用命中
        let result =
            resolve_reference_target(&db, &AppType::Claude, "CTYun/deepseek-v4.1-flash-1017")
                .expect("resolve");
        assert!(result.is_none());
    }
}
