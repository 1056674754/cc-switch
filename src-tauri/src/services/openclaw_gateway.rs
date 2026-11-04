//! P4-A：把 OpenClaw 的配置改成「单 provider 指向网关」。
//!
//! OpenClaw 的供应商在 `models.providers`，默认模型在 `agents.defaults.model.primary`，
//! 引用写法是 `<provider>/<model>`。模型规划复用 OpenCode 的那份（同一个 `plan`）。

use std::collections::HashMap;

use crate::error::AppError;
use crate::openclaw_config::{
    self, OpenClawDefaultModel, OpenClawModelEntry, OpenClawProviderConfig,
};
use crate::services::opencode_gateway::OpencodeGatewayPlan;

/// 写进 OpenClaw 的 provider id：默认模型是 `cc-switch/<模型 id>`。
pub const PROVIDER_ID: &str = "cc-switch";

/// `models.providers.cc-switch` 的内容。
pub fn provider_config(base_url: &str, plan: &OpencodeGatewayPlan) -> OpenClawProviderConfig {
    OpenClawProviderConfig {
        base_url: Some(format!("{}/openclaw/v1", base_url.trim_end_matches('/'))),
        api_key: Some("gateway".to_string()),
        api: Some("openai-completions".to_string()),
        models: plan
            .models
            .iter()
            .map(|(id, name)| OpenClawModelEntry {
                id: id.clone(),
                name: Some(name.clone()),
                alias: None,
                cost: None,
                context_window: None,
                extra: HashMap::new(),
            })
            .collect(),
        headers: HashMap::new(),
        extra: HashMap::new(),
    }
}

/// 默认模型：`agents.defaults.model.primary`。
pub fn default_model(plan: &OpencodeGatewayPlan) -> OpenClawDefaultModel {
    OpenClawDefaultModel {
        primary: format!("{PROVIDER_ID}/{}", plan.default_model),
        fallbacks: Vec::new(),
        extra: HashMap::new(),
    }
}

/// 写 OpenClaw 配置：一条 provider + 默认模型。
pub fn apply(base_url: &str, plan: &OpencodeGatewayPlan) -> Result<(), AppError> {
    openclaw_config::set_typed_provider(PROVIDER_ID, &provider_config(base_url, plan))?;
    openclaw_config::set_default_model(&default_model(plan)).map(|_| ())
}

/// 释放：删掉 `cc-switch` provider。
pub fn release() -> Result<(), AppError> {
    openclaw_config::remove_provider(PROVIDER_ID).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn provider_points_to_the_gateway_prefix_with_models() {
        let config = provider_config(
            "http://127.0.0.1:15721/",
            &plan(&[("kimi-k3", "Kimi K3")], "kimi-k3"),
        );
        assert_eq!(
            config.base_url.as_deref(),
            Some("http://127.0.0.1:15721/openclaw/v1")
        );
        assert_eq!(config.api.as_deref(), Some("openai-completions"));
        assert_eq!(config.models.len(), 1);
        assert_eq!(config.models[0].id, "kimi-k3");
        assert_eq!(config.models[0].name.as_deref(), Some("Kimi K3"));
    }

    #[test]
    fn default_model_uses_the_gateway_reference() {
        let model = default_model(&plan(&[("kimi-k3", "Kimi K3")], "kimi-k3"));
        assert_eq!(model.primary, "cc-switch/kimi-k3");
        assert!(model.fallbacks.is_empty());
    }
}
