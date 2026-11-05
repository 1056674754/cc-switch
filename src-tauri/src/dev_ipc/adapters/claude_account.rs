// 由 scripts/gen_dev_ipc_adapters.py 生成，请勿手改。
// 源文件：src-tauri/src/commands/claude_account.rs（共 4 条命令）

use super::*;
#[allow(non_snake_case)]
pub(crate) async fn __ipc_claude_account_submit_paste(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_flow_id: String = __args.required::<String>("flowId")?;
    let __p_pasted: String = __args.required::<String>("pasted")?;
    let __p_state = __args.state::<ClaudeAccountState>(__app)?;
    let __result = claude_account_submit_paste(__p_flow_id, __p_pasted, __p_state).await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_get_claude_account_quota(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_app: tauri::AppHandle = __app.clone();
    let __p_app_state = __args.state::<crate::store::AppState>(__app)?;
    let __p_account_id: String = __args.required::<String>("accountId")?;
    let __result = get_claude_account_quota(__p_app, __p_app_state, __p_account_id).await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_get_claude_default_plan(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __result = get_claude_default_plan().await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_get_claude_login_switch(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __result = get_claude_login_switch().await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}
