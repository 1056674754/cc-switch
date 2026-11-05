// 由 scripts/gen_dev_ipc_adapters.py 生成，请勿手改。
// 源文件：src-tauri/src/commands/profile.rs（共 6 条命令）

use super::*;
#[allow(non_snake_case)]
pub(crate) async fn __ipc_list_profiles(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __result = list_profiles(__p_state).await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_create_profile(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __p_name: String = __args.required::<String>("name")?;
    let __p_scope: String = __args.required::<String>("scope")?;
    let __result = create_profile(__p_state, __p_name, __p_scope);
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_update_profile(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __p_id: String = __args.required::<String>("id")?;
    let __p_name: Option<String> = __args.option::<String>("name")?;
    let __p_resnapshot: Option<bool> = __args.option::<bool>("resnapshot")?;
    let __p_scope: Option<String> = __args.option::<String>("scope")?;
    let __result = update_profile(__p_state, __p_id, __p_name, __p_resnapshot, __p_scope);
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_delete_profile(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __p_id: String = __args.required::<String>("id")?;
    let __result = delete_profile(__p_state, __p_id);
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_clear_current_profile(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __p_scope: String = __args.required::<String>("scope")?;
    let __result = clear_current_profile(__p_state, __p_scope);
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_apply_profile(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_app: tauri::AppHandle = __app.clone();
    let __p_state = __args.state::<AppState>(__app)?;
    let __p_id: String = __args.required::<String>("id")?;
    let __p_scope: String = __args.required::<String>("scope")?;
    let __result = apply_profile(__p_app, __p_state, __p_id, __p_scope);
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}
