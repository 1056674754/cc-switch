// 由 scripts/gen_dev_ipc_adapters.py 生成，请勿手改。
// 源文件：src-tauri/src/tray_panel.rs（共 5 条命令）

use super::*;
#[allow(non_snake_case)]
pub(crate) async fn __ipc_tray_panel_state(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_app: tauri::AppHandle = __app.clone();
    let __result = tray_panel_state(__p_app).await;
    match __result { Ok(value) => crate::dev_ipc::ok(value), Err(error) => Err(error) }
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_tray_panel_placement(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_app: tauri::AppHandle = __app.clone();
    let __result = tray_panel_placement(__p_app);
    crate::dev_ipc::ok(__result)
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_tray_panel_layout(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_app: tauri::AppHandle = __app.clone();
    let __p_layout: PanelLayout = __args.required::<PanelLayout>("layout")?;
    let __result = tray_panel_layout(__p_app, __p_layout);
    crate::dev_ipc::ok(__result)
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_tray_panel_hide(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_app: tauri::AppHandle = __app.clone();
    let __result = tray_panel_hide(__p_app);
    crate::dev_ipc::ok(__result)
}

#[allow(non_snake_case)]
pub(crate) async fn __ipc_tray_panel_action(__app: &tauri::AppHandle, __args: &mut crate::dev_ipc::Args) -> crate::dev_ipc::Outcome {
    let __p_app: tauri::AppHandle = __app.clone();
    let __p_id: String = __args.required::<String>("id")?;
    let __result = tray_panel_action(__p_app, __p_id).await;
    crate::dev_ipc::ok(__result)
}
