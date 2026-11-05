// 由 scripts/gen_dev_ipc_adapters.py 生成，请勿手改。
// 源文件：src-tauri/src/commands/webdav_sync.rs（共 6 条命令）

use super::*;
#[allow(non_snake_case)]
pub(crate) async fn __ipc_webdav_test_connection(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_settings: WebDavSyncSettings = __args.required::<WebDavSyncSettings>("settings")?;
    let __p_preserveEmptyPassword: Option<bool> = __args.option::<bool>("preserveEmptyPassword")?;
    let __result = webdav_test_connection(__p_settings, __p_preserveEmptyPassword).await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_webdav_sync_upload(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __result = webdav_sync_upload(__p_state).await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_webdav_sync_download(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __result = webdav_sync_download(__p_state).await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_webdav_sync_save_settings(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_settings: WebDavSyncSettings = __args.required::<WebDavSyncSettings>("settings")?;
    let __p_passwordTouched: Option<bool> = __args.option::<bool>("passwordTouched")?;
    let __result = webdav_sync_save_settings(__p_settings, __p_passwordTouched).await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_webdav_sync_clear_settings(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __result = webdav_sync_clear_settings().await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_webdav_sync_fetch_remote_info(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __result = webdav_sync_fetch_remote_info().await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}
