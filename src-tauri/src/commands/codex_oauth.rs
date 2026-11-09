//! Codex OAuth Tauri Commands
//!
//! 提供 OpenAI ChatGPT Plus/Pro OAuth 认证相关的 Tauri 命令。
//!
//! 大部分认证命令通过通用 `auth_*` 命令（参见 `commands::auth`）暴露给前端，
//! 此处定义 State wrapper 以及 Codex OAuth 专属的订阅额度和模型列表查询命令。

use crate::proxy::providers::codex_oauth_auth::{CodexOAuthError, CodexOAuthManager};
use crate::services::model_fetch::FetchedModel;
use crate::services::subscription::{query_codex_quota, CredentialStatus, SubscriptionQuota};
use std::sync::Arc;
use tauri::State;

/// Codex OAuth 认证状态
///
/// `CodexOAuthManager` 内部已使用细粒度锁且所有方法均为 `&self`，因此这里
/// 直接持有 `Arc`，不再包一层 `RwLock`——避免任一命令持有粗粒度锁跨网络刷新
/// 时阻塞其他命令（切换 / 认证中心操作 / token 读取）。
pub struct CodexOAuthState(pub Arc<CodexOAuthManager>);

/// 查询 Codex OAuth (ChatGPT Plus/Pro) 订阅额度
///
/// - `account_id` 未指定时回退到 `CodexOAuthManager` 的默认账号
/// - 没有任何账号时返回 `not_found`，前端 `SubscriptionQuotaView` 会静默不渲染
/// - 复用 `services::subscription::query_codex_quota`，因此 wham/usage 端点协议
///   与 Codex CLI 路径完全一致
#[tauri::command(rename_all = "camelCase")]
pub async fn get_codex_oauth_quota(
    app: tauri::AppHandle,
    app_state: State<'_, crate::store::AppState>,
    account_id: Option<String>,
    state: State<'_, CodexOAuthState>,
) -> Result<SubscriptionQuota, String> {
    let manager = &state.0;

    // 解析最终使用的账号 ID：显式 > 默认账号 > 无账号 (not_found)
    let resolved = match account_id {
        Some(id) => Some(id.trim().to_string()),
        None => manager.default_account_id().await,
    };
    let Some(id) = resolved else {
        return Ok(SubscriptionQuota::not_found("codex_oauth"));
    };

    let result = query_codex_oauth_quota_for(manager, &id).await;
    // Cache by the resolved account, even if the default/binding changes while
    // the request is in flight. Transport errors retain the last good snapshot;
    // authentication/HTTP failures replace it so the tray hides invalid quotas.
    if let Ok(quota) = &result {
        app_state.usage_cache.put_codex_oauth(id, quota.clone());
        crate::tray::schedule_tray_refresh(&app);
    }
    result
}

async fn query_codex_oauth_quota_for(
    manager: &CodexOAuthManager,
    id: &str,
) -> Result<SubscriptionQuota, String> {
    // 获取（必要时自动刷新）access_token
    let token = match manager.get_valid_token_for_account(id).await {
        Ok(t) => t,
        // 网络 / 刷新端点暂时失败不代表登录失效：以 Err 传播，前端 retry 并保留
        // 上次成功值，托盘缓存也不被覆盖；只有确定的凭据问题才标成过期。
        Err(e) if is_transient_token_error(&e) => {
            return Err(format!("Codex OAuth token unavailable: {e}"));
        }
        Err(e) => {
            return Ok(SubscriptionQuota::error(
                "codex_oauth",
                CredentialStatus::Expired,
                format!("Codex OAuth token unavailable: {e}"),
            ));
        }
    };
    let chatgpt_account_id = manager
        .chatgpt_account_id_for_account(id)
        .await
        .map_err(|e| e.to_string())?;

    // 瞬时传输失败以 Err 传播（前端 reject → retry + 保留上次成功值）。
    query_codex_quota(
        &token,
        Some(&chatgpt_account_id),
        "codex_oauth",
        "Codex OAuth access token expired or rejected. Please re-login via cc-switch.",
    )
    .await
}

/// 用掉一张存下的 ChatGPT 重置卡：清空已用完的额度窗口（**不可恢复**）
///
/// - 只作用于 `account_id` 这个账号；token 过期时用刷新令牌换新的
/// - 按最早到期挑一张卡，和界面里「重置 剩余 N 次」的顺序一致
/// - 成功返回清掉的窗口数；前端随后会重查额度
#[tauri::command(rename_all = "camelCase")]
pub async fn consume_codex_reset_credit(
    app: tauri::AppHandle,
    account_id: String,
    state: State<'_, CodexOAuthState>,
) -> Result<crate::services::subscription::CodexResetCreditConsumed, String> {
    let manager = &state.0;
    let id = account_id.trim();
    if id.is_empty() {
        return Err("缺少账号 id".to_string());
    }
    let token = manager
        .get_valid_token_for_account(id)
        .await
        .map_err(|e| format!("Codex OAuth token unavailable: {e}"))?;
    let chatgpt_account_id = manager
        .chatgpt_account_id_for_account(id)
        .await
        .map_err(|e| e.to_string())?;
    let result = crate::services::subscription::consume_codex_reset_credit(
        &token,
        Some(&chatgpt_account_id),
    )
    .await?;
    // 托盘额度是缓存快照，用掉后先别让它显示旧值；前端会紧接着重查
    crate::tray::schedule_tray_refresh(&app);
    Ok(result)
}

fn is_transient_token_error(error: &CodexOAuthError) -> bool {
    matches!(
        error,
        CodexOAuthError::NetworkError(_)
            | CodexOAuthError::TokenFetchFailed(_)
            | CodexOAuthError::IoError(_)
    )
}

/// 获取 Codex OAuth (ChatGPT Plus/Pro) 可用模型列表
///
/// ChatGPT Codex 反代使用 `chatgpt.com/backend-api/codex/*`，不是 OpenAI 兼容
/// `/v1/models`。这里复用托管 OAuth 账号的 access_token，直接读取 Codex 后端
/// 暴露的模型列表端点。
#[tauri::command(rename_all = "camelCase")]
pub async fn get_codex_oauth_models(
    account_id: Option<String>,
    state: State<'_, CodexOAuthState>,
) -> Result<Vec<FetchedModel>, String> {
    let manager = &state.0;
    let resolved = match account_id
        .as_deref()
        .map(str::trim)
        .filter(|id| !id.is_empty())
    {
        Some(id) => Some(id.to_string()),
        None => manager.default_account_id().await,
    };
    let Some(id) = resolved else {
        return Err("No ChatGPT account available".to_string());
    };

    let token = manager
        .get_valid_token_for_account(&id)
        .await
        .map_err(|e| format!("Codex OAuth token unavailable: {e}"))?;
    let chatgpt_account_id = manager
        .chatgpt_account_id_for_account(&id)
        .await
        .map_err(|e| e.to_string())?;

    crate::services::codex_oauth_models::fetch_models_with_token(&token, &chatgpt_account_id).await
}

// >>> command adapters（生成物：scripts/gen_command_adapters.py，勿手改）
#[allow(clippy::all, dead_code, non_snake_case, unused_imports)]
pub(crate) mod ipc_adapters {
    include!("../ipc/adapters/codex_oauth.rs");
}
// <<< command adapters
