// 由 scripts/gen_dev_ipc_adapters.py 生成，请勿手改。
// 源文件：src-tauri/src/commands/copilot.rs（共 15 条命令）

use super::*;
#[allow(non_snake_case)]
pub(crate) async fn __ipc_copilot_start_device_flow(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_github_domain: Option<String> = __args.option::<String>("githubDomain")?;
    let __p_state = __args.state::<CopilotAuthState>(__app)?;
    let __result = copilot_start_device_flow(__p_github_domain, __p_state).await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_copilot_poll_for_auth(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_device_code: String = __args.required::<String>("deviceCode")?;
    let __p_github_domain: Option<String> = __args.option::<String>("githubDomain")?;
    let __p_state = __args.state::<CopilotAuthState>(__app)?;
    let __result = copilot_poll_for_auth(__p_device_code, __p_github_domain, __p_state).await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_copilot_poll_for_account(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_device_code: String = __args.required::<String>("deviceCode")?;
    let __p_github_domain: Option<String> = __args.option::<String>("githubDomain")?;
    let __p_state = __args.state::<CopilotAuthState>(__app)?;
    let __result = copilot_poll_for_account(__p_device_code, __p_github_domain, __p_state).await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_copilot_list_accounts(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_state = __args.state::<CopilotAuthState>(__app)?;
    let __result = copilot_list_accounts(__p_state).await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_copilot_remove_account(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_account_id: String = __args.required::<String>("accountId")?;
    let __p_state = __args.state::<CopilotAuthState>(__app)?;
    let __result = copilot_remove_account(__p_account_id, __p_state).await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_copilot_set_default_account(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_account_id: String = __args.required::<String>("accountId")?;
    let __p_state = __args.state::<CopilotAuthState>(__app)?;
    let __result = copilot_set_default_account(__p_account_id, __p_state).await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_copilot_get_auth_status(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_state = __args.state::<CopilotAuthState>(__app)?;
    let __result = copilot_get_auth_status(__p_state).await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_copilot_logout(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_state = __args.state::<CopilotAuthState>(__app)?;
    let __result = copilot_logout(__p_state).await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_copilot_is_authenticated(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_state = __args.state::<CopilotAuthState>(__app)?;
    let __result = copilot_is_authenticated(__p_state).await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_copilot_get_token(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_state = __args.state::<CopilotAuthState>(__app)?;
    let __result = copilot_get_token(__p_state).await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_copilot_get_token_for_account(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_account_id: String = __args.required::<String>("accountId")?;
    let __p_state = __args.state::<CopilotAuthState>(__app)?;
    let __result = copilot_get_token_for_account(__p_account_id, __p_state).await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_copilot_get_models(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_state = __args.state::<CopilotAuthState>(__app)?;
    let __result = copilot_get_models(__p_state).await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_copilot_get_models_for_account(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_account_id: String = __args.required::<String>("accountId")?;
    let __p_state = __args.state::<CopilotAuthState>(__app)?;
    let __result = copilot_get_models_for_account(__p_account_id, __p_state).await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_copilot_get_usage(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_state = __args.state::<CopilotAuthState>(__app)?;
    let __result = copilot_get_usage(__p_state).await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_copilot_get_usage_for_account(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_account_id: String = __args.required::<String>("accountId")?;
    let __p_state = __args.state::<CopilotAuthState>(__app)?;
    let __result = copilot_get_usage_for_account(__p_account_id, __p_state).await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}
