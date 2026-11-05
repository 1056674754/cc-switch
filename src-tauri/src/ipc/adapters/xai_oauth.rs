// 由 scripts/gen_command_adapters.py 生成，请勿手改。
// 源文件：src-tauri/src/commands/xai_oauth.rs（共 2 条命令）

use super::*;
#[allow(non_snake_case)]
pub(crate) async fn __ipc_get_xai_oauth_models(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_account_id: Option<String> = __args.option::<String>("accountId")?;
    let __p_state = __args.state::<XaiOAuthState>(__app)?;
    let __result = get_xai_oauth_models(__p_account_id, __p_state).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_get_xai_oauth_quota(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_account_id: Option<String> = __args.option::<String>("accountId")?;
    let __p_state = __args.state::<XaiOAuthState>(__app)?;
    let __result = get_xai_oauth_quota(__p_account_id, __p_state).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}
