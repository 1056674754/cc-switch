// 由 scripts/gen_command_adapters.py 生成，请勿手改。
// 源文件：src-tauri/src/commands/lightweight.rs（共 3 条命令）

use super::*;
#[allow(non_snake_case)]
pub(crate) async fn __ipc_enter_lightweight_mode(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_app: tauri::AppHandle = __app.clone();
    let __result = enter_lightweight_mode(__p_app);
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_exit_lightweight_mode(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __p_app: tauri::AppHandle = __app.clone();
    let __result = exit_lightweight_mode(__p_app);
    match __result {
        Ok(value) => crate::ipc::ok(value),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_is_lightweight_mode(
    __app: &tauri::AppHandle,
    __args: &mut crate::ipc::Args,
) -> crate::ipc::Outcome {
    let __result = is_lightweight_mode();
    crate::ipc::ok(__result)
}
