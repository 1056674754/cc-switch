//! 网关模型目录命令（P1）
//!
//! 目录的推导在 `services::gateway_catalog`：只调用现有推导函数，不重写规则。

use crate::services::gateway_catalog::{self, CatalogEntry};
use crate::store::AppState;

/// 整份目录：每条 = 上游 + 规范模型名 + 展示信息 + 来源。
#[tauri::command(async)]
pub fn get_gateway_catalog(state: tauri::State<'_, AppState>) -> Result<Vec<CatalogEntry>, String> {
    gateway_catalog::derive_entries(&state.db).map_err(|error| error.to_string())
}
