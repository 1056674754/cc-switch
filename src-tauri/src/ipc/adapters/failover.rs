// 由 scripts/gen_command_adapters.py 生成，请勿手改。
// 源文件：src-tauri/src/commands/failover.rs（共 6 条命令）

use super::*;
#[allow(non_snake_case)]
pub(crate) async fn __ipc_get_failover_queue(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __p_app_type: String = __args.required::<String>("appType")?;
    let __result = get_failover_queue(__p_state, __p_app_type).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_get_available_providers_for_failover(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __p_app_type: String = __args.required::<String>("appType")?;
    let __result = get_available_providers_for_failover(__p_state, __p_app_type).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_add_to_failover_queue(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_app: tauri::AppHandle = __app.clone();
    let __p_state = __args.state::<AppState>(__app)?;
    let __p_app_type: String = __args.required::<String>("appType")?;
    let __p_provider_id: String = __args.required::<String>("providerId")?;
    let __result = add_to_failover_queue(__p_app, __p_state, __p_app_type, __p_provider_id).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_remove_from_failover_queue(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_app: tauri::AppHandle = __app.clone();
    let __p_state = __args.state::<AppState>(__app)?;
    let __p_app_type: String = __args.required::<String>("appType")?;
    let __p_provider_id: String = __args.required::<String>("providerId")?;
    let __result =
        remove_from_failover_queue(__p_app, __p_state, __p_app_type, __p_provider_id).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_get_auto_failover_enabled(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __p_app_type: String = __args.required::<String>("appType")?;
    let __result = get_auto_failover_enabled(__p_state, __p_app_type).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_set_auto_failover_enabled(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_app: tauri::AppHandle = __app.clone();
    let __p_state = __args.state::<AppState>(__app)?;
    let __p_app_type: String = __args.required::<String>("appType")?;
    let __p_enabled: bool = __args.required::<bool>("enabled")?;
    let __result = set_auto_failover_enabled(__p_app, __p_state, __p_app_type, __p_enabled).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}
