// 由 scripts/gen_command_adapters.py 生成，请勿手改。
// 源文件：src-tauri/src/commands/global_proxy.rs（共 5 条命令）

use super::*;
#[allow(non_snake_case)]
pub(crate) async fn __ipc_get_global_proxy_url(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __result = get_global_proxy_url(__p_state).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_set_global_proxy_url(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __p_url: String = __args.required::<String>("url")?;
    let __result = set_global_proxy_url(__p_state, __p_url).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_test_proxy_url(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_url: String = __args.required::<String>("url")?;
    let __result = test_proxy_url(__p_url).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_get_upstream_proxy_status(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __result = get_upstream_proxy_status();
    crate::ipc::ok(__result)
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_scan_local_proxies(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __result = scan_local_proxies().await;
    crate::ipc::ok(__result)
}
