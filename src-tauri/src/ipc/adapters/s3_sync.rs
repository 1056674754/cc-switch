// 由 scripts/gen_command_adapters.py 生成，请勿手改。
// 源文件：src-tauri/src/commands/s3_sync.rs（共 5 条命令）

use super::*;
#[allow(non_snake_case)]
pub(crate) async fn __ipc_s3_test_connection(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_settings: S3SyncSettings = __args.required::<S3SyncSettings>("settings")?;
    let __p_preserveEmptyPassword: Option<bool> = __args.option::<bool>("preserveEmptyPassword")?;
    let __result = s3_test_connection(__p_settings, __p_preserveEmptyPassword).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_s3_sync_upload(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __result = s3_sync_upload(__p_state).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_s3_sync_download(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __result = s3_sync_download(__p_state).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_s3_sync_save_settings(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_settings: S3SyncSettings = __args.required::<S3SyncSettings>("settings")?;
    let __p_passwordTouched: Option<bool> = __args.option::<bool>("passwordTouched")?;
    let __result = s3_sync_save_settings(__p_settings, __p_passwordTouched).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_s3_sync_fetch_remote_info(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __result = s3_sync_fetch_remote_info().await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}
