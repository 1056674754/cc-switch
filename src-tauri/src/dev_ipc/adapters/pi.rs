// 由 scripts/gen_dev_ipc_adapters.py 生成，请勿手改。
// 源文件：src-tauri/src/commands/pi.rs（共 3 条命令）

use super::*;
#[allow(non_snake_case)]
pub(crate) async fn __ipc_get_pi_current_state(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __result = get_pi_current_state(__p_state);
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_update_pi_provider_usage_script(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __p_app: String = __args.required::<String>("app")?;
    let __p_id: String = __args.required::<String>("id")?;
    let __p_usageScript: UsageScript = __args.required::<UsageScript>("usageScript")?;
    let __result = update_pi_provider_usage_script(__p_state, __p_app, __p_id, __p_usageScript);
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_get_pi_session_discovery(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __result = get_pi_session_discovery();
    crate::dev_ipc::ok(__result)
}
