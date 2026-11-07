// 由 scripts/gen_command_adapters.py 生成，请勿手改。
// 源文件：src-tauri/src/commands/custom_icon.rs（共 4 条命令）

use super::*;
#[allow(non_snake_case)]
pub(crate) async fn __ipc_get_custom_icons(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __result = get_custom_icons(__p_state).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_add_custom_icon(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __p_name: String = __args.required::<String>("name")?;
    let __p_data: String = __args.required::<String>("data")?;
    let __result = add_custom_icon(__p_state, __p_name, __p_data).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_rename_custom_icon(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __p_id: String = __args.required::<String>("id")?;
    let __p_name: String = __args.required::<String>("name")?;
    let __result = rename_custom_icon(__p_state, __p_id, __p_name).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_delete_custom_icon(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_state = __args.state::<AppState>(__app)?;
    let __p_id: String = __args.required::<String>("id")?;
    let __result = delete_custom_icon(__p_state, __p_id).await;
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}
