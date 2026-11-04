//! P4-A：把 Pi 的配置改成「单 provider 指向网关」。
//!
//! Pi 的 provider 存在 `~/.pi/agent/models.json`（键是 provider key），默认模型存在
//! `settings.json` 的 `defaultProvider` / `defaultModel`。两个文件都要写，缺一个都
//! 不算接管成功。模型规划复用 OpenCode 的那份 `plan`。

use serde_json::{json, Value};

use crate::error::AppError;
use crate::pi_config;
use crate::services::opencode_gateway::OpencodeGatewayPlan;

/// 写进 Pi 的 provider key：模型引用为 `cc-switch/<模型 id>`。
pub const PROVIDER_KEY: &str = "cc-switch";

/// models.json 里 `providers.cc-switch` 的内容。
pub fn provider_value(base_url: &str, plan: &OpencodeGatewayPlan) -> Value {
    let models = plan
        .models
        .iter()
        .map(|(id, name)| json!({ "id": id, "name": name }))
        .collect::<Vec<_>>();
    json!({
        "name": "CC Switch",
        "baseUrl": format!("{}/pi/v1", base_url.trim_end_matches('/')),
        "apiKey": "gateway",
        "api": "openai-completions",
        "models": models,
    })
}

/// 写 Pi 的两个文件：provider 节点 + 全局默认。
pub fn apply(base_url: &str, plan: &OpencodeGatewayPlan) -> Result<(), AppError> {
    let value = provider_value(base_url, plan);
    let replaced = pi_config::replace_pi_provider_if_present(PROVIDER_KEY, &value)?;
    if replaced.is_none() {
        pi_config::insert_pi_provider(PROVIDER_KEY, &value)?;
    }
    pi_config::set_pi_native_defaults(PROVIDER_KEY, &plan.default_model)
}

/// 释放：删 provider；默认 provider 还指向网关时一并清掉。
pub fn release() -> Result<(), AppError> {
    let removed = pi_config::remove_pi_provider(PROVIDER_KEY)?;
    if removed.is_some() {
        pi_config::clear_pi_native_defaults_if_provider(PROVIDER_KEY)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pi_config::test_support::TestAgentDir;
    use serial_test::serial;

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
    fn provider_points_to_the_gateway_prefix() {
        let value = provider_value(
            "http://127.0.0.1:15721/",
            &plan(&[("kimi-k3", "Kimi K3")], "kimi-k3"),
        );
        assert_eq!(value["baseUrl"], json!("http://127.0.0.1:15721/pi/v1"));
        assert_eq!(value["api"], json!("openai-completions"));
        assert_eq!(value["models"][0]["id"], json!("kimi-k3"));
        assert_eq!(value["models"][0]["name"], json!("Kimi K3"));
    }

    #[test]
    #[serial]
    fn apply_writes_models_and_defaults_then_release_removes_both() {
        let _agent = TestAgentDir::new();
        apply(
            "http://127.0.0.1:15721",
            &plan(&[("kimi-k3", "Kimi K3")], "kimi-k3"),
        )
        .expect("apply");

        let providers = pi_config::read_pi_native_providers().expect("providers");
        assert_eq!(
            providers[PROVIDER_KEY]["baseUrl"],
            json!("http://127.0.0.1:15721/pi/v1")
        );
        let defaults = pi_config::read_pi_native_defaults().expect("defaults");
        assert_eq!(defaults.default_provider.as_deref(), Some(PROVIDER_KEY));
        assert_eq!(defaults.default_model.as_deref(), Some("kimi-k3"));

        release().expect("release");
        assert!(!pi_config::pi_provider_exists(PROVIDER_KEY).expect("exists"));
        let defaults = pi_config::read_pi_native_defaults().expect("defaults after release");
        assert_eq!(defaults.default_provider, None);
        assert_eq!(defaults.default_model, None);
    }
}
