// 由 scripts/gen_dev_ipc_adapters.py 生成，请勿手改。
// 源文件：src-tauri/src/commands/coding_plan.rs（共 1 条命令）

use super::*;
#[allow(non_snake_case)]
pub(crate) async fn __ipc_get_coding_plan_quota(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_base_url: String = __args.required::<String>("baseUrl")?;
    let __p_api_key: String = __args.required::<String>("apiKey")?;
    let __p_access_key_id: Option<String> = __args.option::<String>("accessKeyId")?;
    let __p_secret_access_key: Option<String> = __args.option::<String>("secretAccessKey")?;
    let __p_coding_plan_provider: Option<String> = __args.option::<String>("codingPlanProvider")?;
    let __p_team_organization_id: Option<String> = __args.option::<String>("teamOrganizationId")?;
    let __p_team_project_id: Option<String> = __args.option::<String>("teamProjectId")?;
    let __result = get_coding_plan_quota(__p_base_url, __p_api_key, __p_access_key_id, __p_secret_access_key, __p_coding_plan_provider, __p_team_organization_id, __p_team_project_id).await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}
