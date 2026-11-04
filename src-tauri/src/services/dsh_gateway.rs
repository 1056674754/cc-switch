//! P4-A：把 DeepSeek Harness 的模型来源改成网关。
//!
//! DSH 的 provider 存在每个 profile 的 `cordis.patch.yml`（`llm-pi-ai` 下），凭据单独存
//! `credentials`；`dsh_config::save_and_commit` 已经做了多 profile 同步 + 凭据改写 + 冲突
//! 检查，这里只负责组装节点。默认模型由 DSH 自己选，CCS 不写。

use serde_json::{json, Map, Value};

use crate::dsh_config;
use crate::error::AppError;
use crate::services::opencode_gateway::OpencodeGatewayPlan;

/// 写进 DSH 的 provider id（小写连字符，`deepseek-official` 保留）。
pub const PROVIDER_ID: &str = "cc-switch";

/// `llm-pi-ai.config.providers.cc-switch` 的内容。
pub fn provider_value(base_url: &str, plan: &OpencodeGatewayPlan) -> Value {
    let models = plan
        .models
        .iter()
        .map(|(id, name)| (id.clone(), json!({ "name": name })))
        .collect::<Map<_, _>>();
    json!({
        "displayName": "CC Switch",
        "api": "openai-completions",
        "options": {
            "baseURL": format!("{}/dsh/v1", base_url.trim_end_matches('/')),
            "apiKey": "gateway",
        },
        "models": models,
    })
}

/// 写所有 target profile：不存在就建，存在就替换。
pub fn apply(base_url: &str, plan: &OpencodeGatewayPlan) -> Result<(), AppError> {
    let create = !dsh_config::provider_key_exists(PROVIDER_ID)?;
    dsh_config::save_and_commit(
        PROVIDER_ID,
        Some(provider_value(base_url, plan)),
        create,
        || Ok(()),
    )
}

/// 释放：删掉所有 profile 里的 `cc-switch` 节点（凭据一并清理）。
pub fn release() -> Result<(), AppError> {
    dsh_config::remove_provider(PROVIDER_ID)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::fs;
    use std::path::Path;

    struct DshHome {
        dir: tempfile::TempDir,
        previous: Option<std::ffi::OsString>,
    }
    impl DshHome {
        fn new() -> Self {
            let dir = tempfile::tempdir().unwrap();
            let previous = std::env::var_os("DSH_HOME");
            std::env::set_var("DSH_HOME", dir.path());
            Self { dir, previous }
        }
    }
    impl Drop for DshHome {
        fn drop(&mut self) {
            match self.previous.take() {
                Some(value) => std::env::set_var("DSH_HOME", value),
                None => std::env::remove_var("DSH_HOME"),
            }
        }
    }

    const SEED: &str = "---\n- id: llm-pi-ai\n  config:\n    providers: {}\n";

    fn init_profiles(home: &Path, names: &[&str]) {
        for name in names {
            let path = home.join("profiles").join(name).join("cordis.patch.yml");
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, SEED).unwrap();
        }
    }

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
        let value = provider_value(
            "http://127.0.0.1:15721/",
            &plan(&[("kimi-k3", "Kimi K3")], "kimi-k3"),
        );
        assert_eq!(value["api"], json!("openai-completions"));
        assert_eq!(
            value["options"]["baseURL"],
            json!("http://127.0.0.1:15721/dsh/v1")
        );
        assert_eq!(value["models"]["kimi-k3"]["name"], json!("Kimi K3"));
    }

    #[test]
    #[serial]
    fn apply_writes_every_profile_and_release_removes_it() {
        let home = DshHome::new();
        init_profiles(home.dir.path(), &["web", "desktop"]);
        apply(
            "http://127.0.0.1:15721",
            &plan(&[("kimi-k3", "Kimi K3")], "kimi-k3"),
        )
        .expect("apply");

        let providers = dsh_config::get_providers().expect("providers");
        assert_eq!(
            providers[PROVIDER_ID]["options"]["baseURL"],
            json!("http://127.0.0.1:15721/dsh/v1")
        );
        for name in ["web", "desktop"] {
            let text = fs::read_to_string(
                home.dir
                    .path()
                    .join("profiles")
                    .join(name)
                    .join("cordis.patch.yml"),
            )
            .unwrap();
            assert!(text.contains("cc-switch"), "{name}: {text}");
        }

        // 二次 apply 走替换，不报 already exists
        apply(
            "http://127.0.0.1:15721",
            &plan(&[("kimi-k3", "Kimi K3")], "kimi-k3"),
        )
        .expect("replace");

        release().expect("release");
        assert!(!dsh_config::get_providers()
            .expect("providers after release")
            .contains_key(PROVIDER_ID));
    }
}
