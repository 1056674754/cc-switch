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
