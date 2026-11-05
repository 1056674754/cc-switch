// 由 scripts/gen_command_adapters.py 生成，请勿手改。
// 源文件：src-tauri/src/commands/deeplink.rs（共 7 条命令）

use super::*;
#[allow(non_snake_case)]
pub(crate) async fn __ipc_parse_deeplink(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_url: String = __args.required::<String>("url")?;
    let __result = parse_deeplink(__p_url);
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_merge_deeplink_config(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_request: DeepLinkImportRequest = __args.required::<DeepLinkImportRequest>("request")?;
    let __result = merge_deeplink_config(__p_request);
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_fetch_deeplink_icon(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_url: String = __args.required::<String>("url")?;
    let __result = fetch_deeplink_icon(__p_url).await;
    crate::ipc::ok(__result)
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_import_from_deeplink(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __p_request: DeepLinkImportRequest = __args.required::<DeepLinkImportRequest>("request")?;
    let __result = import_from_deeplink(__p_state, __p_request);
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_import_from_deeplink_unified(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __p_request: DeepLinkImportRequest = __args.required::<DeepLinkImportRequest>("request")?;
    let __result = import_from_deeplink_unified(__p_state, __p_request).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_peek_pending_deeplink(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __result = peek_pending_deeplink();
    crate::ipc::ok(__result)
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_ack_pending_deeplink(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_id: u64 = __args.required::<u64>("id")?;
    let __result = ack_pending_deeplink(__p_id);
    crate::ipc::ok(__result)
}
