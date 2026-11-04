//! 网关模型目录命令（P1）
//!
//! 目录的推导在 `services::gateway_catalog`：只调用现有推导函数，不重写规则。

use crate::services::gateway_catalog::{self, CatalogViewEntry};
use crate::store::AppState;

/// 整份目录：每条 = 模型引用（`<供应商名>/<模型 id>`）+ 展示信息 + 能承载它的端点 + 来源。
#[tauri::command(async)]
pub fn get_gateway_catalog(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<CatalogViewEntry>, String> {
    gateway_catalog::catalog_view(&state.db).map_err(|error| error.to_string())
}
