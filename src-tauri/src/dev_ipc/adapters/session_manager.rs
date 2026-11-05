// 由 scripts/gen_dev_ipc_adapters.py 生成，请勿手改。
// 源文件：src-tauri/src/commands/session_manager.rs（共 10 条命令）

use super::*;
#[allow(non_snake_case)]
pub(crate) async fn __ipc_list_sessions(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __result = list_sessions().await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_get_session_messages(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_providerId: String = __args.required::<String>("providerId")?;
    let __p_sourcePath: String = __args.required::<String>("sourcePath")?;
    let __result = get_session_messages(__p_providerId, __p_sourcePath).await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_stream_session_messages(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let _ = (__app, __args);
    Err(format!("dev-ipc: 命令 `stream_session_messages` 无法在浏览器开发模式下调用：参数里有 tauri::ipc::Channel（流式回调通道），浏览器桥不支持"))
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_get_session_block_content(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_providerId: String = __args.required::<String>("providerId")?;
    let __p_sourcePath: String = __args.required::<String>("sourcePath")?;
    let __p_contentRef: ContentRef = __args.required::<ContentRef>("contentRef")?;
    let __p_offset: Option<u32> = __args.option::<u32>("offset")?;
    let __p_limit: Option<u32> = __args.option::<u32>("limit")?;
    let __result = get_session_block_content(__p_providerId, __p_sourcePath, __p_contentRef, __p_offset, __p_limit).await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_get_session_image(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let _ = (__app, __args);
    Err(format!("dev-ipc: 命令 `get_session_image` 无法在浏览器开发模式下调用：返回值是 tauri::ipc::Response（原始字节流），浏览器桥只支持 JSON"))
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_reveal_session_path(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_path: String = __args.required::<String>("path")?;
    let __result = reveal_session_path(__p_path).await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_export_session_markdown(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_app: tauri::AppHandle = __app.clone();
    let __p_defaultName: String = __args.required::<String>("defaultName")?;
    let __p_content: String = __args.required::<String>("content")?;
    let __result = export_session_markdown(__p_app, __p_defaultName, __p_content).await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_delete_session(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_providerId: String = __args.required::<String>("providerId")?;
    let __p_sessionId: String = __args.required::<String>("sessionId")?;
    let __p_sourcePath: String = __args.required::<String>("sourcePath")?;
    let __result = delete_session(__p_providerId, __p_sessionId, __p_sourcePath).await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_delete_sessions(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_items: Vec<session_manager::DeleteSessionRequest> = __args.required::<Vec<session_manager::DeleteSessionRequest>>("items")?;
    let __result = delete_sessions(__p_items).await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_launch_session_terminal(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_state = __args.state::<crate::store::AppState>(__app)?;
    let __p_command: String = __args.required::<String>("command")?;
    let __p_cwd: Option<String> = __args.option::<String>("cwd")?;
    let __p_custom_config: Option<String> = __args.option::<String>("customConfig")?;
    let __result = launch_session_terminal(__p_state, __p_command, __p_cwd, __p_custom_config).await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}
