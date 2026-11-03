//! 供应商路由器模块
//!
//! 负责选择和管理代理目标供应商，实现智能故障转移

use crate::app_config::AppType;
use crate::database::{Database, UpstreamBinding};
use crate::error::AppError;
use crate::provider::Provider;
use crate::proxy::circuit_breaker::{AllowResult, CircuitBreaker, CircuitBreakerConfig};
use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// 失败分类——决定这次失败该记到哪个粒度上。
///
/// 用户的原话：「要区分不同的挂了的情况」。区别在于**影响范围**：
///
/// | 失败类型 | 影响范围 | 记录到 |
/// |---|---|---|
/// | DNS / 连接 / TLS / 超时 | 整个 host | host 级 + 凭证级 |
/// | 5xx | 整个 host（上游自己坏了） | host 级 + 凭证级 |
/// | 401/402/403/429 | 只有这把 key | 仅凭证级 |
/// | 4xx 其余、模型不可用 | 只有这条路径 | 仅凭证级 |
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureKind {
    /// 连不上：DNS/连接/TLS/超时
    Transport,
    /// 上游 5xx
    ServerError,
    /// 鉴权或限流：只影响这一把 key
    Credential,
    /// 请求本身的问题：只影响这条路径
    Request,
}

impl FailureKind {
    /// 该失败是否说明**整个 host** 不可用（只有这类才污染 host 级熔断）
    pub fn affects_host(self) -> bool {
        matches!(self, Self::Transport | Self::ServerError)
    }
}

/// 从 `ProxyError` 归类失败。与 forwarder 里「本家出了问题 → 换下一家」的判定保持一致。
pub fn classify_failure(error: &crate::proxy::error::ProxyError) -> FailureKind {
    use crate::proxy::error::ProxyError;
    match error {
        ProxyError::Timeout(_) | ProxyError::ForwardFailed(_) => FailureKind::Transport,
        ProxyError::UpstreamError { status, .. } => match *status {
            401 | 402 | 403 | 429 => FailureKind::Credential,
            500..=599 => FailureKind::ServerError,
            _ => FailureKind::Request,
        },
        _ => FailureKind::Request,
    }
}

/// 熔断键里「网关级上游」生效的范围（灰度开关，环境变量 `CCS_CIRCUIT_KEY_MODE`）。
///
/// - `off`：完全旧行为，`{app_type}:{provider_id}`，不同 app 各自熔断；
/// - `shadow`（默认）：**仍用旧键**，只在启动时打一份「哪些跨 app 的键会合并」报告；
/// - `on`：改用 `up:{upstream_id}:{protocol}`，同一上游同一协议跨 app 共享健康状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitKeyMode {
    Off,
    Shadow,
    On,
}

impl CircuitKeyMode {
    fn from_env() -> Self {
        match std::env::var("CCS_CIRCUIT_KEY_MODE")
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase()
            .as_str()
        {
            "on" | "upstream" | "1" | "true" => Self::On,
            "off" | "legacy" | "0" | "false" => Self::Off,
            _ => Self::Shadow,
        }
    }
}

/// `provider_id → 网关级键` 的按 app 缓存；provider 增删改后最迟 [`UPSTREAM_KEY_TTL`] 生效。
struct UpstreamKeyEntry {
    loaded_at: Instant,
    keys: HashMap<String, UpstreamBinding>,
}

/// 缓存有效期。取值权衡：太长则新建 provider 迟迟拿不到网关键（只会回落到旧键，
/// 不影响正确性，只是少共享一次熔断）；太短则每次请求都查库。
const UPSTREAM_KEY_TTL: Duration = Duration::from_secs(30);

/// Codex Official requests carry the selected account's native Authorization
/// header. Reusing that request against another account card would cross the
/// account boundary, so these cards must never participate in provider retry.
pub(crate) fn provider_supports_failover(app_type: &str, provider: &Provider) -> bool {
    app_type != AppType::Codex.as_str()
        || !crate::proxy::providers::is_codex_official_provider(provider)
}

/// 供应商路由器
pub struct ProviderRouter {
    /// 数据库连接
    db: Arc<Database>,
    /// 熔断器管理器 - key 见 [`ProviderRouter::circuit_key_for`]
    circuit_breakers: Arc<RwLock<HashMap<String, Arc<CircuitBreaker>>>>,
    /// 熔断键 → 归属的 app_types。网关级键会被多个 app 共用，
    /// `update_app_configs` 需要据此判断某个 key 是否与目标 app 相关。
    circuit_key_apps: Arc<RwLock<HashMap<String, HashSet<String>>>>,
    /// 网关级键缓存（按 app_type）
    upstream_keys: Arc<RwLock<HashMap<String, UpstreamKeyEntry>>>,
    /// 熔断键模式
    circuit_mode: CircuitKeyMode,
}

impl ProviderRouter {
    /// 创建新的供应商路由器
    pub fn new(db: Arc<Database>) -> Self {
        Self {
            db,
            circuit_breakers: Arc::new(RwLock::new(HashMap::new())),
            circuit_key_apps: Arc::new(RwLock::new(HashMap::new())),
            upstream_keys: Arc::new(RwLock::new(HashMap::new())),
            circuit_mode: CircuitKeyMode::from_env(),
        }
    }

    /// 当前的熔断键模式（供启动日志/诊断使用）
    pub fn circuit_key_mode(&self) -> CircuitKeyMode {
        self.circuit_mode
    }

    /// 解析单个 `(app_type, provider_id)` 的熔断键（凭证级）。
    ///
    /// 旧键 `{app_type}:{provider_id}`；网关级键 `up:{upstream_id}:{protocol}:{密钥指纹}`。
    /// 没有绑定关系时（新库未迁移、迁移后被新建的 provider）回落到旧键。
    async fn circuit_key_for(&self, app_type: &str, provider_id: &str) -> String {
        let legacy = format!("{app_type}:{provider_id}");
        if self.circuit_mode != CircuitKeyMode::On {
            return legacy;
        }
        self.resolve_binding(app_type, provider_id)
            .await
            .map(|binding| binding.circuit_key)
            .unwrap_or(legacy)
    }

    /// 解析 host 级熔断键。整个网关连不上时该键熔断，跨 key、跨 app 一起生效。
    ///
    /// 没有显式 URL 的 provider（官方托管）返回 `None`，不参与 host 级熔断。
    async fn host_key_for(&self, app_type: &str, provider_id: &str) -> Option<String> {
        if self.circuit_mode != CircuitKeyMode::On {
            return None;
        }
        self.resolve_binding(app_type, provider_id)
            .await
            .map(|binding| binding.host)
            .filter(|host| !host.is_empty())
            .map(|host| format!("host:{host}"))
    }

    /// 查缓存/数据库得到 provider 的上游绑定（熔断键 + host）
    async fn resolve_binding(&self, app_type: &str, provider_id: &str) -> Option<UpstreamBinding> {
        {
            let cache = self.upstream_keys.read().await;
            if let Some(entry) = cache.get(app_type) {
                if entry.loaded_at.elapsed() < UPSTREAM_KEY_TTL {
                    return entry.keys.get(provider_id).cloned();
                }
            }
        }

        // 同步查库；此处不持有任何异步锁
        let loaded = match self.db.get_provider_upstream_keys(app_type) {
            Ok(keys) => keys,
            Err(err) => {
                log::warn!("[{app_type}] 读取上游绑定失败，熔断键回落旧格式: {err}");
                return None;
            }
        };

        let mut cache = self.upstream_keys.write().await;
        cache.insert(
            app_type.to_string(),
            UpstreamKeyEntry {
                loaded_at: Instant::now(),
                keys: loaded.clone(),
            },
        );
        loaded.get(provider_id).cloned()
    }

    /// 该 provider 所在端点的 base_url（网关表真源）。
    ///
    /// `None` 表示没有绑定、或端点没有显式 URL（官方托管 / OAuth 由适配器算），
    /// 调用方应回落到适配器解析。
    pub async fn endpoint_base_url(&self, app_type: &str, provider_id: &str) -> Option<String> {
        self.resolve_binding(app_type, provider_id)
            .await
            .map(|binding| binding.base_url)
            .filter(|url| !url.is_empty())
    }

    /// 该接入的模型映射（**绑定优先**，取自 `provider_bindings.model_map`）。
    ///
    /// 模型映射属于「某个 agent 怎么用这条接入」，所以归接入而不是网关。
    /// 解析不出时返回 `None`，调用方回落 `ModelMapping::from_provider`——
    /// 现阶段绑定里的值是派生自 provider 的，两条路径结果相同，切换不改行为。
    pub async fn model_mapping(
        &self,
        app_type: &str,
        provider_id: &str,
    ) -> Option<super::model_mapper::ModelMapping> {
        let binding = self.resolve_binding(app_type, provider_id).await?;
        super::model_mapper::ModelMapping::from_json(binding.model_map.as_deref())
    }

    /// 启动诊断：报告「哪些旧键会因为网关级改造而合并」。
    ///
    /// 只在 `shadow` / `on` 下打日志。合并项 > 0 说明确实存在跨 app 的重复上游——
    /// 也就是改造前每个 app 各自重试一遍的那批。
    pub async fn log_upstream_key_diagnostics(&self) {
        if self.circuit_mode == CircuitKeyMode::Off {
            return;
        }
        let mut by_upstream: HashMap<String, Vec<String>> = HashMap::new();
        let mut total = 0usize;
        for app in [
            "claude",
            "claude-desktop",
            "codex",
            "gemini",
            "kimi",
            "opencode",
            "omp",
            "hermes",
        ] {
            let Ok(keys) = self.db.get_provider_upstream_keys(app) else {
                continue;
            };
            for (provider_id, upstream_key) in keys {
                total += 1;
                by_upstream
                    .entry(upstream_key.circuit_key)
                    .or_default()
                    .push(format!("{app}:{provider_id}"));
            }
        }

        let merged: Vec<_> = by_upstream
            .iter()
            .filter(|(_, members)| members.len() > 1)
            .collect();
        let merged_keys = merged.iter().map(|(_, m)| m.len()).sum::<usize>();

        log::info!(
            "熔断键模式 {:?}：{} 条 provider 绑定，{} 个网关级键，其中 {} 个键由多个 app 共享（覆盖 {} 条绑定）",
            self.circuit_mode,
            total,
            by_upstream.len(),
            merged.len(),
            merged_keys
        );
        for (upstream_key, members) in merged {
            log::info!("  {upstream_key} ← {}", members.join(", "));
        }
    }


    /// 选择可用的供应商（支持故障转移）
    ///
    /// 返回按优先级排序的可用供应商列表：
    /// - 故障转移关闭时：仅返回当前供应商
    /// - 故障转移开启时：仅使用故障转移队列，按队列顺序依次尝试（P1 → P2 → ...）
    pub async fn select_providers(&self, app_type: &str) -> Result<Vec<Provider>, AppError> {
        // 代理模式下路由到代理路由那家，和直连指针无关。
        let current_provider = AppType::from_str(app_type).ok().and_then(|app_enum| {
            crate::mode::current::provider_in_use(&self.db, &app_enum)
                .ok()
                .flatten()
        });
        self.select_providers_with_current(app_type, current_provider)
            .await
    }

    /// 同 [`Self::select_providers`]，正在用的那家由调用方给出：处理请求时上下文已经读过
    /// 一次（要读 `live-state.json` 和这一行），不用每个请求再读一遍，两处用的也一定是
    /// 同一家。
    pub async fn select_providers_with_current(
        &self,
        app_type: &str,
        current_provider: Option<Provider>,
    ) -> Result<Vec<Provider>, AppError> {
        let mut result = Vec::new();
        let mut total_providers = 0usize;
        let mut circuit_open_count = 0usize;

        // 检查该应用的自动故障转移开关是否开启（从 proxy_config 表读取）
        let auto_failover_enabled = match self.db.get_proxy_config_for_app(app_type).await {
            Ok(config) => config.auto_failover_enabled,
            Err(e) => {
                log::error!("[{app_type}] 读取 proxy_config 失败: {e}，默认禁用故障转移");
                false
            }
        };

        if auto_failover_enabled
            && current_provider
                .as_ref()
                .is_some_and(|provider| !provider_supports_failover(app_type, provider))
        {
            // A selected Codex Official account is an explicit account choice.
            // Keep it as a single route even if an old failover setting remains
            // enabled; retrying would reuse its inbound token for another card.
            total_providers = 1;
            result.push(current_provider.expect("checked above"));
        } else if auto_failover_enabled {
            // 故障转移开启：仅按队列顺序依次尝试（P1 → P2 → ...）
            let all_providers = self.db.get_all_providers(app_type)?;

            // 使用 DAO 返回的排序结果，确保和前端展示一致
            let ordered_ids: Vec<String> = self
                .db
                .get_failover_queue(app_type)?
                .into_iter()
                .map(|item| item.provider_id)
                .collect();

            // 同一「上游 + 协议 + 密钥」在一次选路里只尝试一次：完全相同的路由重试，
            // 只会把同一个失败再撞一遍。注意**不能按端点折叠**——同一端点换一把 key
            // 仍然值得重试，那正是凭证级故障转移存在的意义。
            let mut seen_routes: std::collections::HashSet<String> =
                std::collections::HashSet::new();

            for provider_id in ordered_ids {
                let Some(provider) = all_providers.get(&provider_id).cloned() else {
                    continue;
                };
                if !provider_supports_failover(app_type, &provider) {
                    continue;
                }

                // 去重看的是「真实路由身份」，与熔断键模式无关：即使还在 shadow，
                // 两条完全相同的上游+协议+密钥也没有重试价值。
                let route_identity = match self.resolve_binding(app_type, &provider.id).await {
                    Some(binding) => binding.circuit_key,
                    None => format!("{app_type}:{}", provider.id),
                };
                if !seen_routes.insert(route_identity) {
                    log::debug!(
                        "[{app_type}] 跳过重复路由候选 {}（与队列中前面的条目同一条路由）",
                        provider.id
                    );
                    continue;
                }
                total_providers += 1;

                let circuit_key = self.circuit_key_for(app_type, &provider.id).await;
                let breaker = self
                    .get_or_create_circuit_breaker(&circuit_key, app_type)
                    .await;

                // host 级：整个网关连不上时跨 key、跨 app 一起跳过
                let host_available = match self.host_key_for(app_type, &provider.id).await {
                    Some(host_key) => {
                        self.get_or_create_circuit_breaker(&host_key, app_type)
                            .await
                            .is_available()
                            .await
                    }
                    None => true,
                };

                if breaker.is_available().await && host_available {
                    result.push(provider);
                } else {
                    circuit_open_count += 1;
                }
            }
        } else {
            // 故障转移关闭：仅使用当前供应商，跳过熔断器检查
            if let Some(current) = current_provider {
                total_providers = 1;
                result.push(current);
            }
        }

        if result.is_empty() {
            if total_providers > 0 && circuit_open_count == total_providers {
                log::warn!("[{app_type}] [FO-004] 所有供应商均已熔断");
                return Err(AppError::AllProvidersCircuitOpen);
            } else {
                log::warn!("[{app_type}] [FO-005] 未配置供应商");
                return Err(AppError::NoProvidersConfigured);
            }
        }

        Ok(result)
    }

    /// 请求执行前获取熔断器“放行许可”
    ///
    /// - Closed：直接放行
    /// - Open：超时到达后切到 HalfOpen 并放行一次探测
    /// - HalfOpen：按限流规则放行探测
    ///
    /// 注意：调用方必须在请求结束后通过 `record_result()` 释放 HalfOpen 名额，
    /// 否则会导致该 Provider 长时间无法进入探测状态。
    pub async fn allow_provider_request(&self, provider_id: &str, app_type: &str) -> AllowResult {
        let circuit_key = self.circuit_key_for(app_type, provider_id).await;
        let breaker = self
            .get_or_create_circuit_breaker(&circuit_key, app_type)
            .await;
        breaker.allow_request().await
    }

    /// 记录供应商请求结果
    ///
    /// `failure_kind` 决定这次失败波及多广：只有 [`FailureKind::affects_host`] 的失败
    /// 才会污染 host 级熔断。`None` 视为与成功无关的未知失败（只记凭证级）。
    pub async fn record_result(
        &self,
        provider_id: &str,
        app_type: &str,
        used_half_open_permit: bool,
        success: bool,
        error_msg: Option<String>,
        failure_kind: Option<FailureKind>,
    ) -> Result<(), AppError> {
        // 1. 按应用独立获取熔断器配置
        let failure_threshold = match self.db.get_proxy_config_for_app(app_type).await {
            Ok(app_config) => app_config.circuit_failure_threshold,
            Err(_) => 5, // 默认值
        };

        // 2. 更新凭证级熔断器
        let circuit_key = self.circuit_key_for(app_type, provider_id).await;
        let breaker = self
            .get_or_create_circuit_breaker(&circuit_key, app_type)
            .await;

        if success {
            breaker.record_success(used_half_open_permit).await;
        } else {
            breaker.record_failure(used_half_open_permit).await;
        }

        // 3. 更新 host 级熔断器
        //
        // host 级不参与 HalfOpen 许可协议（恒定传 false）：`is_available()` 在 HalfOpen
        // 返回 true，`record_*(false)` 只计数不碰许可，因此它能自行超时恢复。
        if let Some(host_key) = self.host_key_for(app_type, provider_id).await {
            let should_touch = match (success, failure_kind) {
                (true, _) => true,
                (false, Some(kind)) => kind.affects_host(),
                (false, None) => false,
            };
            if should_touch {
                let host_breaker = self.get_or_create_circuit_breaker(&host_key, app_type).await;
                if success {
                    host_breaker.record_success(false).await;
                } else {
                    host_breaker.record_failure(false).await;
                }
            }
        }

        // 3. 更新数据库健康状态（使用配置的阈值）
        self.db
            .update_provider_health_with_threshold(
                provider_id,
                app_type,
                success,
                error_msg.clone(),
                failure_threshold,
            )
            .await?;

        Ok(())
    }

    /// 重置熔断器（手动恢复）
    pub async fn reset_circuit_breaker(&self, circuit_key: &str) {
        let breakers = self.circuit_breakers.read().await;
        if let Some(breaker) = breakers.get(circuit_key) {
            breaker.reset().await;
        }
    }

    /// 重置指定供应商的熔断器
    pub async fn reset_provider_breaker(&self, provider_id: &str, app_type: &str) {
        let circuit_key = self.circuit_key_for(app_type, provider_id).await;
        self.reset_circuit_breaker(&circuit_key).await;
        // host 级也要一起复位，否则用户点了恢复、请求仍被 host 熔断挡下
        if let Some(host_key) = self.host_key_for(app_type, provider_id).await {
            self.reset_circuit_breaker(&host_key).await;
        }
    }

    /// 仅释放 HalfOpen permit，不影响健康统计（neutral 接口）
    ///
    /// 用于整流器等场景：请求结果不应计入 Provider 健康度，
    /// 但仍需释放占用的探测名额，避免 HalfOpen 状态卡死
    pub async fn release_permit_neutral(
        &self,
        provider_id: &str,
        app_type: &str,
        used_half_open_permit: bool,
    ) {
        if !used_half_open_permit {
            return;
        }
        let circuit_key = self.circuit_key_for(app_type, provider_id).await;
        let breaker = self
            .get_or_create_circuit_breaker(&circuit_key, app_type)
            .await;
        breaker.release_half_open_permit();
    }

    /// 更新所有熔断器的配置（热更新）
    pub async fn update_all_configs(&self, config: CircuitBreakerConfig) {
        let breakers = self.circuit_breakers.read().await;
        for breaker in breakers.values() {
            breaker.update_config(config.clone()).await;
        }
    }

    /// 更新指定应用已创建熔断器的配置（热更新）
    ///
    /// 网关级键可能被多个 app 共用，因此不能靠 key 前缀判断归属，
    /// 改用 `circuit_key_apps` 记录的真实归属关系。
    pub async fn update_app_configs(&self, app_type: &str, config: CircuitBreakerConfig) {
        let key_apps = self.circuit_key_apps.read().await;
        let breakers = self.circuit_breakers.read().await;
        for (key, breaker) in breakers.iter() {
            let belongs = match key_apps.get(key) {
                Some(apps) => apps.contains(app_type),
                // 归属未知（理论上不会发生）时按旧规则判断，避免漏更新
                None => key.starts_with(&format!("{app_type}:")),
            };
            if belongs {
                breaker.update_config(config.clone()).await;
            }
        }
    }

    /// 获取熔断器状态
    #[allow(dead_code)]
    pub async fn get_circuit_breaker_stats(
        &self,
        provider_id: &str,
        app_type: &str,
    ) -> Option<crate::proxy::circuit_breaker::CircuitBreakerStats> {
        let circuit_key = self.circuit_key_for(app_type, provider_id).await;
        let breakers = self.circuit_breakers.read().await;

        if let Some(breaker) = breakers.get(&circuit_key) {
            Some(breaker.get_stats().await)
        } else {
            None
        }
    }

    /// 获取或创建熔断器
    ///
    /// `app_type` 必须由调用方显式传入：网关级键 `up:{upstream_id}:{protocol}` 不再
    /// 以 app_type 开头，无法从 key 反推（旧实现靠 `split(':')`，换键后会静默取错配置）。
    async fn get_or_create_circuit_breaker(&self, key: &str, app_type: &str) -> Arc<CircuitBreaker> {
        // 先尝试读锁获取
        {
            let breakers = self.circuit_breakers.read().await;
            if let Some(breaker) = breakers.get(key) {
                let breaker = breaker.clone();
                drop(breakers);
                self.register_key_app(key, app_type).await;
                return breaker;
            }
        }

        // 如果不存在，获取写锁创建
        let mut breakers = self.circuit_breakers.write().await;

        // 双重检查，防止竞争条件
        if let Some(breaker) = breakers.get(key) {
            let breaker = breaker.clone();
            drop(breakers);
            self.register_key_app(key, app_type).await;
            return breaker;
        }

        // 按应用独立读取熔断器配置
        let config = match self.db.get_proxy_config_for_app(app_type).await {
            Ok(app_config) => crate::proxy::circuit_breaker::CircuitBreakerConfig {
                failure_threshold: app_config.circuit_failure_threshold,
                success_threshold: app_config.circuit_success_threshold,
                timeout_seconds: app_config.circuit_timeout_seconds as u64,
                error_rate_threshold: app_config.circuit_error_rate_threshold,
                min_requests: app_config.circuit_min_requests,
            },
            Err(_) => crate::proxy::circuit_breaker::CircuitBreakerConfig::default(),
        };

        let breaker = Arc::new(CircuitBreaker::new(config));
        breakers.insert(key.to_string(), breaker.clone());
        drop(breakers);
        self.register_key_app(key, app_type).await;

        breaker
    }

    /// 记录某个熔断键被哪个 app 使用（网关级键会被多个 app 共用）
    async fn register_key_app(&self, key: &str, app_type: &str) {
        let mut key_apps = self.circuit_key_apps.write().await;
        key_apps
            .entry(key.to_string())
            .or_default()
            .insert(app_type.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::lock_conn;
    use crate::database::Database;
    use crate::provider::{AuthBinding, AuthBindingSource, ProviderMeta};
    use serde_json::json;
    use serial_test::serial;
    use std::env;
    use tempfile::TempDir;

    fn managed_codex_official(id: &str, account_id: &str) -> Provider {
        let mut provider = Provider::with_id(
            id.to_string(),
            "OpenAI Official".to_string(),
            json!({ "auth": {}, "config": "" }),
            None,
        );
        provider.category = Some("official".to_string());
        provider.meta = Some(ProviderMeta {
            provider_type: Some("codex_oauth".to_string()),
            auth_binding: Some(AuthBinding {
                source: AuthBindingSource::ManagedAccount,
                auth_provider: Some("codex_oauth".to_string()),
                account_id: Some(account_id.to_string()),
            }),
            ..Default::default()
        });
        provider
    }

    struct TempHome {
        #[allow(dead_code)]
        dir: TempDir,
        original_home: Option<String>,
        original_userprofile: Option<String>,
        original_test_home: Option<String>,
    }

    impl TempHome {
        fn new() -> Self {
            let dir = TempDir::new().expect("failed to create temp home");
            let original_home = env::var("HOME").ok();
            let original_userprofile = env::var("USERPROFILE").ok();
            let original_test_home = env::var("CC_SWITCH_TEST_HOME").ok();

            env::set_var("HOME", dir.path());
            env::set_var("USERPROFILE", dir.path());
            env::set_var("CC_SWITCH_TEST_HOME", dir.path());
            crate::settings::reload_settings().expect("reload settings");

            Self {
                dir,
                original_home,
                original_userprofile,
                original_test_home,
            }
        }
    }

    impl Drop for TempHome {
        fn drop(&mut self) {
            match &self.original_home {
                Some(value) => env::set_var("HOME", value),
                None => env::remove_var("HOME"),
            }

            match &self.original_userprofile {
                Some(value) => env::set_var("USERPROFILE", value),
                None => env::remove_var("USERPROFILE"),
            }

            match &self.original_test_home {
                Some(value) => env::set_var("CC_SWITCH_TEST_HOME", value),
                None => env::remove_var("CC_SWITCH_TEST_HOME"),
            }
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_provider_router_creation() {
        let _home = TempHome::new();
        let db = Arc::new(Database::memory().unwrap());
        let router = ProviderRouter::new(db);

        let breaker = router
            .get_or_create_circuit_breaker("claude:test", "claude")
            .await;
        assert!(breaker.allow_request().await.allowed);
    }

    /// 造一个「两个 app 绑定到同一上游端点」的库，用于验证熔断键合并
    fn seed_shared_upstream(db: &Database) -> Result<(), AppError> {
        let conn = lock_conn!(db.conn);
        conn.execute_batch(
            "INSERT INTO upstreams (id, name, host, created_at)
                VALUES ('up-ai.ctaigw.cn', 'CTYun', 'ai.ctaigw.cn', 1);
             INSERT INTO upstream_endpoints (id, upstream_id, protocol, base_url, created_at)
                VALUES ('ep-a', 'up-ai.ctaigw.cn', 'anthropic', 'https://ai.ctaigw.cn/v1', 1);
             INSERT INTO provider_bindings
                (app_type, provider_id, endpoint_id, is_current, in_failover_queue, sort_index, created_at)
                VALUES ('claude', 'shared', 'ep-a', 0, 0, 0, 1),
                       ('claude-desktop', 'shared', 'ep-a', 0, 0, 0, 1);",
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn gateway_circuit_key_is_shared_across_apps() {
        let _home = TempHome::new();
        let db = Arc::new(Database::memory().unwrap());
        seed_shared_upstream(&db).unwrap();

        let mut router = ProviderRouter::new(db.clone());
        router.circuit_mode = CircuitKeyMode::On;

        let claude_key = router.circuit_key_for("claude", "shared").await;
        let desktop_key = router.circuit_key_for("claude-desktop", "shared").await;

        assert!(
            claude_key.starts_with("up:up-ai.ctaigw.cn:anthropic:"),
            "实际键: {claude_key}"
        );
        assert_eq!(claude_key, desktop_key, "同一上游同一协议必须共用熔断器");

        // 关键：两个 app 拿到的必须是同一个实例，否则「共享健康状态」只是名字好听
        let a = router
            .get_or_create_circuit_breaker(&claude_key, "claude")
            .await;
        let b = router
            .get_or_create_circuit_breaker(&desktop_key, "claude-desktop")
            .await;
        assert!(Arc::ptr_eq(&a, &b), "跨 app 必须复用同一熔断器实例");

        // 没有绑定的 provider 回落旧键：新库、迁移后新建的 provider 都不会因此失效
        assert_eq!(
            router.circuit_key_for("claude", "brand-new").await,
            "claude:brand-new"
        );
    }

    /// 造一个「两个 app 在同一 host、但用不同 key」的库
    fn seed_same_host_two_credentials(db: &Database) -> Result<(), AppError> {
        let conn = lock_conn!(db.conn);
        // 只种 providers：上游 / 端点 / 绑定由读时对账派生，跟真实路径一致
        conn.execute_batch(
            "INSERT INTO providers (id, app_type, name, settings_config, created_at) VALUES
                ('p-a', 'claude', 'A',
                 '{\"env\":{\"ANTHROPIC_BASE_URL\":\"https://api.kimi.com/coding\",\"ANTHROPIC_AUTH_TOKEN\":\"sk-aaa\"}}', 1),
                ('p-b', 'claude-desktop', 'B',
                 '{\"env\":{\"ANTHROPIC_BASE_URL\":\"https://api.kimi.com/coding\",\"ANTHROPIC_AUTH_TOKEN\":\"sk-bbb\"}}', 1);",
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    /// 模型映射的读路径已经走绑定。
    ///
    /// 注意现阶段**写源仍是 provider**：手工改 `provider_bindings.model_map` 会被下一次
    /// 对账按 provider 覆盖回来。这是预期行为——读路径先切，等界面上能编辑模型映射时，
    /// 再按 base_url 那套「改一处 + 向下传播」把写源翻过来。
    #[tokio::test]
    #[serial]
    async fn model_mapping_is_read_from_binding() -> Result<(), AppError> {
        let _home = TempHome::new();
        let db = Arc::new(Database::memory().unwrap());

        let provider = Provider::with_id(
            "p".to_string(),
            "P".to_string(),
            json!({
                "env": {
                    "ANTHROPIC_BASE_URL": "https://api.kimi.com/coding",
                    "ANTHROPIC_AUTH_TOKEN": "sk-abc",
                    "ANTHROPIC_DEFAULT_SONNET_MODEL": "kimi-k2",
                }
            }),
            None,
        );
        db.save_provider("claude", &provider)?;

        // 第一次读会派生绑定
        let router = ProviderRouter::new(db.clone());
        assert_eq!(
            router
                .model_mapping("claude", "p")
                .await
                .and_then(|mapping| mapping.sonnet_model),
            Some("kimi-k2".to_string()),
            "映射应从绑定读出来（当前由 provider 派生）"
        );

        // 绑定里确实落库了槽位化 JSON，而不是空
        let stored: String = {
            let conn = lock_conn!(db.conn);
            conn.query_row(
                "SELECT COALESCE(model_map, '') FROM provider_bindings
                 WHERE app_type = 'claude' AND provider_id = 'p'",
                [],
                |row| row.get(0),
            )
            .map_err(|e| AppError::Database(e.to_string()))?
        };
        assert!(
            stored.contains("kimi-k2") && stored.contains("sonnet"),
            "绑定里的 model_map 应是槽位化 JSON，实际 {stored}"
        );

        // 没有绑定的 provider：返回 None，调用方回落 provider
        assert!(router.model_mapping("claude", "missing").await.is_none());
        Ok(())
    }

    /// 改「接入」的模型映射，路由器读到的必须跟着变。
    ///
    /// 这是"映射归接入"的闭环：写的是该 app 自己的 provider 配置，
    /// 再对账刷新绑定，读路径立刻生效——同一个网关在不同 app 里可以是不同模型。
    #[tokio::test]
    #[serial]
    async fn editing_binding_model_map_changes_what_router_reads() -> Result<(), AppError> {
        let _home = TempHome::new();
        let db = Arc::new(Database::memory().unwrap());
        let provider = Provider::with_id(
            "p".to_string(),
            "P".to_string(),
            json!({
                "env": {
                    "ANTHROPIC_BASE_URL": "https://api.kimi.com/coding",
                    "ANTHROPIC_AUTH_TOKEN": "sk-abc",
                    "ANTHROPIC_DEFAULT_SONNET_MODEL": "from-provider",
                }
            }),
            None,
        );
        db.save_provider("claude", &provider)?;

        let router = ProviderRouter::new(db.clone());
        assert_eq!(
            router
                .model_mapping("claude", "p")
                .await
                .and_then(|m| m.sonnet_model),
            Some("from-provider".to_string())
        );

        {
            let mut conn = lock_conn!(db.conn);
            let changed = Database::update_binding_model_map(
                &mut conn,
                "claude",
                "p",
                &json!({"sonnet": "kimi-k2", "haiku": "kimi-k2-small"}),
            )?;
            assert!(changed, "claude 系配置应可写模型映射");
            Database::reconcile_provider_bindings(&mut conn, "claude")?;
        }

        let router = ProviderRouter::new(db.clone());
        let mapping = router.model_mapping("claude", "p").await.unwrap();
        assert_eq!(mapping.sonnet_model.as_deref(), Some("kimi-k2"));
        assert_eq!(mapping.haiku_model.as_deref(), Some("kimi-k2-small"));

        // 写源确实翻到了 provider 配置上，而不只是改了绑定
        let stored: String = {
            let conn = lock_conn!(db.conn);
            conn.query_row(
                "SELECT settings_config FROM providers WHERE app_type='claude' AND id='p'",
                [],
                |row| row.get(0),
            )
            .map_err(|e| AppError::Database(e.to_string()))?
        };
        assert!(stored.contains("kimi-k2"), "provider 配置应已改写: {stored}");
        assert!(!stored.contains("from-provider"), "旧映射应被覆盖");
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn endpoint_base_url_comes_from_gateway_table() -> Result<(), AppError> {
        let _home = TempHome::new();
        let db = Arc::new(Database::memory().unwrap());
        seed_same_host_two_credentials(&db)?;
        // 再加一个没有显式 URL 的 provider：它必须交回 None，让适配器去算
        {
            let conn = lock_conn!(db.conn);
            conn.execute(
                "INSERT INTO providers (id, app_type, name, settings_config, created_at)
                 VALUES ('official', 'claude', 'Official', '{\"env\":{}}', 1)",
                [],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        }

        let router = ProviderRouter::new(db.clone());

        let a = router.endpoint_base_url("claude", "p-a").await;
        let b = router.endpoint_base_url("claude-desktop", "p-b").await;
        assert_eq!(a.as_deref(), Some("https://api.kimi.com/coding"));
        assert_eq!(a, b, "同一网关的端点 URL 必须一致");

        assert_eq!(
            router.endpoint_base_url("claude", "official").await,
            None,
            "无显式 URL 的 provider 不得编造 base_url"
        );
        assert_eq!(router.endpoint_base_url("claude", "missing-id").await, None);
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn transport_failure_trips_host_breaker_for_other_app() -> Result<(), AppError> {
        let _home = TempHome::new();
        let db = Arc::new(Database::memory().unwrap());
        seed_same_host_two_credentials(&db)?;

        let mut router = ProviderRouter::new(db.clone());
        router.circuit_mode = CircuitKeyMode::On;

        // 两个 app 在同一个 host，host 级键必须相同（凭证不同不影响 host 级）
        let host_a = router.host_key_for("claude", "p-a").await;
        let host_b = router.host_key_for("claude-desktop", "p-b").await;
        assert_eq!(host_a, host_b, "同一 host 必须共用 host 级熔断器");
        assert_eq!(host_a.as_deref(), Some("host:api.kimi.com"));

        // claude 侧连不上（Transport）若干次 → host 级熔断
        for _ in 0..8 {
            router
                .record_result(
                    "p-a",
                    "claude",
                    false,
                    false,
                    Some("connect refused".into()),
                    Some(FailureKind::Transport),
                )
                .await?;
        }

        let host_breaker = router
            .get_or_create_circuit_breaker(host_a.as_ref().unwrap(), "claude")
            .await;
        assert!(
            !host_breaker.is_available().await,
            "host 已熔断：另一个 app 不该再去撞同一个挂掉的网关"
        );
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn credential_failure_does_not_trip_host_breaker() -> Result<(), AppError> {
        let _home = TempHome::new();
        let db = Arc::new(Database::memory().unwrap());
        seed_same_host_two_credentials(&db)?;

        let mut router = ProviderRouter::new(db.clone());
        router.circuit_mode = CircuitKeyMode::On;

        // claude 侧 key 失效（401/429）若干次
        for _ in 0..8 {
            router
                .record_result(
                    "p-a",
                    "claude",
                    false,
                    false,
                    Some("401 unauthorized".into()),
                    Some(FailureKind::Credential),
                )
                .await?;
        }

        // 凭证级熔断，但 host 级必须干净——另一把 key 完全无辜
        let cred_key = router.circuit_key_for("claude", "p-a").await;
        let cred_breaker = router
            .get_or_create_circuit_breaker(&cred_key, "claude")
            .await;
        assert!(!cred_breaker.is_available().await, "这把 key 应该熔断");

        let host_key = router.host_key_for("claude-desktop", "p-b").await.unwrap();
        let host_breaker = router
            .get_or_create_circuit_breaker(&host_key, "claude-desktop")
            .await;
        assert!(
            host_breaker.is_available().await,
            "key 配额问题不代表网关挂了：另一把 key 不得被牵连"
        );
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn different_keys_on_same_upstream_do_not_share_breaker() -> Result<(), AppError> {
        let _home = TempHome::new();
        let db = Arc::new(Database::memory().unwrap());
        {
            let conn = lock_conn!(db.conn);
            // 只种 providers，其余交给读时对账派生
            conn.execute_batch(
                "INSERT INTO providers (id, app_type, name, settings_config, created_at)
                    VALUES ('kimi-coding', 'claude', 'Kimi For Coding',
                            '{\"env\":{\"ANTHROPIC_BASE_URL\":\"https://api.kimi.com/coding\",\"ANTHROPIC_AUTH_TOKEN\":\"sk-aaa\"}}', 1),
                           ('kimi-coding', 'claude-desktop', 'Kimi For Coding',
                            '{\"env\":{\"ANTHROPIC_BASE_URL\":\"https://api.kimi.com/coding\",\"ANTHROPIC_AUTH_TOKEN\":\"sk-bbb\"}}', 1);",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        }

        let mut router = ProviderRouter::new(db.clone());
        router.circuit_mode = CircuitKeyMode::On;

        let a = router.circuit_key_for("claude", "kimi-coding").await;
        let b = router.circuit_key_for("claude-desktop", "kimi-coding").await;

        assert!(a.starts_with("up:up-api.kimi.com:anthropic:"), "实际键: {a}");
        assert_ne!(
            a, b,
            "同一上游但 key 不同：一把 key 配额耗尽不得把另一把也熔断"
        );
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn legacy_mode_keeps_per_app_isolation() {
        let _home = TempHome::new();
        let db = Arc::new(Database::memory().unwrap());
        seed_shared_upstream(&db).unwrap();

        let mut router = ProviderRouter::new(db.clone());
        router.circuit_mode = CircuitKeyMode::Off;

        let claude_key = router.circuit_key_for("claude", "shared").await;
        let desktop_key = router.circuit_key_for("claude-desktop", "shared").await;
        assert_eq!(claude_key, "claude:shared");
        assert_ne!(
            claude_key, desktop_key,
            "旧键按 app 隔离——这正是改造前同一网关被两个 agent 各重试一遍的原因"
        );

        let a = router
            .get_or_create_circuit_breaker(&claude_key, "claude")
            .await;
        let b = router
            .get_or_create_circuit_breaker(&desktop_key, "claude-desktop")
            .await;
        assert!(!Arc::ptr_eq(&a, &b));
    }

    #[tokio::test]
    #[serial]
    async fn test_failover_disabled_uses_current_provider() {
        let _home = TempHome::new();
        let db = Arc::new(Database::memory().unwrap());

        let provider_a =
            Provider::with_id("a".to_string(), "Provider A".to_string(), json!({}), None);
        let provider_b =
            Provider::with_id("b".to_string(), "Provider B".to_string(), json!({}), None);

        db.save_provider("claude", &provider_a).unwrap();
        db.save_provider("claude", &provider_b).unwrap();
        db.set_current_provider("claude", "a").unwrap();
        db.add_to_failover_queue("claude", "b").unwrap();

        let router = ProviderRouter::new(db.clone());
        let providers = router.select_providers("claude").await.unwrap();

        assert_eq!(providers.len(), 1);
        assert_eq!(providers[0].id, "a");
    }

    #[tokio::test]
    #[serial]
    async fn test_failover_enabled_uses_queue_order_ignoring_current() {
        let _home = TempHome::new();
        let db = Arc::new(Database::memory().unwrap());

        // 设置 sort_index 来控制顺序：b=1, a=2
        let mut provider_a =
            Provider::with_id("a".to_string(), "Provider A".to_string(), json!({}), None);
        provider_a.sort_index = Some(2);
        let mut provider_b =
            Provider::with_id("b".to_string(), "Provider B".to_string(), json!({}), None);
        provider_b.sort_index = Some(1);

        db.save_provider("claude", &provider_a).unwrap();
        db.save_provider("claude", &provider_b).unwrap();
        db.set_current_provider("claude", "a").unwrap();

        db.add_to_failover_queue("claude", "b").unwrap();
        db.add_to_failover_queue("claude", "a").unwrap();

        // 启用自动故障转移（使用新的 proxy_config API）
        let mut config = db.get_proxy_config_for_app("claude").await.unwrap();
        config.auto_failover_enabled = true;
        db.update_proxy_config_for_app(config).await.unwrap();

        let router = ProviderRouter::new(db.clone());
        let providers = router.select_providers("claude").await.unwrap();

        assert_eq!(providers.len(), 2);
        // 故障转移开启时：仅按队列顺序选择（忽略当前供应商）
        assert_eq!(providers[0].id, "b");
        assert_eq!(providers[1].id, "a");
    }

    #[tokio::test]
    #[serial]
    async fn test_failover_enabled_uses_queue_only_even_if_current_not_in_queue() {
        let _home = TempHome::new();
        let db = Arc::new(Database::memory().unwrap());

        let provider_a =
            Provider::with_id("a".to_string(), "Provider A".to_string(), json!({}), None);
        let mut provider_b =
            Provider::with_id("b".to_string(), "Provider B".to_string(), json!({}), None);
        provider_b.sort_index = Some(1);

        db.save_provider("claude", &provider_a).unwrap();
        db.save_provider("claude", &provider_b).unwrap();
        db.set_current_provider("claude", "a").unwrap();

        // 只把 b 加入故障转移队列（模拟“当前供应商不在队列里”的常见配置）
        db.add_to_failover_queue("claude", "b").unwrap();

        let mut config = db.get_proxy_config_for_app("claude").await.unwrap();
        config.auto_failover_enabled = true;
        db.update_proxy_config_for_app(config).await.unwrap();

        let router = ProviderRouter::new(db.clone());
        let providers = router.select_providers("claude").await.unwrap();

        assert_eq!(providers.len(), 1);
        assert_eq!(providers[0].id, "b");
    }

    /// 同一条路由（上游 + 协议 + 密钥完全相同）在队列里出现两次时只尝试一次；
    /// 但同一端点换一把 key 必须保留——那是凭证级故障转移的意义。
    #[tokio::test]
    #[serial]
    async fn failover_queue_skips_duplicate_routes_but_keeps_distinct_keys(
    ) -> Result<(), AppError> {
        let _home = TempHome::new();
        let db = Arc::new(Database::memory().unwrap());

        let make = |id: &str, key: &str| {
            Provider::with_id(
                id.to_string(),
                id.to_string(),
                json!({
                    "env": {
                        "ANTHROPIC_BASE_URL": "https://api.kimi.com/coding",
                        "ANTHROPIC_AUTH_TOKEN": key,
                    }
                }),
                None,
            )
        };

        db.save_provider("claude", &make("dup-a", "sk-same"))?;
        db.save_provider("claude", &make("dup-b", "sk-same"))?;
        db.save_provider("claude", &make("other-key", "sk-diff"))?;
        for id in ["dup-a", "dup-b", "other-key"] {
            db.add_to_failover_queue("claude", id)?;
        }

        let mut config = db.get_proxy_config_for_app("claude").await?;
        config.auto_failover_enabled = true;
        db.update_proxy_config_for_app(config).await?;

        let router = ProviderRouter::new(db.clone());
        let providers = router.select_providers("claude").await?;
        let ids: Vec<&str> = providers.iter().map(|p| p.id.as_str()).collect();

        assert_eq!(
            ids,
            vec!["dup-a", "other-key"],
            "同一路由应折叠，不同密钥不得折叠"
        );
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn codex_official_current_stays_single_route_when_failover_is_stale() {
        let _home = TempHome::new();
        let db = Arc::new(Database::memory().unwrap());
        let official = managed_codex_official("official-a", "account-a");
        let fallback = Provider::with_id(
            "fallback".to_string(),
            "Fallback".to_string(),
            json!({}),
            None,
        );
        db.save_provider("codex", &official).unwrap();
        db.save_provider("codex", &fallback).unwrap();
        db.set_current_provider("codex", &official.id).unwrap();
        db.add_to_failover_queue("codex", &fallback.id).unwrap();

        let mut config = db.get_proxy_config_for_app("codex").await.unwrap();
        config.auto_failover_enabled = true;
        db.update_proxy_config_for_app(config).await.unwrap();

        let providers = ProviderRouter::new(db)
            .select_providers("codex")
            .await
            .unwrap();
        assert_eq!(providers.len(), 1);
        assert_eq!(providers[0].id, official.id);
    }

    #[tokio::test]
    #[serial]
    async fn stale_codex_official_queue_entries_are_not_retry_targets() {
        let _home = TempHome::new();
        let db = Arc::new(Database::memory().unwrap());
        let current = Provider::with_id(
            "third-party".to_string(),
            "Third Party".to_string(),
            json!({}),
            None,
        );
        let official = managed_codex_official("official-a", "account-a");
        let fallback = Provider::with_id(
            "fallback".to_string(),
            "Fallback".to_string(),
            json!({}),
            None,
        );
        db.save_provider("codex", &current).unwrap();
        db.save_provider("codex", &official).unwrap();
        db.save_provider("codex", &fallback).unwrap();
        db.set_current_provider("codex", &current.id).unwrap();
        db.add_to_failover_queue("codex", &official.id).unwrap();
        db.add_to_failover_queue("codex", &fallback.id).unwrap();

        let mut config = db.get_proxy_config_for_app("codex").await.unwrap();
        config.auto_failover_enabled = true;
        db.update_proxy_config_for_app(config).await.unwrap();

        let providers = ProviderRouter::new(db)
            .select_providers("codex")
            .await
            .unwrap();
        assert_eq!(
            providers
                .iter()
                .map(|provider| provider.id.as_str())
                .collect::<Vec<_>>(),
            vec!["fallback"]
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_select_providers_does_not_consume_half_open_permit() {
        let _home = TempHome::new();
        let db = Arc::new(Database::memory().unwrap());

        db.update_circuit_breaker_config(&CircuitBreakerConfig {
            failure_threshold: 1,
            timeout_seconds: 0,
            ..Default::default()
        })
        .await
        .unwrap();

        let provider_a =
            Provider::with_id("a".to_string(), "Provider A".to_string(), json!({}), None);
        let provider_b =
            Provider::with_id("b".to_string(), "Provider B".to_string(), json!({}), None);

        db.save_provider("claude", &provider_a).unwrap();
        db.save_provider("claude", &provider_b).unwrap();

        db.add_to_failover_queue("claude", "a").unwrap();
        db.add_to_failover_queue("claude", "b").unwrap();

        // 启用自动故障转移（使用新的 proxy_config API）
        let mut config = db.get_proxy_config_for_app("claude").await.unwrap();
        config.auto_failover_enabled = true;
        db.update_proxy_config_for_app(config).await.unwrap();

        let router = ProviderRouter::new(db.clone());

        router
            .record_result("b", "claude", false, false, Some("fail".to_string()), None)
            .await
            .unwrap();

        let providers = router.select_providers("claude").await.unwrap();
        assert_eq!(providers.len(), 2);

        assert!(router.allow_provider_request("b", "claude").await.allowed);
    }

    #[tokio::test]
    #[serial]
    async fn test_release_permit_neutral_frees_half_open_slot() {
        let _home = TempHome::new();
        let db = Arc::new(Database::memory().unwrap());

        // 配置熔断器：1 次失败即熔断，0 秒超时立即进入 HalfOpen
        db.update_circuit_breaker_config(&CircuitBreakerConfig {
            failure_threshold: 1,
            timeout_seconds: 0,
            ..Default::default()
        })
        .await
        .unwrap();

        let provider_a =
            Provider::with_id("a".to_string(), "Provider A".to_string(), json!({}), None);
        db.save_provider("claude", &provider_a).unwrap();
        db.add_to_failover_queue("claude", "a").unwrap();

        // 启用自动故障转移
        let mut config = db.get_proxy_config_for_app("claude").await.unwrap();
        config.auto_failover_enabled = true;
        db.update_proxy_config_for_app(config).await.unwrap();

        let router = ProviderRouter::new(db.clone());

        // 触发熔断：1 次失败
        router
            .record_result("a", "claude", false, false, Some("fail".to_string()), None)
            .await
            .unwrap();

        // 第一次请求：获取 HalfOpen 探测名额
        let first = router.allow_provider_request("a", "claude").await;
        assert!(first.allowed);
        assert!(first.used_half_open_permit);

        // 第二次请求应被拒绝（名额已被占用）
        let second = router.allow_provider_request("a", "claude").await;
        assert!(!second.allowed);

        // 使用 release_permit_neutral 释放名额（不影响健康统计）
        router
            .release_permit_neutral("a", "claude", first.used_half_open_permit)
            .await;

        // 第三次请求应被允许（名额已释放）
        let third = router.allow_provider_request("a", "claude").await;
        assert!(third.allowed);
        assert!(third.used_half_open_permit);
    }
}
