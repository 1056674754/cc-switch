//! P4-A：把 Hermes 的配置改成「单 provider 指向网关」。
//!
//! 模型规划与 OpenCode 一致（复用 [`crate::services::opencode_gateway::plan`]），
//! 差别只在目的文件：Hermes 写 `custom_providers` 里的一条 + `model.default/provider`。
//! provider 的 `models` 用插入顺序保证第一个是默认模型（Hermes 读第一条做 provider.model）。

use serde_json::{json, Map, Value};

use crate::error::AppError;
use crate::hermes_config;
use crate::services::opencode_gateway::OpencodeGatewayPlan;

/// 写进 Hermes 的 provider 名：模型显示为 `cc-switch/<模型 id>`。
pub const PROVIDER_NAME: &str = "cc-switch";

/// provider 条目（Hermes `custom_providers` 的一条）。
pub fn provider_value(base_url: &str, plan: &OpencodeGatewayPlan) -> Value {
    let mut models = Map::new();
    // 先插默认模型：Hermes 的 set_provider 把 models 的第一个 key 当成 provider.model。
    let insert = |models: &mut Map<String, Value>, id: &str, name: &str| {
        models.insert(id.to_string(), json!({ "name": name }));
    };
    if let Some(name) = plan.models.get(&plan.default_model) {
        insert(&mut models, &plan.default_model, name);
    }
    for (id, name) in &plan.models {
        if *id != plan.default_model {
            insert(&mut models, id, name);
        }
    }
    json!({
        "base_url": format!("{}/hermes/v1", base_url.trim_end_matches('/')),
        "api_key": "gateway",
        "models": models,
    })
}

/// 写 Hermes 配置：一条 custom provider + 顶层默认模型。
pub fn apply(base_url: &str, plan: &OpencodeGatewayPlan) -> Result<(), AppError> {
    hermes_config::set_provider(PROVIDER_NAME, provider_value(base_url, plan))?;
    let mut model = hermes_config::get_model_config()?.unwrap_or_default();
    model.default = Some(plan.default_model.clone());
    model.provider = Some(PROVIDER_NAME.to_string());
    hermes_config::set_model_config(&model)?;
    Ok(())
}

/// 释放：删掉 `cc-switch` custom provider。
pub fn release() -> Result<(), AppError> {
    hermes_config::remove_provider(PROVIDER_NAME).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hermes_config::HermesModelConfig;

    fn plan(models: &[(&str, &str)], default: &str) -> OpencodeGatewayPlan {
        OpencodeGatewayPlan {
            default_model: default.to_string(),
            models: models
                .iter()
                .map(|(id, name)| (id.to_string(), name.to_string()))
                .collect(),
        }
    }

    #[test]
    fn provider_points_to_the_gateway_and_puts_the_default_model_first() {
        let value = provider_value(
            "http://127.0.0.1:15721",
            &plan(
                &[("kimi-k3", "Kimi K3"), ("gpt-5.6-sol", "GPT 5.6 Sol")],
                "gpt-5.6-sol",
            ),
        );
        assert_eq!(value["base_url"], json!("http://127.0.0.1:15721/hermes/v1"));
        assert_eq!(value["api_key"], json!("gateway"));
        let first = value["models"]
            .as_object()
            .and_then(|models| models.keys().next())
            .cloned();
        assert_eq!(first.as_deref(), Some("gpt-5.6-sol"));
    }

    #[test]
    fn provider_value_accepts_a_trailing_slash_base() {
        let value = provider_value(
            "http://127.0.0.1:15721/",
            &plan(&[("kimi-k3", "Kimi K3")], "kimi-k3"),
        );
        assert_eq!(value["base_url"], json!("http://127.0.0.1:15721/hermes/v1"));
    }

    #[test]
    fn model_config_fields_are_the_ones_hermes_reads() {
        let config = HermesModelConfig {
            default: Some("kimi-k3".to_string()),
            provider: Some(PROVIDER_NAME.to_string()),
            ..Default::default()
        };
        let json = serde_json::to_value(&config).unwrap();
        assert_eq!(json["default"], json!("kimi-k3"));
        assert_eq!(json["provider"], json!("cc-switch"));
    }
}
