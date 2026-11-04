//! P4-A：把 OMP 的模型来源改成网关。
//!
//! OMP 的 provider 在 `models.yml` 的 `providers` 下，节点形状与 Pi 类似
//! （`baseUrl` / `apiKey` / `api` / `models`）。注意：`settings.yml` 的
//! `modelRoles.default` 是用户自己的选择，CCS 一直不写它（见 `omp_config` 的注释），
//! 所以接管只写 provider，默认模型仍由用户在 OMP 里挑。

use serde_json::{json, Value};

use crate::error::AppError;
use crate::omp_config::{self, Change};
use crate::services::opencode_gateway::OpencodeGatewayPlan;

/// 写进 OMP 的 provider id：默认模型会显示成 `cc-switch/<模型 id>`。
pub const PROVIDER_ID: &str = "cc-switch";

/// `providers.cc-switch` 的内容。
pub fn provider_value(base_url: &str, plan: &OpencodeGatewayPlan) -> Value {
    let models = plan
        .models
        .iter()
        .map(|(id, name)| json!({ "id": id, "name": name }))
        .collect::<Vec<_>>();
    json!({
        "baseUrl": format!("{}/omp/v1", base_url.trim_end_matches('/')),
        "apiKey": "gateway",
        "api": "openai-completions",
        "models": models,
    })
}

/// 写 `models.yml`：存在就替换，不存在就插入。
pub fn apply(base_url: &str, plan: &OpencodeGatewayPlan) -> Result<(), AppError> {
    let value = provider_value(base_url, plan);
    let exists = omp_config::get_providers()?.contains_key(PROVIDER_ID);
    let change = if exists {
        Change::ReplaceIfPresent
    } else {
        Change::Insert
    };
    omp_config::change_and_commit(PROVIDER_ID, Some(&value), change, |_| Ok(()))
}

/// 释放：删掉 `cc-switch` provider（默认模型是用户的选择，不动）。
pub fn release() -> Result<(), AppError> {
    omp_config::change_and_commit(PROVIDER_ID, None, Change::Remove, |_| Ok(()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::omp_config::test_support::TestAgentDir;
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
        assert_eq!(value["baseUrl"], json!("http://127.0.0.1:15721/omp/v1"));
        assert_eq!(value["api"], json!("openai-completions"));
        assert_eq!(value["models"][0]["name"], json!("Kimi K3"));
    }

    #[test]
    #[serial]
    fn apply_writes_the_provider_and_release_removes_it() {
        let _agent = TestAgentDir::new();
        apply(
            "http://127.0.0.1:15721",
            &plan(&[("kimi-k3", "Kimi K3")], "kimi-k3"),
        )
        .expect("apply");
        let providers = omp_config::get_providers().expect("providers");
        assert_eq!(
            providers[PROVIDER_ID]["baseUrl"],
            json!("http://127.0.0.1:15721/omp/v1")
        );

        // 二次 apply 走替换，不报 duplicate
        apply(
            "http://127.0.0.1:15721",
            &plan(&[("kimi-k3", "Kimi K3")], "kimi-k3"),
        )
        .expect("replace");

        release().expect("release");
        assert!(!omp_config::get_providers()
            .expect("providers after release")
            .contains_key(PROVIDER_ID));
    }
}
