//! P4-A：把 OpenCode 的配置改成「单 provider 指向网关」。
//!
//! 模型选择变成网关引用：OpenCode 侧只有一条 `cc-switch` provider，模型名就是引用里的
//! 模型段（`<供应商>/<模型>` 的后者）。目录显示名用作 `models.<id>.name`，顶层 `model`
//! 指向默认模型。写文件走 `opencode_config`（保留注释），并保持 provider 段是投影区。

use std::collections::BTreeMap;

use serde_json::{json, Value};

use crate::error::AppError;
use crate::opencode_config;
use crate::services::gateway_catalog::CatalogViewEntry;
use crate::services::gateway_config::AppModelConfig;

/// 写进 OpenCode 的 provider id：模型显示为 `cc-switch/<模型 id>`。
pub const PROVIDER_ID: &str = "cc-switch";
pub const PROVIDER_NAME: &str = "CC Switch";

/// 要写进 opencode 的内容（与文件 IO 分开，方便单测）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpencodeGatewayPlan {
    /// 默认模型的模型段（不含 provider 前缀）。
    pub default_model: String,
    /// 模型 id → 显示名。
    pub models: BTreeMap<String, String>,
}

/// 从每 app 模型配置派生 OpenCode 侧要写的 provider。
/// 引用先过目录校验：查不到就报错，不写一份残缺配置。
pub fn plan(
    config: &AppModelConfig,
    catalog: &[CatalogViewEntry],
) -> Result<OpencodeGatewayPlan, AppError> {
    let default_ref = config
        .default
        .as_deref()
        .ok_or_else(|| AppError::Config("OpenCode 还没有默认模型".to_string()))?;

    let mut references: Vec<&str> = Vec::new();
    for reference in std::iter::once(default_ref)
        .chain(config.allowed.iter().map(String::as_str))
        .chain(config.slots.values().map(String::as_str))
    {
        if !references.contains(&reference) {
            references.push(reference);
        }
    }

    let mut models = BTreeMap::new();
    let mut default_model = None;
    for reference in references {
        let entry = catalog
            .iter()
            .find(|entry| entry.reference == reference)
            .ok_or_else(|| AppError::Config(format!("模型引用不存在: {reference}")))?;
        let name = entry
            .display_name
            .clone()
            .filter(|name| !name.trim().is_empty())
            .unwrap_or_else(|| entry.model_id.clone());
        models.insert(entry.model_id.clone(), name);
        if reference == default_ref {
            default_model = Some(entry.model_id.clone());
        }
    }

    Ok(OpencodeGatewayPlan {
        default_model: default_model.expect("default 一定在 references 里"),
        models,
    })
}

/// provider 段的 JSON（V1 形状：`npm` + `options` + `models`）。
pub fn provider_value(base_url: &str, plan: &OpencodeGatewayPlan) -> Value {
    let models = plan
        .models
        .iter()
        .map(|(id, name)| (id.clone(), json!({ "name": name })))
        .collect::<serde_json::Map<_, _>>();
    json!({
        "npm": "@ai-sdk/openai-compatible",
        "name": PROVIDER_NAME,
        "options": {
            "baseURL": format!("{}/opencode/v1", base_url.trim_end_matches('/')),
            "apiKey": "gateway",
        },
        "models": models,
    })
}

/// 写 OpenCode 配置：一条 provider + 顶层默认模型。
pub fn apply(base_url: &str, plan: &OpencodeGatewayPlan) -> Result<(), AppError> {
    opencode_config::set_provider_with_format(
        PROVIDER_ID,
        provider_value(base_url, plan),
        crate::provider::OpenCodeConfigFormat::V1,
    )?;
    opencode_config::set_default_model(&format!("{PROVIDER_ID}/{}", plan.default_model))
}

/// 释放：删掉 `cc-switch` provider（顶层 `model` 保持原样，由用户重新选择）。
pub fn release() -> Result<(), AppError> {
    opencode_config::remove_provider(PROVIDER_ID)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::gateway_catalog::CatalogViewEntry;

    fn entry(reference: &str, model_id: &str, display: Option<&str>) -> CatalogViewEntry {
        CatalogViewEntry {
            reference: reference.to_string(),
            upstream_id: "u1".to_string(),
            upstream_name: "CTYun".to_string(),
            model_id: model_id.to_string(),
            display_name: display.map(str::to_string),
            one_m: false,
            window: None,
            endpoint_ids: Vec::new(),
            sources: Vec::new(),
        }
    }

    fn config(default: &str, allowed: &[&str], slots: &[(&str, &str)]) -> AppModelConfig {
        AppModelConfig {
            default: Some(default.to_string()),
            slots: slots
                .iter()
                .map(|(key, value)| (key.to_string(), value.to_string()))
                .collect(),
            allowed: allowed.iter().map(|value| value.to_string()).collect(),
            routing: None,
        }
    }

    #[test]
    fn plan_covers_default_allowed_and_slots_with_display_names() {
        let catalog = vec![
            entry("CTYun/kimi-k3", "kimi-k3", Some("Kimi K3")),
            entry("CTYun/gpt-5.6-sol", "gpt-5.6-sol", None),
        ];
        let plan = plan(
            &config(
                "CTYun/kimi-k3",
                &["CTYun/gpt-5.6-sol"],
                &[("sonnet", "CTYun/kimi-k3")],
            ),
            &catalog,
        )
        .expect("plan");
        assert_eq!(plan.default_model, "kimi-k3");
        assert_eq!(
            plan.models.get("kimi-k3").map(String::as_str),
            Some("Kimi K3")
        );
        assert_eq!(
            plan.models.get("gpt-5.6-sol").map(String::as_str),
            Some("gpt-5.6-sol")
        );
    }

    #[test]
    fn plan_rejects_a_reference_missing_from_the_catalog() {
        let error = plan(&config("CTYun/nope", &[], &[]), &[]).unwrap_err();
        assert!(error.to_string().contains("CTYun/nope"), "{error}");
    }

    #[test]
    fn provider_value_points_to_the_gateway_prefix() {
        let plan = OpencodeGatewayPlan {
            default_model: "kimi-k3".to_string(),
            models: [("kimi-k3".to_string(), "Kimi K3".to_string())]
                .into_iter()
                .collect(),
        };
        let value = provider_value("http://127.0.0.1:15721", &plan);
        assert_eq!(
            value["options"]["baseURL"],
            json!("http://127.0.0.1:15721/opencode/v1")
        );
        assert_eq!(value["models"]["kimi-k3"]["name"], json!("Kimi K3"));
    }
}
