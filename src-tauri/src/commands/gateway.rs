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

/// P4-A：把 OpenCode 指到网关 —— 写一条 `cc-switch` provider，模型用当前配置里的引用。
/// 需要网关已启动：先有网关地址，才写得出 baseURL。
#[tauri::command(async)]
pub async fn adopt_opencode_to_gateway(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let config = state
        .db
        .get_app_model_config("opencode")
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "OpenCode 还没有模型配置，先在应用页保存一次".to_string())?;
    let catalog =
        gateway_catalog::catalog_view_cached(&state.db).map_err(|error| error.to_string())?;
    let plan = crate::services::opencode_gateway::plan(&config, &catalog)
        .map_err(|error| error.to_string())?;
    let status = state
        .proxy_service
        .get_status()
        .await
        .map_err(|error| error.to_string())?;
    if !status.running {
        return Err("网关未启动，先在设置里打开总开关".to_string());
    }
    let origin = crate::services::proxy::proxy_origin(&status.address, status.port);
    crate::services::opencode_gateway::apply(&origin, &plan).map_err(|error| error.to_string())
}

/// P4-A：OpenCode 当前是否已经指向网关（配置里有没有 `cc-switch` provider）。
#[tauri::command(async)]
pub fn get_opencode_gateway_status() -> Result<bool, String> {
    let providers = crate::opencode_config::get_providers().map_err(|error| error.to_string())?;
    Ok(providers.contains_key(crate::services::opencode_gateway::PROVIDER_ID))
}

/// P4-A：释放 OpenCode —— 删掉 `cc-switch` provider。
#[tauri::command(async)]
pub fn release_opencode_from_gateway() -> Result<(), String> {
    crate::services::opencode_gateway::release().map_err(|error| error.to_string())
}

/// P4-A：把 Hermes 指到网关 —— 写一条 `cc-switch` custom provider + 顶层默认模型。
#[tauri::command(async)]
pub async fn adopt_hermes_to_gateway(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let config = state
        .db
        .get_app_model_config("hermes")
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "Hermes 还没有模型配置，先在应用页保存一次".to_string())?;
    let catalog =
        gateway_catalog::catalog_view_cached(&state.db).map_err(|error| error.to_string())?;
    let plan = crate::services::opencode_gateway::plan(&config, &catalog)
        .map_err(|error| error.to_string())?;
    let status = state
        .proxy_service
        .get_status()
        .await
        .map_err(|error| error.to_string())?;
    if !status.running {
        return Err("网关未启动，先在设置里打开总开关".to_string());
    }
    let origin = crate::services::proxy::proxy_origin(&status.address, status.port);
    crate::services::hermes_gateway::apply(&origin, &plan).map_err(|error| error.to_string())
}

/// P4-A：Hermes 当前是否已经指向网关。
#[tauri::command(async)]
pub fn get_hermes_gateway_status() -> Result<bool, String> {
    let provider =
        crate::hermes_config::get_provider(crate::services::hermes_gateway::PROVIDER_NAME)
            .map_err(|error| error.to_string())?;
    Ok(provider.is_some())
}

/// P4-A：释放 Hermes —— 删掉 `cc-switch` custom provider。
#[tauri::command(async)]
pub fn release_hermes_from_gateway() -> Result<(), String> {
    crate::services::hermes_gateway::release().map_err(|error| error.to_string())
}

/// P4-A：把 OpenClaw 指到网关 —— 写一条 provider + 默认模型。
#[tauri::command(async)]
pub async fn adopt_openclaw_to_gateway(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let config = state
        .db
        .get_app_model_config("openclaw")
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "OpenClaw 还没有模型配置，先在应用页保存一次".to_string())?;
    let catalog =
        gateway_catalog::catalog_view_cached(&state.db).map_err(|error| error.to_string())?;
    let plan = crate::services::opencode_gateway::plan(&config, &catalog)
        .map_err(|error| error.to_string())?;
    let status = state
        .proxy_service
        .get_status()
        .await
        .map_err(|error| error.to_string())?;
    if !status.running {
        return Err("网关未启动，先在设置里打开总开关".to_string());
    }
    let origin = crate::services::proxy::proxy_origin(&status.address, status.port);
    crate::services::openclaw_gateway::apply(&origin, &plan).map_err(|error| error.to_string())
}

/// P4-A：OpenClaw 当前是否已经指向网关。
#[tauri::command(async)]
pub fn get_openclaw_gateway_status() -> Result<bool, String> {
    let provider =
        crate::openclaw_config::get_provider(crate::services::openclaw_gateway::PROVIDER_ID)
            .map_err(|error| error.to_string())?;
    Ok(provider.is_some())
}

/// P4-A：释放 OpenClaw —— 删掉 `cc-switch` provider。
#[tauri::command(async)]
pub fn release_openclaw_from_gateway() -> Result<(), String> {
    crate::services::openclaw_gateway::release().map_err(|error| error.to_string())
}

/// P4-A：把 Pi 指到网关 —— 写 models.json 的 provider + settings.json 的默认模型。
#[tauri::command(async)]
pub async fn adopt_pi_to_gateway(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let config = state
        .db
        .get_app_model_config("pi")
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "Pi 还没有模型配置，先在应用页保存一次".to_string())?;
    let catalog =
        gateway_catalog::catalog_view_cached(&state.db).map_err(|error| error.to_string())?;
    let plan = crate::services::opencode_gateway::plan(&config, &catalog)
        .map_err(|error| error.to_string())?;
    let status = state
        .proxy_service
        .get_status()
        .await
        .map_err(|error| error.to_string())?;
    if !status.running {
        return Err("网关未启动，先在设置里打开总开关".to_string());
    }
    let origin = crate::services::proxy::proxy_origin(&status.address, status.port);
    crate::services::pi_gateway::apply(&origin, &plan).map_err(|error| error.to_string())
}

/// P4-A：Pi 当前是否已经指向网关。
#[tauri::command(async)]
pub fn get_pi_gateway_status() -> Result<bool, String> {
    crate::pi_config::pi_provider_exists(crate::services::pi_gateway::PROVIDER_KEY)
        .map_err(|error| error.to_string())
}

/// P4-A：释放 Pi —— 删掉 provider 并清掉指向网关的默认值。
#[tauri::command(async)]
pub fn release_pi_from_gateway() -> Result<(), String> {
    crate::services::pi_gateway::release().map_err(|error| error.to_string())
}

/// P4-A：把 OMP 的模型来源指向网关（写 models.yml 的 cc-switch provider）。
#[tauri::command(async)]
pub async fn adopt_omp_to_gateway(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let config = state
        .db
        .get_app_model_config("omp")
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "OMP 还没有模型配置，先在应用页保存一次".to_string())?;
    let catalog =
        gateway_catalog::catalog_view_cached(&state.db).map_err(|error| error.to_string())?;
    let plan = crate::services::opencode_gateway::plan(&config, &catalog)
        .map_err(|error| error.to_string())?;
    let status = state
        .proxy_service
        .get_status()
        .await
        .map_err(|error| error.to_string())?;
    if !status.running {
        return Err("网关未启动，先在设置里打开总开关".to_string());
    }
    let origin = crate::services::proxy::proxy_origin(&status.address, status.port);
    crate::services::omp_gateway::apply(&origin, &plan).map_err(|error| error.to_string())
}

/// P4-A：OMP 是否已经有 cc-switch provider。
#[tauri::command(async)]
pub fn get_omp_gateway_status() -> Result<bool, String> {
    Ok(crate::omp_config::get_providers()
        .map_err(|error| error.to_string())?
        .contains_key(crate::services::omp_gateway::PROVIDER_ID))
}

/// P4-A：释放 OMP（删 provider，默认模型不动）。
#[tauri::command(async)]
pub fn release_omp_from_gateway() -> Result<(), String> {
    crate::services::omp_gateway::release().map_err(|error| error.to_string())
}

/// P4-A：把 DSH 的模型来源指向网关（写所有 profile 的 cc-switch provider）。
#[tauri::command(async)]
pub async fn adopt_dsh_to_gateway(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let config = state
        .db
        .get_app_model_config("dsh")
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "DSH 还没有模型配置，先在应用页保存一次".to_string())?;
    let catalog =
        gateway_catalog::catalog_view_cached(&state.db).map_err(|error| error.to_string())?;
    let plan = crate::services::opencode_gateway::plan(&config, &catalog)
        .map_err(|error| error.to_string())?;
    let status = state
        .proxy_service
        .get_status()
        .await
        .map_err(|error| error.to_string())?;
    if !status.running {
        return Err("网关未启动，先在设置里打开总开关".to_string());
    }
    let origin = crate::services::proxy::proxy_origin(&status.address, status.port);
    crate::services::dsh_gateway::apply(&origin, &plan).map_err(|error| error.to_string())
}

/// P4-A：DSH 是否已经有 cc-switch provider。
#[tauri::command(async)]
pub fn get_dsh_gateway_status() -> Result<bool, String> {
    crate::dsh_config::provider_key_exists(crate::services::dsh_gateway::PROVIDER_ID)
        .map_err(|error| error.to_string())
}

/// P4-A：释放 DSH（删 provider、清凭据）。
#[tauri::command(async)]
pub fn release_dsh_from_gateway() -> Result<(), String> {
    crate::services::dsh_gateway::release().map_err(|error| error.to_string())
}

/// 供应商卡片里的模型列表用：目录已应用过人工覆盖（隐藏的不出现、显示名已替换），
/// 这里额外给「覆盖本身」，界面才能标出哪些是改过的、并提供还原。
#[tauri::command(async)]
pub fn get_model_overrides(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<gateway_config::ModelOverride>, String> {
    state
        .db
        .list_model_overrides()
        .map_err(|error| error.to_string())
}

/// 写一条人工覆盖：`display_name` 改名（空 = 用派生值），`hidden` = 从目录里隐藏。
/// 隐藏后目录缓存立即失效，各 app 的模型选择器下次读取就看不到它。
#[tauri::command(async)]
pub fn set_model_override(
    state: tauri::State<'_, AppState>,
    upstream_id: String,
    model_id: String,
    display_name: Option<String>,
    hidden: bool,
) -> Result<(), String> {
    state
        .db
        .set_model_override(
            &upstream_id,
            &model_id,
            display_name
                .as_deref()
                .filter(|name| !name.trim().is_empty()),
            hidden,
        )
        .map_err(|error| error.to_string())
}

/// 删掉一条人工覆盖，回到 provider 行派生的值。
#[tauri::command(async)]
pub fn clear_model_override(
    state: tauri::State<'_, AppState>,
    upstream_id: String,
    model_id: String,
) -> Result<(), String> {
    state
        .db
        .clear_model_override(&upstream_id, &model_id)
        .map_err(|error| error.to_string())
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
