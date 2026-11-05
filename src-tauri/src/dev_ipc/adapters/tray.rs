// 由 scripts/gen_dev_ipc_adapters.py 生成，请勿手改。
// 源文件：src-tauri/src/tray.rs（共 2 条命令）

use super::*;
#[allow(non_snake_case)]
pub(crate) async fn __ipc_take_tray_navigation(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __result = take_tray_navigation();
    crate::dev_ipc::ok(__result)
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_tray_app_page_seen(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_app: tauri::AppHandle = __app.clone();
    let __p_app_type: String = __args.required::<String>("appType")?;
    let __result = tray_app_page_seen(__p_app, __p_app_type);
    crate::dev_ipc::ok(__result)
}
