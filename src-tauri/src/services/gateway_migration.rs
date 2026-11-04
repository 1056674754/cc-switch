//! Stack（聚合）配置 → 每 app 模型配置的一次性迁移（P1 收尾）。
//!
//! 迁移只做「翻译」：把 `live-state.json` 里 Stack 名单（路由那家 + 其余成员）翻成
//! `gateway_app_profiles` 的一份 `AppModelConfig`——目录里本来就有这些模型（见
//! [`crate::services::gateway_catalog`]），这里只选默认、槽位和允许列表。
//!
//! 幂等：已经配过该 app 的不覆盖（状态 `已配置，跳过`），所以重跑安全。

use serde::Serialize;

use crate::app_config::AppType;
use crate::database::Database;
use crate::error::AppError;
use crate::mode::stack::{self, Member};
use crate::services::gateway_catalog::{self, CatalogViewEntry};
use crate::services::gateway_config::AppModelConfig;

/// 一个 app 的迁移结果。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppMigration {
    pub app: String,
    pub status: MigrationStatus,
    /// 选出来的默认模型引用。
    pub default: Option<String>,
    /// 允许列表里有几条。
    pub models: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum MigrationStatus {
    /// 已写入。
    Migrated,
    /// 这个 app 没有开 Stack，跳过。
    NoStack,
    /// 已经配过，不覆盖。
    AlreadyConfigured,
    /// 这家不支持 Stack。
    Unsupported,
    /// Stack 成员都找不到可用模型，没写。
    NothingToMigrate,
}

/// 从 Stack 名单翻译出一份模型配置。纯函数，测试直接喂 `Member`。
///
/// - 默认 = 路由那家的第一个模型（Stack 写客户端文件时也用它占默认）
/// - 槽位 = 路由那家自己环境变量里配的档位（claude 的四档）
/// - 允许列表 = 所有成员的模型（就是 Stack 里能选的那些）
pub fn build_app_config(
    app: &AppType,
    members: &[Member],
    catalog: &[CatalogViewEntry],
) -> AppModelConfig {
    let Some(route) = members
        .iter()
        .find(|member| member.route)
        .or_else(|| members.first())
    else {
        return AppModelConfig::default();
    };

    let reference_of = |provider_name: &str, app: &AppType, model: &str| -> Option<String> {
        let model = crate::proxy::model_mapper::strip_one_m_suffix_for_upstream(model)
            .trim()
            .to_string();
        let source = format!("{}:{}", app.as_str(), provider_name);
        catalog
            .iter()
            .find(|entry| entry.model_id == model && entry.sources.iter().any(|s| *s == source))
            .map(|entry| entry.reference.clone())
    };

    let mut config = AppModelConfig::default();
    let route_models = gateway_catalog::raw_models(app, &route.provider);
    config.default = route_models
        .iter()
        .find_map(|model| reference_of(&route.provider.name, app, &model.model));

    // 槽位只对 claude 有意义：其余 app 的模型是单一入口。
    if matches!(app, AppType::Claude) {
        for (slot, key) in [
            ("fable", "ANTHROPIC_DEFAULT_FABLE_MODEL"),
            ("opus", "ANTHROPIC_DEFAULT_OPUS_MODEL"),
            ("sonnet", "ANTHROPIC_DEFAULT_SONNET_MODEL"),
            ("haiku", "ANTHROPIC_DEFAULT_HAIKU_MODEL"),
        ] {
            let Some(model) = route
                .provider
                .settings_config
                .get("env")
                .and_then(|env| env.get(key))
                .and_then(serde_json::Value::as_str)
            else {
                continue;
            };
            if let Some(reference) = reference_of(&route.provider.name, app, model) {
                config.slots.insert(slot.to_string(), reference);
            }
        }
    }

    let mut allowed: Vec<String> = Vec::new();
    for member in members {
        for model in gateway_catalog::raw_models(app, &member.provider) {
            let Some(reference) = reference_of(&member.provider.name, app, &model.model) else {
                continue;
            };
            if !allowed.contains(&reference) {
                allowed.push(reference);
            }
        }
    }
    config.allowed = allowed;
    config
}

/// 迁移一个 app 的 Stack 配置；已配过的 app 不动。
pub fn migrate_app(
    db: &Database,
    app: &AppType,
    mode: &crate::mode::state::ModeState,
    stack: &crate::mode::state::StackState,
    catalog: &[CatalogViewEntry],
) -> Result<AppMigration, AppError> {
    let skipped = |status| AppMigration {
        app: app.as_str().to_string(),
        status,
        default: None,
        models: 0,
    };
    if !stack::supports_stack(app) {
        return Ok(skipped(MigrationStatus::Unsupported));
    }
    if !(mode.is_proxy() && stack.enabled) {
        return Ok(skipped(MigrationStatus::NoStack));
    }
    if db.get_app_model_config(app.as_str())?.is_some() {
        return Ok(skipped(MigrationStatus::AlreadyConfigured));
    }
    let members = stack::members(db, app, stack, mode.proxy_route.as_deref())?;
    let config = build_app_config(app, &members, catalog);
    if config.default.is_none() && config.allowed.is_empty() {
        return Ok(skipped(MigrationStatus::NothingToMigrate));
    }
    db.set_app_model_config(app.as_str(), &config)?;
    Ok(AppMigration {
        app: app.as_str().to_string(),
        status: MigrationStatus::Migrated,
        default: config.default,
        models: config.allowed.len(),
    })
}

/// 迁移所有支持 Stack 的 app。
pub fn migrate_all(db: &Database) -> Result<Vec<AppMigration>, AppError> {
    let catalog = gateway_catalog::catalog_view(db)?;
    let store = crate::live::engine::DeviceStore::for_device();
    let mut results = Vec::new();
    for app in [AppType::Claude, AppType::ClaudeDesktop, AppType::Codex] {
        let mode = crate::mode::current::mode_state(&app);
        let stack = crate::mode::state::stack(&store, app.as_str())?;
        results.push(migrate_app(db, &app, &mode, &stack, &catalog)?);
    }
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::Provider;
    use serde_json::json;

    fn member(id: &str, name: &str, env: serde_json::Value, route: bool) -> Member {
        Member {
            provider: Provider::with_id(
                id.to_string(),
                name.to_string(),
                json!({ "env": env }),
                None,
            ),
            key: name.to_lowercase(),
            route,
            model_ids: Vec::new(),
        }
    }

    fn catalog_of(entries: &[(&str, &str, &str)]) -> Vec<CatalogViewEntry> {
        entries
            .iter()
            .map(|(upstream, model, source)| CatalogViewEntry {
                reference: format!("{upstream}/{model}"),
                upstream_id: format!("up-{upstream}"),
                upstream_name: (*upstream).to_string(),
                model_id: (*model).to_string(),
                display_name: None,
                one_m: false,
                window: None,
                endpoint_ids: vec!["ep-1".into()],
                sources: vec![(*source).to_string()],
            })
            .collect()
    }

    #[test]
    fn claude_migration_maps_default_slots_and_allowed() {
        let route = member(
            "p1",
            "CTYun",
            json!({
                "ANTHROPIC_MODEL": "deepseek-v4.1-flash-1017[1M]",
                "ANTHROPIC_DEFAULT_OPUS_MODEL": "qwen3.8-max",
                "ANTHROPIC_DEFAULT_HAIKU_MODEL": "qwen3.8-omni-flash"
            }),
            true,
        );
        let other = member("p2", "Kimi", json!({"ANTHROPIC_MODEL": "kimi-k3"}), false);
        let catalog = catalog_of(&[
            ("CTYun", "deepseek-v4.1-flash-1017", "claude:CTYun"),
            ("CTYun", "qwen3.8-max", "claude:CTYun"),
            ("CTYun", "qwen3.8-omni-flash", "claude:CTYun"),
            ("Kimi", "kimi-k3", "claude:Kimi"),
        ]);

        let config = build_app_config(&AppType::Claude, &[route, other], &catalog);
        assert_eq!(
            config.default.as_deref(),
            Some("CTYun/deepseek-v4.1-flash-1017")
        );
        assert_eq!(
            config.slots.get("opus").map(String::as_str),
            Some("CTYun/qwen3.8-max")
        );
        assert_eq!(
            config.slots.get("haiku").map(String::as_str),
            Some("CTYun/qwen3.8-omni-flash")
        );
        assert_eq!(
            config.allowed,
            vec![
                "CTYun/deepseek-v4.1-flash-1017",
                "CTYun/qwen3.8-max",
                "CTYun/qwen3.8-omni-flash",
                "Kimi/kimi-k3"
            ]
        );
    }

    #[test]
    fn codex_migration_has_no_slots() {
        let route = member("p1", "自定义网关", json!({}), true);
        let mut route = route;
        route.provider.settings_config = json!({
            "config": "model = \"deepseek-v4.1-flash-1017\"\n"
        });
        let catalog = catalog_of(&[("自定义网关", "deepseek-v4.1-flash-1017", "codex:自定义网关")]);
        let config = build_app_config(&AppType::Codex, &[route], &catalog);
        assert_eq!(
            config.default.as_deref(),
            Some("自定义网关/deepseek-v4.1-flash-1017")
        );
        assert!(config.slots.is_empty());
        assert_eq!(config.allowed.len(), 1);
    }

    #[test]
    fn unknown_models_are_dropped_not_fabricated() {
        let route = member(
            "p1",
            "CTYun",
            json!({"ANTHROPIC_MODEL": "gone-model"}),
            true,
        );
        let catalog = catalog_of(&[("CTYun", "qwen3.8-max", "claude:CTYun")]);
        let config = build_app_config(&AppType::Claude, &[route], &catalog);
        assert!(config.default.is_none());
        assert!(config.allowed.is_empty());
    }

    /// 真实库副本 + 本机 live-state 验证：
    /// `CCS_VERIFY_DB=/path/to/copy.db cargo test --lib gateway_migration -- --ignored --nocapture`
    #[test]
    #[ignore = "手动：用真实库副本 + 本机 live-state 验证，需 CCS_VERIFY_DB"]
    fn manual_migrate_against_real_db_copy() -> Result<(), AppError> {
        let Ok(path) = std::env::var("CCS_VERIFY_DB") else {
            return Ok(());
        };
        let db = Database::open_and_migrate_for_test(std::path::Path::new(&path))?;
        let first = migrate_all(&db)?;
        println!("first: {first:#?}");
        let second = migrate_all(&db)?;
        println!("second: {second:#?}");
        // 第二次必须全是「已配过」或「没有 Stack」
        assert!(second
            .iter()
            .all(|result| result.status != MigrationStatus::Migrated));

        let migrated: Vec<&AppMigration> = first
            .iter()
            .filter(|result| result.status == MigrationStatus::Migrated)
            .collect();
        let configs: Vec<Option<AppModelConfig>> = migrated
            .iter()
            .map(|result| db.get_app_model_config(result.app.as_str()).expect("read"))
            .collect();
        for (result, config) in migrated.iter().zip(&configs) {
            let config = config.as_ref().expect("迁移过的 app 要能读回");
            assert_eq!(config.default.as_deref(), result.default.as_deref());
            assert_eq!(config.allowed.len(), result.models);
            println!(
                "{}: default={:?} allowed={:?}",
                result.app, config.default, config.allowed
            );
        }
        Ok(())
    }

    #[test]
    fn empty_catalog_migrates_nothing() {
        let config = build_app_config(&AppType::Claude, &[], &[]);
        assert!(config.default.is_none() && config.allowed.is_empty());
    }
}
