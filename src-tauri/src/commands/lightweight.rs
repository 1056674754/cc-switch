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

// >>> command adapters（生成物：scripts/gen_command_adapters.py，勿手改）
#[allow(clippy::all, dead_code, non_snake_case, unused_imports)]
pub(crate) mod ipc_adapters {
    include!("../ipc/adapters/lightweight.rs");
}
// <<< command adapters
