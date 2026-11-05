// 由 scripts/gen_dev_ipc_adapters.py 生成，请勿手改。
// 源文件：src-tauri/src/commands/misc.rs（共 15 条命令）

use super::*;
#[allow(non_snake_case)]
pub(crate) async fn __ipc_open_external(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_app: tauri::AppHandle = __app.clone();
    let __p_url: String = __args.required::<String>("url")?;
    let __result = open_external(__p_app, __p_url).await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_get_init_error(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __result = get_init_error().await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_get_migration_result(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __result = get_migration_result().await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_get_skills_migration_result(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __result = get_skills_migration_result().await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_check_for_updates(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_handle: tauri::AppHandle = __app.clone();
    let __result = check_for_updates(__p_handle).await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_open_portable_update_download(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_handle: tauri::AppHandle = __app.clone();
    let __p_version: String = __args.required::<String>("version")?;
    let __result = open_portable_update_download(__p_handle, __p_version).await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_is_portable_mode(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __result = is_portable_mode().await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_copy_text_to_clipboard(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_text: String = __args.required::<String>("text")?;
    let __result = copy_text_to_clipboard(__p_text).await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_get_tool_versions(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_tools: Option<Vec<String>> = __args.option::<Vec<String>>("tools")?;
    let __p_wsl_shell_by_tool: Option<HashMap<String, WslShellPreferenceInput>> = __args.option::<HashMap<String, WslShellPreferenceInput>>("wslShellByTool")?;
    let __result = get_tool_versions(__p_tools, __p_wsl_shell_by_tool).await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_run_tool_lifecycle_action(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_tools: Vec<String> = __args.required::<Vec<String>>("tools")?;
    let __p_action: String = __args.required::<String>("action")?;
    let __p_wsl_shell_by_tool: Option<HashMap<String, WslShellPreferenceInput>> = __args.option::<HashMap<String, WslShellPreferenceInput>>("wslShellByTool")?;
    let __p_uninstall_plan: Option<ToolUninstallPlan> = __args.option::<ToolUninstallPlan>("uninstallPlan")?;
    let __result = run_tool_lifecycle_action(__p_tools, __p_action, __p_wsl_shell_by_tool, __p_uninstall_plan).await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_prepare_tool_uninstall(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_tool: String = __args.required::<String>("tool")?;
    let __p_wsl_shell_by_tool: Option<HashMap<String, WslShellPreferenceInput>> = __args.option::<HashMap<String, WslShellPreferenceInput>>("wslShellByTool")?;
    let __result = prepare_tool_uninstall(__p_tool, __p_wsl_shell_by_tool).await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_probe_tool_installations(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_tools: Vec<String> = __args.required::<Vec<String>>("tools")?;
    let __result = probe_tool_installations(__p_tools).await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_list_tool_installations(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_tools: Vec<String> = __args.required::<Vec<String>>("tools")?;
    let __result = list_tool_installations(__p_tools).await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_open_provider_terminal(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_state = __args.state::<crate::store::AppState>(__app)?;
    let __p_agy_accounts = __args.state::<crate::commands::AgyAccountState>(__app)?;
    let __p_app: String = __args.required::<String>("app")?;
    let __p_providerId: String = __args.required::<String>("providerId")?;
    let __p_cwd: Option<String> = __args.option::<String>("cwd")?;
    let __p_skipPermissions: Option<bool> = __args.option::<bool>("skipPermissions")?;
    let __p_extraArgs: Option<String> = __args.option::<String>("extraArgs")?;
    let __result = open_provider_terminal(__p_state, __p_agy_accounts, __p_app, __p_providerId, __p_cwd, __p_skipPermissions, __p_extraArgs).await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_set_window_theme(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let _ = (__app, __args);
    crate::dev_ipc::ok(serde_json::Value::Null)
}
