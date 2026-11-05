// 由 scripts/gen_dev_ipc_adapters.py 生成，请勿手改。
// 源文件：src-tauri/src/commands/hermes.rs（共 10 条命令）

use super::*;
#[allow(non_snake_case)]
pub(crate) async fn __ipc_import_hermes_providers_from_live(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __result = import_hermes_providers_from_live(__p_state);
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_get_hermes_live_provider_ids(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __result = get_hermes_live_provider_ids();
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_get_hermes_live_provider(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_providerId: String = __args.required::<String>("providerId")?;
    let __result = get_hermes_live_provider(__p_providerId);
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_get_hermes_model_config(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __result = get_hermes_model_config();
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_open_hermes_web_ui(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_app: tauri::AppHandle = __app.clone();
    let __p_path: Option<String> = __args.option::<String>("path")?;
    let __result = open_hermes_web_ui(__p_app, __p_path).await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_launch_hermes_dashboard(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __result = launch_hermes_dashboard().await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_get_hermes_memory(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_kind: hermes_config::MemoryKind = __args.required::<hermes_config::MemoryKind>("kind")?;
    let __result = get_hermes_memory(__p_kind);
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_set_hermes_memory(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_kind: hermes_config::MemoryKind = __args.required::<hermes_config::MemoryKind>("kind")?;
    let __p_content: String = __args.required::<String>("content")?;
    let __result = set_hermes_memory(__p_kind, __p_content);
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_get_hermes_memory_limits(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __result = get_hermes_memory_limits();
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_set_hermes_memory_enabled(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_kind: hermes_config::MemoryKind = __args.required::<hermes_config::MemoryKind>("kind")?;
    let __p_enabled: bool = __args.required::<bool>("enabled")?;
    let __result = set_hermes_memory_enabled(__p_kind, __p_enabled);
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}
