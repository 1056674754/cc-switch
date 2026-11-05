// 由 scripts/gen_command_adapters.py 生成，请勿手改。
// 源文件：src-tauri/src/commands/mcp.rs（共 17 条命令）

use super::*;
#[allow(non_snake_case)]
pub(crate) async fn __ipc_get_claude_mcp_status(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __result = get_claude_mcp_status().await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_read_claude_mcp_config(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __result = read_claude_mcp_config().await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_upsert_claude_mcp_server(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_id: String = __args.required::<String>("id")?;
    let __p_spec: serde_json::Value = __args.required::<serde_json::Value>("spec")?;
    let __result = upsert_claude_mcp_server(__p_id, __p_spec).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_delete_claude_mcp_server(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_id: String = __args.required::<String>("id")?;
    let __result = delete_claude_mcp_server(__p_id).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_validate_mcp_command(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_cmd: String = __args.required::<String>("cmd")?;
    let __result = validate_mcp_command(__p_cmd).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_get_mcp_config(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __p_app: String = __args.required::<String>("app")?;
    let __result = get_mcp_config(__p_state, __p_app).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_upsert_mcp_server_in_config(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __p_app: String = __args.required::<String>("app")?;
    let __p_id: String = __args.required::<String>("id")?;
    let __p_spec: serde_json::Value = __args.required::<serde_json::Value>("spec")?;
    let __p_sync_other_side: Option<bool> = __args.option::<bool>("syncOtherSide")?;
    let __result =
        upsert_mcp_server_in_config(__p_state, __p_app, __p_id, __p_spec, __p_sync_other_side)
            .await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_delete_mcp_server_in_config(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __p__app: String = __args.required::<String>("App")?;
    let __p_id: String = __args.required::<String>("id")?;
    let __result = delete_mcp_server_in_config(__p_state, __p__app, __p_id).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_set_mcp_enabled(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __p_app: String = __args.required::<String>("app")?;
    let __p_id: String = __args.required::<String>("id")?;
    let __p_enabled: bool = __args.required::<bool>("enabled")?;
    let __result = set_mcp_enabled(__p_state, __p_app, __p_id, __p_enabled).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_get_mcp_servers(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __result = get_mcp_servers(__p_state).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_start_mcp_connectivity_checks(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let _ = (__app, __args);
    Err(format!("ipc: 命令 `start_mcp_connectivity_checks` 无法在浏览器开发模式下调用：参数里有 tauri::ipc::Channel（流式回调通道），浏览器桥不支持"))
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_cancel_mcp_connectivity_checks(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_batch_id: String = __args.required::<String>("batchId")?;
    let __result = cancel_mcp_connectivity_checks(__p_batch_id);
    crate::ipc::ok(__result)
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_upsert_mcp_server(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __p_server: McpServer = __args.required::<McpServer>("server")?;
    let __result = upsert_mcp_server(__p_state, __p_server).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_delete_mcp_server(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __p_id: String = __args.required::<String>("id")?;
    let __result = delete_mcp_server(__p_state, __p_id).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_toggle_mcp_app(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __p_server_id: String = __args.required::<String>("serverId")?;
    let __p_app: String = __args.required::<String>("app")?;
    let __p_enabled: bool = __args.required::<bool>("enabled")?;
    let __result = toggle_mcp_app(__p_state, __p_server_id, __p_app, __p_enabled).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_import_mcp_from_apps(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __result = import_mcp_from_apps(__p_state).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_resync_mcp_to_apps(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __p_apps: Option<Vec<String>> = __args.option::<Vec<String>>("apps")?;
    let __result = resync_mcp_to_apps(__p_state, __p_apps).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}
