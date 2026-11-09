//! 网关模型目录与每 app 模型配置命令（P1）
//!
//! 目录抬头两条命令：`get_gateway_catalog` 给 UI 选模型；`get/set_app_model_config`
//! 读写每 app 的默认 + 槽位 + 允许列表。目录本身由 provider 行派生，这里不物化。

use crate::app_config::AppType;
use crate::services::gateway_catalog::{self, CatalogViewEntry};
use crate::services::gateway_config::{self, AppModelConfig};
use crate::services::gateway_migration::{self, AppMigration};
use crate::services::gateway_model_sync::{self, SupplierModelFetch};
use crate::store::AppState;

/// 整份目录：每条 = 模型引用（`<供应商名>/<模型 id>`）+ 展示信息 + 能承载它的端点 + 来源。
///
/// `refresh` 为真时先清掉进程内的目录缓存再派生：界面的「刷新」用它把外部写入
/// （`ccswitch` 命令行、另一个实例、备份恢复）之后仍然命中的旧缓存顶掉。缓存本身
/// 还要留给 Claude Code 的模型发现请求，所以不能直接把这条命令的读路径改成每次重算。
///
/// `includeHidden` 为真时把「人工隐藏」的条目也返回：供应商编辑弹窗的左栏要展示上游
/// 有、但还没启用的模型，才能点「加入」。普通读路径（应用页选模型、路由、发布）必须
/// 保持只看到可见目录，所以默认仍是过滤。
#[tauri::command(async)]
pub fn get_gateway_catalog(
    state: tauri::State<'_, AppState>,
    refresh: Option<bool>,
    include_hidden: Option<bool>,
) -> Result<Vec<CatalogViewEntry>, String> {
    if refresh.unwrap_or(false) {
        gateway_catalog::invalidate_catalog_cache();
    }
    let read = if include_hidden.unwrap_or(false) {
        gateway_catalog::catalog_view_including_hidden(&state.db)
    } else {
        gateway_catalog::catalog_view_cached(&state.db)
    };
    read.map_err(|error| error.to_string())
}

/// 从上游拉模型列表，写进目录的「发现」表。
///
/// 这是供应商弹窗里模型区那个刷新按钮的实现：目录原本只能从 provider 行派生，
/// 用户看不到上游「其实还有哪些模型」。拉到的候选默认隐藏（未启用），点「加入」才放开。
///
/// **供应商就是上游**：同一个上游在库里可能被按 app 派生成多行（Codex 订阅同时有
/// `chatgpt.com` 与官方托管上游行），这里先按真实上游归并，一家只发一次请求，再把结果
/// 写进它对应的每一行——否则就是对着同一家上游重复拉，还各存一套。
#[tauri::command(async)]
pub async fn fetch_supplier_models(
    state: tauri::State<'_, AppState>,
    oauth_state: tauri::State<'_, crate::commands::CodexOAuthState>,
    upstream_ids: Vec<String>,
) -> Result<SupplierModelFetch, String> {
    let (plans, mut errors) = gateway_model_sync::plan_refresh(&state.db, &upstream_ids)
        .map_err(|error| error.to_string())?;
    if plans.is_empty() {
        return Err(errors
            .into_iter()
            .next()
            .unwrap_or_else(|| "没有可刷新的上游".to_string()));
    }

    let mut fetched: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut added: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut refreshed: Vec<String> = Vec::new();
    for plan in plans {
        let models = match fetch_service_models(&plan.source, &oauth_state).await {
            Ok(models) => models,
            Err(error) => {
                errors.push(error);
                continue;
            }
        };
        fetched.extend(models.iter().cloned());
        for upstream_id in &plan.upstream_ids {
            let (_, added_models) =
                gateway_model_sync::store_discovered_models(&state.db, upstream_id, &models)
                    .map_err(|error| error.to_string())?;
            added.extend(added_models);
            refreshed.push(upstream_id.clone());
        }
    }
    if refreshed.is_empty() {
        return Err(errors.join("；"));
    }
    Ok(SupplierModelFetch {
        upstream_ids: refreshed,
        fetched: fetched.len(),
        added: added.len(),
    })
}

/// 按上游凭证形态发请求：自定义网关走 OpenAI 兼容 `/v1/models`，托管 Codex 账号走
/// `chatgpt.com/backend-api/codex/models`（不是兼容端点，要用 OAuth token）。
async fn fetch_service_models(
    source: &gateway_model_sync::FetchSource,
    oauth_state: &tauri::State<'_, crate::commands::CodexOAuthState>,
) -> Result<Vec<String>, String> {
    use crate::services::gateway_model_sync::FetchSource;

    let models = match source {
        FetchSource::Literal(target) => {
            crate::services::model_fetch::fetch_models(
                &target.base_url,
                &target.secret,
                target.is_full_url,
                None,
                None,
                target.api_format,
                None,
            )
            .await?
        }
        FetchSource::CodexOAuth { account_id } => {
            let manager = &oauth_state.0;
            let token = manager
                .get_valid_token_for_account(account_id)
                .await
                .map_err(|error| {
                    format!("Codex OAuth 账号 {account_id} 的 token 不可用：{error}")
                })?;
            let chatgpt_account_id = manager
                .chatgpt_account_id_for_account(account_id)
                .await
                .map_err(|error| {
                    format!("Codex OAuth 账号 {account_id} 缺少工作区信息：{error}")
                })?;
            crate::services::codex_oauth_models::fetch_models_with_token(
                &token,
                &chatgpt_account_id,
            )
            .await
            .map_err(|error| format!("从 ChatGPT Codex 获取模型失败：{error}"))?
        }
    };
    Ok(models.into_iter().map(|model| model.id).collect())
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
/// 迁移完（或本来就已经配过）让配置接管客户端：Claude Desktop 的旧聚合名单不再发布。
#[tauri::command(async)]
pub async fn migrate_gateway_from_stack(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<AppMigration>, String> {
    let migrations =
        gateway_migration::migrate_all(&state.db).map_err(|error| error.to_string())?;
    for migration in &migrations {
        if let Ok(app) = migration.app.parse::<AppType>() {
            crate::mode::controller::retire_stack_for_app_config(state.inner(), &app)
                .await
                .map_err(|error| error.to_string())?;
        }
    }
    Ok(migrations)
}

/// 读某 app 的模型配置；没配过返回 `null`。
///
/// 读之前先修一遍过期的 `供应商/模型` 引用：供应商改名 / 合并后，老引用在目录里查不到，
/// 界面会满屏「模型不在目录里」，代理发布时也会拿不到模型名。修正按模型 id 唯一匹配，幂等。
#[tauri::command(async)]
pub fn get_app_model_config(
    state: tauri::State<'_, AppState>,
    app_type: String,
) -> Result<Option<AppModelConfig>, String> {
    gateway_config::repair_app_model_config(&state.db, &app_type)
        .map_err(|error| error.to_string())?;
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
    // 旧客户端可能还带着供应商改名前的引用：能唯一对上就改写，再按目录严格校验。
    let mut config = config;
    gateway_config::remap_stale_references(&mut config, &catalog);
    gateway_config::validate_app_model_config(&config, &catalog)
        .map_err(|error| error.to_string())?;
    state
        .db
        .set_app_model_config(&app_type, &config)
        .map_err(|error| error.to_string())?;
    if let Ok(app) = app_type.parse::<AppType>() {
        if crate::mode::controller::PROXY_APPS.contains(&app) {
            // 配置是权威：还在旧聚合模式的应用先退出聚合，再按新配置重写客户端。
            crate::mode::controller::retire_stack_for_app_config(state.inner(), &app)
                .await
                .map_err(|error| format!("配置已保存，但退出聚合模式失败: {error}"))?;
            crate::mode::controller::resync_route(state.inner(), &app)
                .await
                .map_err(|error| format!("配置已保存，但重写客户端失败: {error}"))?;
        }
    }
    Ok(())
}

// >>> command adapters（生成物：scripts/gen_command_adapters.py，勿手改）
#[allow(clippy::all, dead_code, non_snake_case, unused_imports)]
pub(crate) mod ipc_adapters {
    include!("../ipc/adapters/gateway.rs");
}
// <<< command adapters
