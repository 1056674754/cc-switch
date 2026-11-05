// 由 scripts/gen_dev_ipc_adapters.py 生成，请勿手改。
// 源文件：src-tauri/src/commands/auth.rs（共 8 条命令）

use super::*;
#[allow(non_snake_case)]
pub(crate) async fn __ipc_auth_start_login(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_auth_provider: String = __args.required::<String>("authProvider")?;
    let __p_github_domain: Option<String> = __args.option::<String>("githubDomain")?;
    let __p_target_account_id: Option<String> = __args.option::<String>("targetAccountId")?;
    let __p_login_method: Option<String> = __args.option::<String>("loginMethod")?;
    let __p_copilot_state = __args.state::<CopilotAuthState>(__app)?;
    let __p_codex_state = __args.state::<CodexOAuthState>(__app)?;
    let __p_xai_state = __args.state::<XaiOAuthState>(__app)?;
    let __p_agy_state = __args.state::<AgyAccountState>(__app)?;
    let __p_claude_state = __args.state::<ClaudeAccountState>(__app)?;
    let __result = auth_start_login(__p_auth_provider, __p_github_domain, __p_target_account_id, __p_login_method, __p_copilot_state, __p_codex_state, __p_xai_state, __p_agy_state, __p_claude_state).await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_auth_poll_for_account(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_auth_provider: String = __args.required::<String>("authProvider")?;
    let __p_device_code: String = __args.required::<String>("deviceCode")?;
    let __p_github_domain: Option<String> = __args.option::<String>("githubDomain")?;
    let __p_app_state = __args.state::<AppState>(__app)?;
    let __p_copilot_state = __args.state::<CopilotAuthState>(__app)?;
    let __p_codex_state = __args.state::<CodexOAuthState>(__app)?;
    let __p_xai_state = __args.state::<XaiOAuthState>(__app)?;
    let __p_agy_state = __args.state::<AgyAccountState>(__app)?;
    let __p_claude_state = __args.state::<ClaudeAccountState>(__app)?;
    let __result = auth_poll_for_account(__p_auth_provider, __p_device_code, __p_github_domain, __p_app_state, __p_copilot_state, __p_codex_state, __p_xai_state, __p_agy_state, __p_claude_state).await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_auth_cancel_login(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_auth_provider: String = __args.required::<String>("authProvider")?;
    let __p_device_code: String = __args.required::<String>("deviceCode")?;
    let __p_codex_state = __args.state::<CodexOAuthState>(__app)?;
    let __p_agy_state = __args.state::<AgyAccountState>(__app)?;
    let __p_claude_state = __args.state::<ClaudeAccountState>(__app)?;
    let __result = auth_cancel_login(__p_auth_provider, __p_device_code, __p_codex_state, __p_agy_state, __p_claude_state).await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_auth_list_accounts(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_auth_provider: String = __args.required::<String>("authProvider")?;
    let __p_copilot_state = __args.state::<CopilotAuthState>(__app)?;
    let __p_codex_state = __args.state::<CodexOAuthState>(__app)?;
    let __p_xai_state = __args.state::<XaiOAuthState>(__app)?;
    let __p_agy_state = __args.state::<AgyAccountState>(__app)?;
    let __p_claude_state = __args.state::<ClaudeAccountState>(__app)?;
    let __result = auth_list_accounts(__p_auth_provider, __p_copilot_state, __p_codex_state, __p_xai_state, __p_agy_state, __p_claude_state).await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_auth_get_status(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_auth_provider: String = __args.required::<String>("authProvider")?;
    let __p_copilot_state = __args.state::<CopilotAuthState>(__app)?;
    let __p_codex_state = __args.state::<CodexOAuthState>(__app)?;
    let __p_xai_state = __args.state::<XaiOAuthState>(__app)?;
    let __p_agy_state = __args.state::<AgyAccountState>(__app)?;
    let __p_claude_state = __args.state::<ClaudeAccountState>(__app)?;
    let __result = auth_get_status(__p_auth_provider, __p_copilot_state, __p_codex_state, __p_xai_state, __p_agy_state, __p_claude_state).await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_auth_remove_account(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_auth_provider: String = __args.required::<String>("authProvider")?;
    let __p_account_id: String = __args.required::<String>("accountId")?;
    let __p_app_state = __args.state::<AppState>(__app)?;
    let __p_copilot_state = __args.state::<CopilotAuthState>(__app)?;
    let __p_xai_state = __args.state::<XaiOAuthState>(__app)?;
    let __p_agy_state = __args.state::<AgyAccountState>(__app)?;
    let __p_claude_state = __args.state::<ClaudeAccountState>(__app)?;
    let __result = auth_remove_account(__p_auth_provider, __p_account_id, __p_app_state, __p_copilot_state, __p_xai_state, __p_agy_state, __p_claude_state).await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_auth_set_default_account(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_auth_provider: String = __args.required::<String>("authProvider")?;
    let __p_account_id: String = __args.required::<String>("accountId")?;
    let __p_copilot_state = __args.state::<CopilotAuthState>(__app)?;
    let __p_codex_state = __args.state::<CodexOAuthState>(__app)?;
    let __p_xai_state = __args.state::<XaiOAuthState>(__app)?;
    let __p_agy_state = __args.state::<AgyAccountState>(__app)?;
    let __p__claude_state = __args.state::<ClaudeAccountState>(__app)?;
    let __result = auth_set_default_account(__p_auth_provider, __p_account_id, __p_copilot_state, __p_codex_state, __p_xai_state, __p_agy_state, __p__claude_state).await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_auth_logout(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_auth_provider: String = __args.required::<String>("authProvider")?;
    let __p_app_state = __args.state::<AppState>(__app)?;
    let __p_copilot_state = __args.state::<CopilotAuthState>(__app)?;
    let __p_xai_state = __args.state::<XaiOAuthState>(__app)?;
    let __p_agy_state = __args.state::<AgyAccountState>(__app)?;
    let __p_claude_state = __args.state::<ClaudeAccountState>(__app)?;
    let __result = auth_logout(__p_auth_provider, __p_app_state, __p_copilot_state, __p_xai_state, __p_agy_state, __p_claude_state).await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}
