use crate::services::subscription::SubscriptionQuota;

#[tauri::command]
pub async fn get_coding_plan_quota(
    base_url: String,
    api_key: String,
    // 火山方舟用控制面 AK/SK 签名查询用量；其他供应商不传，沿用 api_key。
    access_key_id: Option<String>,
    secret_access_key: Option<String>,
    // 智谱团队版（zhipu_team）靠显式标识路由（base_url 与个人版相同无法区分）。
    coding_plan_provider: Option<String>,
    team_organization_id: Option<String>,
    team_project_id: Option<String>,
) -> Result<SubscriptionQuota, String> {
    crate::services::coding_plan::get_coding_plan_quota(
        &base_url,
        &api_key,
        access_key_id.as_deref(),
        secret_access_key.as_deref(),
        coding_plan_provider.as_deref(),
        team_organization_id.as_deref(),
        team_project_id.as_deref(),
    )
    .await
}

// >>> dev-ipc adapters（生成物：scripts/gen_dev_ipc_adapters.py，勿手改）
#[cfg(feature = "dev-ipc")]
#[allow(clippy::all, dead_code, non_snake_case, unused_imports)]
pub(crate) mod dev_ipc_adapters {
    include!("../dev_ipc/adapters/coding_plan.rs");
}
// <<< dev-ipc adapters
