//! 「我的图标」：用户上传过的自定义图标库

use tauri::State;

use crate::database::CustomIconEntry;
use crate::services::provider::icon::{self, AddLibraryIconResult};
use crate::store::AppState;

#[tauri::command(async)]
pub fn get_custom_icons(state: State<'_, AppState>) -> Result<Vec<CustomIconEntry>, String> {
    icon::list_library_icons(&state.db).map_err(|e| e.to_string())
}

#[tauri::command(async)]
pub fn add_custom_icon(
    state: State<'_, AppState>,
    name: String,
    data: String,
) -> Result<AddLibraryIconResult, String> {
    icon::add_library_icon(&state.db, &name, &data).map_err(|e| e.to_string())
}

#[tauri::command(async)]
pub fn rename_custom_icon(
    state: State<'_, AppState>,
    id: String,
    name: String,
) -> Result<CustomIconEntry, String> {
    icon::rename_library_icon(&state.db, &id, &name).map_err(|e| e.to_string())
}

#[tauri::command(async)]
pub fn delete_custom_icon(state: State<'_, AppState>, id: String) -> Result<(), String> {
    icon::delete_library_icon(&state.db, &id).map_err(|e| e.to_string())
}

// >>> dev-ipc adapters（生成物：scripts/gen_dev_ipc_adapters.py，勿手改）
#[cfg(feature = "dev-ipc")]
#[allow(clippy::all, dead_code, non_snake_case, unused_imports)]
pub(crate) mod dev_ipc_adapters {
    include!("../dev_ipc/adapters/custom_icon.rs");
}
// <<< dev-ipc adapters
