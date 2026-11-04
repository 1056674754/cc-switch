//! 网关模型目录与每 app 模型配置命令（P1）
//!
//! 目录抬头两条命令：`get_gateway_catalog` 给 UI 选模型；`get/set_app_model_config`
//! 读写每 app 的默认 + 槽位 + 允许列表。目录本身由 provider 行派生，这里不物化。

use crate::app_config::AppType;
use crate::services::gateway_catalog::{self, CatalogViewEntry};
use crate::services::gateway_config::{self, AppModelConfig};
use crate::services::gateway_migration::{self, AppMigration};
use crate::store::AppState;

/// 整份目录：每条 = 模型引用（`<供应商名>/<模型 id>`）+ 展示信息 + 能承载它的端点 + 来源。
#[tauri::command(async)]
pub fn get_gateway_catalog(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<CatalogViewEntry>, String> {
    gateway_catalog::catalog_view_cached(&state.db).map_err(|error| error.to_string())
}

/// 把旧的 Stack（聚合）配置一次性翻译成每 app 模型配置。幂等：已配过的 app 不覆盖。
#[tauri::command(async)]
pub fn migrate_gateway_from_stack(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<AppMigration>, String> {
    gateway_migration::migrate_all(&state.db).map_err(|error| error.to_string())
}

/// 读某 app 的模型配置；没配过返回 `null`。
#[tauri::command(async)]
pub fn get_app_model_config(
    state: tauri::State<'_, AppState>,
    app_type: String,
) -> Result<Option<AppModelConfig>, String> {
    state
        .db
        .get_app_model_config(&app_type)
        .map_err(|error| error.to_string())
}

/// 写某 app 的模型配置（整体覆盖）。引用必须先过目录校验：查不到就报错，不回落。
/// 已经接管（代理已接上）的 app 立刻按新配置重写客户端文件；没接管或直连时是空操作。
#[tauri::command(async)]
pub async fn set_app_model_config(
    state: tauri::State<'_, AppState>,
    app_type: String,
    config: AppModelConfig,
) -> Result<(), String> {
    let catalog =
        gateway_catalog::catalog_view_cached(&state.db).map_err(|error| error.to_string())?;
    gateway_config::validate_app_model_config(&config, &catalog)
        .map_err(|error| error.to_string())?;
    state
        .db
        .set_app_model_config(&app_type, &config)
        .map_err(|error| error.to_string())?;
    if let Ok(app) = app_type.parse::<AppType>() {
        if crate::mode::controller::PROXY_APPS.contains(&app) {
            crate::mode::controller::resync_route(state.inner(), &app)
                .await
                .map_err(|error| format!("配置已保存，但重写客户端失败: {error}"))?;
        }
    }
    Ok(())
}
