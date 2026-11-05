// 由 scripts/gen_dev_ipc_adapters.py 生成，请勿手改。
// 源文件：src-tauri/src/commands/stream_check.rs（共 1 条命令）

use super::*;
#[allow(non_snake_case)]
pub(crate) async fn __ipc_stream_check_provider(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __p_copilot_state = __args.state::<CopilotAuthState>(__app)?;
    let __p_app_type: AppType = __args.required::<AppType>("appType")?;
    let __p_provider_id: String = __args.required::<String>("providerId")?;
    let __result = stream_check_provider(__p_state, __p_copilot_state, __p_app_type, __p_provider_id).await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error.to_string()) }
}
