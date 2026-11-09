//! 网关模型配置（P1）：每 app 的模型配置 + 目录的人工覆盖。
//!
//! 两张表在 v27 迁移里建（`database/schema.rs::create_gateway_tables`）：
//! `gateway_app_profiles`（每 app 一份，对应 CCR `ProfileConfig` 里的模型部分）与
//! `gateway_model_overrides`（人工改名/隐藏，目录本身仍由 provider 行派生）。

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::database::Database;
use crate::error::AppError;

/// 每 app 的模型配置。字段对齐 CCR 的 `ProfileConfig`（P0 §3.4 给出每个 app 的落点）。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppModelConfig {
    /// 默认模型：`<供应商名>/<模型 id>`。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
    /// 槽位 → 模型引用。哪几个槽位可用由 app 决定（claude 四个、codex 两个…），空的就不写。
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub slots: BTreeMap<String, String>,
    /// 允许模型列表；空 = 不限制。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allowed: Vec<String>,
    /// 路由策略（增强路由 / 配置级路由），原样存。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub routing: Option<Value>,
}

/// 目录里的一条人工覆盖：改名或隐藏。目录本身仍由 provider 行派生，这里只存差异。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelOverride {
    pub upstream_id: String,
    pub model_id: String,
    pub display_name: Option<String>,
    pub hidden: bool,
}

impl Database {
    /// 读某 app 的模型配置；没配过返回 `None`。
    pub fn get_app_model_config(&self, app_type: &str) -> Result<Option<AppModelConfig>, AppError> {
        let conn = crate::database::lock_conn!(self.conn);
        let payload: Option<String> = conn
            .query_row(
                "SELECT payload FROM gateway_app_profiles WHERE app_type = ?1",
                [app_type],
                |row| row.get(0),
            )
            .ok();
        let Some(payload) = payload else {
            return Ok(None);
        };
        let config = serde_json::from_str(&payload)
            .map_err(|e| AppError::Config(format!("解析 {app_type} 模型配置失败: {e}")))?;
        Ok(Some(config))
    }

    /// 写某 app 的模型配置（覆盖）。
    pub fn set_app_model_config(
        &self,
        app_type: &str,
        config: &AppModelConfig,
    ) -> Result<(), AppError> {
        crate::services::gateway_catalog::invalidate_catalog_cache();
        let payload = serde_json::to_string(config)
            .map_err(|e| AppError::Config(format!("序列化 {app_type} 模型配置失败: {e}")))?;
        let now = chrono::Utc::now().timestamp();
        let conn = crate::database::lock_conn!(self.conn);
        conn.execute(
            "INSERT INTO gateway_app_profiles (app_type, payload, updated_at)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(app_type) DO UPDATE SET payload = excluded.payload, updated_at = excluded.updated_at",
            rusqlite::params![app_type, payload, now],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    /// 目录的全部人工覆盖。
    pub fn list_model_overrides(&self) -> Result<Vec<ModelOverride>, AppError> {
        let conn = crate::database::lock_conn!(self.conn);
        let mut stmt = conn.prepare(
            "SELECT upstream_id, model_id, display_name, hidden FROM gateway_model_overrides",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(ModelOverride {
                upstream_id: row.get(0)?,
                model_id: row.get(1)?,
                display_name: row.get(2)?,
                hidden: row.get::<_, i64>(3)? != 0,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// 写一条人工覆盖（改名/隐藏）。
    pub fn set_model_override(
        &self,
        upstream_id: &str,
        model_id: &str,
        display_name: Option<&str>,
        hidden: bool,
    ) -> Result<(), AppError> {
        crate::services::gateway_catalog::invalidate_catalog_cache();
        let id = Database::p1_stable_hash(&format!("{upstream_id}|{model_id}"));
        let now = chrono::Utc::now().timestamp();
        let conn = crate::database::lock_conn!(self.conn);
        conn.execute(
            "INSERT INTO gateway_model_overrides
                 (id, upstream_id, model_id, display_name, hidden, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(upstream_id, model_id) DO UPDATE SET
                 display_name = excluded.display_name,
                 hidden = excluded.hidden",
            rusqlite::params![id, upstream_id, model_id, display_name, hidden as i64, now],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    /// 删掉一条人工覆盖，回到派生值。
    pub fn clear_model_override(&self, upstream_id: &str, model_id: &str) -> Result<(), AppError> {
        crate::services::gateway_catalog::invalidate_catalog_cache();
        let conn = crate::database::lock_conn!(self.conn);
        conn.execute(
            "DELETE FROM gateway_model_overrides WHERE upstream_id = ?1 AND model_id = ?2",
            rusqlite::params![upstream_id, model_id],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }
}

/// 供应商改名 / 合并后，老的 `供应商/模型` 引用会失效（目录里查不到）：
/// 如果目录里同一个模型 id 只剩唯一一条，就按「模型 id 后缀」把引用改写成新名字。
/// 同一个模型挂在多家供应商下时不猜（宁可报错，也不把请求导到错的家）。
/// 返回改写的引用条数（幂等）。
pub fn remap_stale_references(
    config: &mut AppModelConfig,
    catalog: &[crate::services::gateway_catalog::CatalogViewEntry],
) -> usize {
    let known: std::collections::HashSet<&str> =
        catalog.iter().map(|entry| entry.reference.as_str()).collect();
    // 老的引用 = `<旧供应商名>/<模型 id>`，只有模型 id 这段在新的命名里还可靠。
    let resolve = |reference: &str| -> Option<String> {
        if known.contains(reference) {
            return None;
        }
        let suffix_matches: Vec<&str> = catalog
            .iter()
            .filter(|entry| reference.ends_with(&format!("/{}", entry.model_id)))
            .map(|entry| entry.reference.as_str())
            .collect();
        if suffix_matches.len() != 1 {
            return None;
        }
        Some(suffix_matches[0].to_string())
    };

    let mut changed = 0;
    if let Some(default) = config.default.clone() {
        if let Some(next) = resolve(&default) {
            config.default = Some(next);
            changed += 1;
        }
    }
    for reference in config.slots.values_mut() {
        if let Some(next) = resolve(reference) {
            *reference = next;
            changed += 1;
        }
    }
    let mut seen = std::collections::HashSet::new();
    let mut allowed = Vec::with_capacity(config.allowed.len());
    for reference in &config.allowed {
        let next = resolve(reference).unwrap_or_else(|| reference.clone());
        if &next != reference {
            changed += 1;
        }
        if seen.insert(next.clone()) {
            allowed.push(next);
        }
    }
    config.allowed = allowed;
    changed
}

/// 一次修复计划：引用改写了多少条、哪些被「隐藏」挡住的模型该放出来。
#[derive(Debug, Default, PartialEq)]
pub struct AppConfigRepair {
    pub remapped: usize,
    /// `(upstream_id, model_id)`：配置里点名要用、但目录里被隐藏覆盖挡住的模型。
    pub unhide: Vec<(String, String)>,
}

/// 规划一次修复：先按模型 id 把供应商改名前的引用改写成新名字，再看这些引用是否被
/// 隐藏覆盖挡住（配置点名要用的模型必须可见，否则校验不过、发布时也拿不到模型名）。
/// 只做计划，不改库，方便单测。
pub fn plan_app_model_config_repair(
    config: &mut AppModelConfig,
    catalog_including_hidden: &[crate::services::gateway_catalog::CatalogViewEntry],
    hidden: &BTreeMap<(String, String), bool>,
) -> AppConfigRepair {
    let remapped = remap_stale_references(config, catalog_including_hidden);
    let mut references: Vec<&str> = Vec::new();
    if let Some(default) = config.default.as_deref() {
        references.push(default);
    }
    references.extend(config.slots.values().map(String::as_str));
    references.extend(config.allowed.iter().map(String::as_str));

    let mut unhide: Vec<(String, String)> = Vec::new();
    for reference in references {
        if let Some(entry) =
            crate::services::gateway_catalog::resolve_reference(catalog_including_hidden, reference)
        {
            let key = (entry.upstream_id.clone(), entry.model_id.clone());
            if hidden.get(&key).copied().unwrap_or(false) && !unhide.contains(&key) {
                unhide.push(key);
            }
        }
    }
    AppConfigRepair { remapped, unhide }
}

/// 修一遍所有 app 的模型配置（启动时跑一次，命令里也兜底）。返回修复条数。
pub fn repair_app_model_configs(db: &Database) -> Result<usize, AppError> {
    let apps: Vec<String> = {
        let conn = crate::database::lock_conn!(db.conn);
        let mut stmt = conn.prepare("SELECT app_type FROM gateway_app_profiles")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        rows.collect::<Result<Vec<_>, _>>()?
    };
    let mut total = 0;
    for app_type in apps {
        total += repair_app_model_config(db, &app_type)?;
    }
    Ok(total)
}

/// 只修一个 app（读配置前兜底；目录缓存命中时几乎无成本）。返回修复条数。
pub fn repair_app_model_config(db: &Database, app_type: &str) -> Result<usize, AppError> {
    let Some(mut config) = db.get_app_model_config(app_type)? else {
        return Ok(0);
    };
    let catalog = crate::services::gateway_catalog::catalog_view_including_hidden(db)?;
    let overrides = db.list_model_overrides()?;
    let hidden: BTreeMap<(String, String), bool> = overrides
        .iter()
        .map(|over| {
            (
                (over.upstream_id.clone(), over.model_id.clone()),
                over.hidden,
            )
        })
        .collect();
    let plan = plan_app_model_config_repair(&mut config, &catalog, &hidden);
    if plan.remapped == 0 && plan.unhide.is_empty() {
        return Ok(0);
    }

    // 配置点名要用的模型必须可见：把「发现候选默认隐藏」误伤的放出来（保留人工改的显示名）。
    let display_names: BTreeMap<(String, String), Option<String>> = overrides
        .into_iter()
        .map(|over| {
            (
                (over.upstream_id.clone(), over.model_id.clone()),
                over.display_name.clone(),
            )
        })
        .collect();
    for (upstream_id, model_id) in &plan.unhide {
        let display_name = display_names
            .get(&(upstream_id.clone(), model_id.clone()))
            .cloned()
            .flatten();
        db.set_model_override(upstream_id, model_id, display_name.as_deref(), false)?;
        log::info!("模型目录：按 {app_type} 的配置放开隐藏模型 {model_id}");
    }
    db.set_app_model_config(app_type, &config)?;
    if plan.remapped > 0 {
        log::info!("模型配置 {app_type}：{} 处引用按供应商改名修正", plan.remapped);
    }
    Ok(plan.remapped + plan.unhide.len())
}

/// 校验一份 per-app 模型配置。规则对齐 CCR 的 `AddProfileForm`：/// 校验一份 per-app 模型配置。规则对齐 CCR 的 `AddProfileForm`：
/// 引用必须能在目录里查到（严格，不回落）；允许列表非空时默认模型必填且必须在列表里。
pub fn validate_app_model_config(
    config: &AppModelConfig,
    catalog: &[crate::services::gateway_catalog::CatalogViewEntry],
) -> Result<(), AppError> {
    let exists = |reference: &str| {
        crate::services::gateway_catalog::resolve_reference(catalog, reference).is_some()
    };
    if let Some(default) = &config.default {
        if !exists(default) {
            return Err(AppError::Config(format!("默认模型不在目录里: {default}")));
        }
    }
    for (slot, reference) in &config.slots {
        if !exists(reference) {
            return Err(AppError::Config(format!(
                "槽位 {slot} 的模型不在目录里: {reference}"
            )));
        }
    }
    for reference in &config.allowed {
        if !exists(reference) {
            return Err(AppError::Config(format!(
                "允许列表里的模型不在目录里: {reference}"
            )));
        }
    }
    if !config.allowed.is_empty() {
        let Some(default) = &config.default else {
            return Err(AppError::Config(
                "允许列表非空时必须指定默认模型".to_string(),
            ));
        };
        if !config.allowed.iter().any(|allowed| allowed == default) {
            return Err(AppError::Config(format!(
                "默认模型不在允许列表里: {default}"
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_model_config_round_trips() {
        let db = Database::memory().expect("memory db");
        assert!(db.get_app_model_config("claude").expect("get").is_none());

        let config = AppModelConfig {
            default: Some("CTYun/deepseek-v4.1-flash-1017".into()),
            slots: BTreeMap::from([("haiku".into(), "CTYun/qwen3.8-omni-flash".into())]),
            allowed: vec!["CTYun/deepseek-v4.1-flash-1017".into()],
            routing: None,
        };
        db.set_app_model_config("claude", &config).expect("set");

        let read = db
            .get_app_model_config("claude")
            .expect("get")
            .expect("some");
        assert_eq!(read, config);
        // 其它 app 不受影响
        assert!(db.get_app_model_config("codex").expect("get").is_none());
    }

    fn catalog(
        entries: &[(&str, &str)],
    ) -> Vec<crate::services::gateway_catalog::CatalogViewEntry> {
        entries
            .iter()
            .map(
                |(upstream, model)| crate::services::gateway_catalog::CatalogViewEntry {
                    reference: format!("{upstream}/{model}"),
                    upstream_id: format!("up-{upstream}"),
                    upstream_name: (*upstream).to_string(),
                    model_id: (*model).to_string(),
                    display_name: None,
                    one_m: false,
                    window: None,
                    endpoint_ids: vec!["ep-1".into()],
                    sources: vec![],
                },
            )
            .collect()
    }

    #[test]
    fn plan_unhides_models_the_config_references_but_discovery_hid() {
        // 目录里模型还在（含隐藏），只是被「发现候选默认隐藏」挡住
        let entries = catalog(&[("OpenAI Official", "gpt-6.1-sol")]);
        let hidden = BTreeMap::from([(
            ("up-OpenAI Official".to_string(), "gpt-6.1-sol".to_string()),
            true,
        )]);
        let mut config = AppModelConfig {
            default: Some("Codex/gpt-6.1-sol".into()),
            slots: BTreeMap::from([("opus".into(), "Codex/gpt-6.1-sol".into())]),
            allowed: vec!["Codex/gpt-6.1-sol".into()],
            routing: None,
        };

        let plan = plan_app_model_config_repair(&mut config, &entries, &hidden);

        // default + 槽位 + 允许列表三处引用都改写成新名字
        assert_eq!(plan.remapped, 3);
        assert_eq!(config.default.as_deref(), Some("OpenAI Official/gpt-6.1-sol"));
        // 同一个模型只放开一次
        assert_eq!(
            plan.unhide,
            vec![("up-OpenAI Official".to_string(), "gpt-6.1-sol".to_string())]
        );

        // 已经可见的模型不会被列进放开名单
        let visible = BTreeMap::from([(
            ("up-OpenAI Official".to_string(), "gpt-6.1-sol".to_string()),
            false,
        )]);
        let plan = plan_app_model_config_repair(&mut config, &entries, &visible);
        assert!(plan.unhide.is_empty());
        assert_eq!(plan.remapped, 0);
    }

    #[test]
    fn validation_rejects_unknown_references() {
        let entries = catalog(&[("CTYun", "qwen3.8-max")]);
        let mut config = AppModelConfig {
            default: Some("CTYun/not-there".into()),
            ..Default::default()
        };
        assert!(validate_app_model_config(&config, &entries).is_err());

        config.default = Some("CTYun/qwen3.8-max".into());
        assert!(validate_app_model_config(&config, &entries).is_ok());

        config.slots = BTreeMap::from([("haiku".into(), "CTYun/nope".into())]);
        assert!(validate_app_model_config(&config, &entries).is_err());
    }

    #[test]
    fn remap_rewrites_references_after_a_supplier_rename() {
        // 供应商 Codex 合并进 OpenAI Official：模型 id 还在，名字变了
        let entries = catalog(&[
            ("OpenAI Official", "gpt-6.1-sol"),
            ("OpenAI Official", "gpt-6.1-terra"),
            ("CTYun", "deepseek-v4.1-flash-1017"),
        ]);
        let mut config = AppModelConfig {
            default: Some("Codex/gpt-6.1-sol".into()),
            slots: BTreeMap::from([
                ("opus".into(), "Codex/gpt-6.1-sol".into()),
                ("sonnet".into(), "Codex/gpt-6.1-terra".into()),
                ("haiku".into(), "CTYun/deepseek-v4.1-flash-1017".into()),
            ]),
            allowed: vec![
                "Codex/gpt-6.1-sol".into(),
                "Codex/gpt-6.1-terra".into(),
                "Codex/gpt-6.1-sol".into(),
                "CTYun/deepseek-v4.1-flash-1017".into(),
            ],
            routing: None,
        };

        let changed = remap_stale_references(&mut config, &entries);

        // default + 两个槽位 + 允许列表三条过期引用
        assert_eq!(changed, 6);
        assert_eq!(config.default.as_deref(), Some("OpenAI Official/gpt-6.1-sol"));
        assert_eq!(config.slots["opus"], "OpenAI Official/gpt-6.1-sol");
        assert_eq!(config.slots["sonnet"], "OpenAI Official/gpt-6.1-terra");
        assert_eq!(config.slots["haiku"], "CTYun/deepseek-v4.1-flash-1017");
        assert_eq!(
            config.allowed,
            vec![
                "OpenAI Official/gpt-6.1-sol",
                "OpenAI Official/gpt-6.1-terra",
                "CTYun/deepseek-v4.1-flash-1017",
            ]
        );
        assert!(validate_app_model_config(&config, &entries).is_ok());

        // 幂等：再修一次没有变化
        assert_eq!(remap_stale_references(&mut config, &entries), 0);
    }

    #[test]
    fn remap_leaves_ambiguous_and_unknown_references_alone() {
        // 同一个模型挂在两家供应商下 → 不猜
        let entries = catalog(&[("CTYun", "shared-model"), ("Kimi", "shared-model")]);
        let mut config = AppModelConfig {
            default: Some("Old/shared-model".into()),
            ..Default::default()
        };
        assert_eq!(remap_stale_references(&mut config, &entries), 0);
        assert_eq!(config.default.as_deref(), Some("Old/shared-model"));

        // 目录里根本没有这个模型 → 原样保留（由校验报错）
        let entries = catalog(&[("CTYun", "kimi-k3")]);
        let mut config = AppModelConfig {
            default: Some("Gone/not-there".into()),
            ..Default::default()
        };
        assert_eq!(remap_stale_references(&mut config, &entries), 0);
        assert_eq!(config.default.as_deref(), Some("Gone/not-there"));

        // 已经正确的引用不动
        let mut config = AppModelConfig {
            default: Some("CTYun/kimi-k3".into()),
            ..Default::default()
        };
        assert_eq!(remap_stale_references(&mut config, &entries), 0);
    }

    #[test]
    fn validation_requires_default_when_allowed_is_set() {
        let entries = catalog(&[("CTYun", "qwen3.8-max"), ("CTYun", "kimi-k3")]);
        let without_default = AppModelConfig {
            allowed: vec!["CTYun/qwen3.8-max".into()],
            ..Default::default()
        };
        assert!(validate_app_model_config(&without_default, &entries).is_err());

        let outside = AppModelConfig {
            default: Some("CTYun/kimi-k3".into()),
            allowed: vec!["CTYun/qwen3.8-max".into()],
            ..Default::default()
        };
        assert!(validate_app_model_config(&outside, &entries).is_err());

        let ok = AppModelConfig {
            default: Some("CTYun/qwen3.8-max".into()),
            allowed: vec!["CTYun/qwen3.8-max".into()],
            ..Default::default()
        };
        assert!(validate_app_model_config(&ok, &entries).is_ok());
    }

    #[test]
    fn model_overrides_round_trip_and_clear() {
        let db = Database::memory().expect("memory db");
        assert!(db.list_model_overrides().expect("list").is_empty());

        db.set_model_override("up-1", "kimi-k3", Some("Kimi K3"), false)
            .expect("set");
        let overrides = db.list_model_overrides().expect("list");
        assert_eq!(overrides.len(), 1);
        assert_eq!(overrides[0].display_name.as_deref(), Some("Kimi K3"));

        // 同一个（上游, 模型）覆盖写，不新增行
        db.set_model_override("up-1", "kimi-k3", None, true)
            .expect("set");
        let overrides = db.list_model_overrides().expect("list");
        assert_eq!(overrides.len(), 1);
        assert!(overrides[0].hidden);

        db.clear_model_override("up-1", "kimi-k3").expect("clear");
        assert!(db.list_model_overrides().expect("list").is_empty());
    }
}
