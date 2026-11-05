// 由 scripts/gen_command_adapters.py 生成，请勿手改。
// 源文件：src-tauri/src/commands/agy_account.rs（共 2 条命令）

use super::*;
#[allow(non_snake_case)]
pub(crate) async fn __ipc_agy_account_is_running(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __result = agy_account_is_running().await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_agy_account_quota(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_account_id: String = __args.required::<String>("accountId")?;
    let __p_state = __args.state::<AgyAccountState>(__app)?;
    let __result = agy_account_quota(__p_account_id, __p_state).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}
