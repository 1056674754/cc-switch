#[tauri::command]
pub fn enter_lightweight_mode(app: tauri::AppHandle) -> Result<(), String> {
    crate::lightweight::enter_lightweight_mode(&app)
}

#[tauri::command]
pub fn exit_lightweight_mode(app: tauri::AppHandle) -> Result<(), String> {
    crate::lightweight::exit_lightweight_mode(&app)
}

#[tauri::command]
pub fn is_lightweight_mode() -> bool {
    crate::lightweight::is_lightweight_mode()
}

// >>> dev-ipc adapters（生成物：scripts/gen_dev_ipc_adapters.py，勿手改）
#[cfg(feature = "dev-ipc")]
#[allow(clippy::all, dead_code, non_snake_case, unused_imports)]
pub(crate) mod dev_ipc_adapters {
    include!("../dev_ipc/adapters/lightweight.rs");
}
// <<< dev-ipc adapters
