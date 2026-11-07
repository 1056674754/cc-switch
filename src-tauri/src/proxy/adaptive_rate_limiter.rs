//! 自适应 429 限流退避控制器
//!
//! 解决的问题：并发挂机任务撞上 429 后疯狂重试上游。此前代理只有两段固定行为——
//! 客户端看到的所有 429 都盖写死的 `retry-after: 10`，上游真正的 `Retry-After`
//! 头在转发层被丢弃；429 之后立刻 failover 下一家，没有间隔、没有统计。
//!
//! 做法：每个端点（与熔断器同款 key，见 [`crate::proxy::provider_router::ProviderRouter`]）
//! 维护一份退避状态，算法是 AIMD 式拥塞控制的变体：
//!
//! 1. **升级（收到 429）**：退避档位乘性增大，倍率随近 60s 的 429 密度加权——
//!    越是被密集锤打，冷却涨得越快；上游给了 `Retry-After` 则以其为下限。
//! 2. **恢复（探测）**：冷却到期只放行 1 个探测请求，成功则档位减半并退出冷却，
//!    失败则按 429 重新升级。非冷却期的成功让档位温和衰减回下限。
//! 3. **羊群打散**：冷却期内第 k 个被拒的客户端拿到「剩余冷却 + 档位的 k%4/4」
//!    作为重试间隔——并发客户端的重试时刻被摊开到整个冷却窗口，不会同时踩踏。
//! 4. **钳制**：档位永远在 `[floor, cap]` 内。cap 默认 60s：Claude 客户端对
//!    超过 60s 的 retry-after 会放弃重试（见 `error.rs` 的注释）。
//!
//! 锁策略：`acquire`/`note_success` 是每个请求都会走的热路径，先走读锁快速放行
//! （无状态或未冷却时零写入）；只有真正发生 429 / 冷却到期升级探测时才拿写锁。

use serde::Serialize;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// 429 滑动窗口宽度：密度加权看的就是这段时间内锤了几次。
const WINDOW: Duration = Duration::from_secs(60);

/// 羊群打散的档位 granularity：被拒客户端的重试时刻按 1/4 档位错开。
const STAGGER_SLOTS: u64 = 4;

/// 每个端点的退避状态。只在写锁内变更。
#[derive(Debug)]
struct EndpointState {
    /// 当前退避档位（秒）。下次 429 的冷却时长、探测失败后的重试间隔都由它派生。
    level_secs: f64,
    /// 冷却截止时刻；`None` 表示不在冷却期。
    cooldown_until: Option<Instant>,
    /// 近 `WINDOW` 内的 429 到达时刻（含探测失败）。
    window: VecDeque<Instant>,
    /// 冷却到期后放出的探测请求是否还在途。在途期间其余请求继续被拒。
    probe_in_flight: bool,
    /// 冷却期内被拒请求的计数，用于羊群打散取槽位。
    rejected: AtomicU64,
}

impl EndpointState {
    fn new(level_secs: f64) -> Self {
        Self {
            level_secs,
            cooldown_until: None,
            window: VecDeque::new(),
            probe_in_flight: false,
            rejected: AtomicU64::new(0),
        }
    }

    /// 丢弃滑窗外的旧 429，返回窗口内的次数。
    fn trim_window(&mut self, now: Instant) -> usize {
        while let Some(&front) = self.window.front() {
            if now.duration_since(front) <= WINDOW {
                break;
            }
            self.window.pop_front();
        }
        self.window.len()
    }
}

/// 按应用生效的退避参数（来自 proxy_config，热更新）。
#[derive(Debug, Clone, Copy)]
pub struct RateLimitBackoffConfig {
    /// 总开关。关闭时不建状态、不打冷却，429 退避回落旧行为（上游 hint 或固定 10s）。
    pub enabled: bool,
    /// 档位下限（秒）：首次 429 的最短冷却。
    pub floor_secs: f64,
    /// 档位上限（秒）：默认 60，超过 Claude 客户端会放弃重试。
    pub cap_secs: f64,
}

impl Default for RateLimitBackoffConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            floor_secs: 2.0,
            cap_secs: 60.0,
        }
    }
}

/// `acquire` 的放行/拒绝结论。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Admission {
    /// 放行（可能作为冷却到期后的探测请求）。
    Allow,
    /// 拒绝：不要打上游，客户端按 `retry_after_secs` 重试。
    Deny { retry_after_secs: u64 },
}

/// 暴露给前端/命令层的单端点限流状态。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RateLimitSnapshot {
    /// 端点 key（与熔断键同格式）。
    pub key: String,
    /// 已知使用该端点的 (app_type, provider_id)。
    pub owners: Vec<RateLimitOwner>,
    /// 当前退避档位（秒）。
    pub level_secs: u64,
    /// 距冷却结束的剩余毫秒；不在冷却期为 0。
    pub cooldown_remaining_ms: u64,
    /// 近 60s 的 429 次数。
    pub recent_429_count: u64,
}

/// 快照里的使用方。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RateLimitOwner {
    pub app_type: String,
    pub provider_id: String,
}

/// 自适应限流退避控制器。挂在 [`crate::proxy::provider_router::ProviderRouter`] 上，
/// 与熔断器共享 key 体系。
pub struct AdaptiveRateLimiter {
    /// key → 端点状态。只对发生过 429 的端点建条目，热路径读锁零成本。
    states: RwLock<HashMap<String, EndpointState>>,
    /// key → 已知使用方，快照时把端点状态还原成 provider 视角。
    owners: RwLock<HashMap<String, HashSet<(String, String)>>>,
    /// app_type → 退避参数（热更新自 proxy_config）。
    configs: RwLock<HashMap<String, RateLimitBackoffConfig>>,
}

impl AdaptiveRateLimiter {
    pub fn new() -> Self {
        Self {
            states: RwLock::new(HashMap::new()),
            owners: RwLock::new(HashMap::new()),
            configs: RwLock::new(HashMap::new()),
        }
    }

    /// 热更新某 app 的退避参数。
    pub async fn update_config(&self, app_type: &str, config: RateLimitBackoffConfig) {
        self.configs
            .write()
            .await
            .insert(app_type.to_string(), config);
    }

    async fn config_for(&self, app_type: &str) -> RateLimitBackoffConfig {
        self.configs
            .read()
            .await
            .get(app_type)
            .copied()
            .unwrap_or_default()
    }

    fn remember_owner(&self, key: &str, app_type: &str, provider_id: &str) {
        if let Ok(mut owners) = self.owners.try_write() {
            owners
                .entry(key.to_string())
                .or_default()
                .insert((app_type.to_string(), provider_id.to_string()));
        }
    }

    /// 记录一次上游 429，升级退避档位并开启冷却。
    ///
    /// `upstream_retry_after` 是上游响应头里给出的建议等待；算法把它作为档位下限。
    /// 返回建议客户端等待的秒数（冷却刚开始，即整个档位）。
    pub async fn note_429(
        &self,
        key: &str,
        app_type: &str,
        provider_id: &str,
        upstream_retry_after: Option<Duration>,
    ) -> u64 {
        self.note_429_at(
            key,
            app_type,
            provider_id,
            upstream_retry_after,
            Instant::now(),
        )
        .await
    }

    /// [`Self::note_429`] 的可测版本：时间由调用方注入。
    async fn note_429_at(
        &self,
        key: &str,
        app_type: &str,
        provider_id: &str,
        upstream_retry_after: Option<Duration>,
        now: Instant,
    ) -> u64 {
        self.remember_owner(key, app_type, provider_id);
        let config = self.config_for(app_type).await;

        // 未启用自适应退避：保持旧行为，不建状态，
        // 客户端拿到上游 hint 或默认固定值。
        if !config.enabled {
            return upstream_retry_after
                .map(|d| d.as_secs().max(1))
                .unwrap_or(10);
        }

        let mut states = self.states.write().await;
        let state = states
            .entry(key.to_string())
            .or_insert_with(|| EndpointState::new(config.floor_secs));

        let density = state.trim_window(now) as f64;
        state.window.push_back(now);

        // 密度加权乘性升级：窗口内已有的 429 越多，倍率越陡。
        let multiplier = 1.5 + (0.5 * density).min(1.5);
        let mut candidate = state.level_secs * multiplier;
        if let Some(hint) = upstream_retry_after {
            candidate = candidate.max(hint.as_secs_f64());
        }
        state.level_secs = candidate.clamp(config.floor_secs, config.cap_secs);
        state.cooldown_until = Some(now + Duration::from_secs_f64(state.level_secs));
        state.probe_in_flight = false;
        state.rejected.store(0, Ordering::Relaxed);

        state.level_secs.ceil() as u64
    }

    /// 请求放行判定。冷却期内拒绝（不打上游）；冷却到期放一个探测请求，其余继续拒。
    pub async fn acquire(&self, key: &str, app_type: &str, provider_id: &str) -> Admission {
        self.acquire_at(key, app_type, provider_id, Instant::now())
            .await
    }

    /// [`Self::acquire`] 的可测版本：时间由调用方注入。
    async fn acquire_at(
        &self,
        key: &str,
        app_type: &str,
        provider_id: &str,
        now: Instant,
    ) -> Admission {
        // 快路径：无状态或未冷却，读锁直接放行。
        {
            let states = self.states.read().await;
            let Some(state) = states.get(key) else {
                return Admission::Allow;
            };
            let Some(cooldown_until) = state.cooldown_until else {
                return Admission::Allow;
            };
            if now < cooldown_until {
                // 已在运行中关闭自适应（或参数异常）：状态作废，直接放行。
                let config = self.config_for(app_type).await;
                if !config.enabled {
                    return Admission::Allow;
                }
                // 冷却中：本地拒绝，不打上游。间隔 = 剩余冷却 + 打散槽位，
                // 整体钳到 cap（打散槽位最多加 3/4 档位，高档位时会顶到上限——
                // 此时剩余冷却本身已接近 cap，摊不开了也无妨）。
                let remaining = cooldown_until.duration_since(now).as_secs_f64();
                let slot = state.rejected.fetch_add(1, Ordering::Relaxed) % STAGGER_SLOTS;
                let level = state.level_secs;
                let retry = remaining + slot as f64 * level / STAGGER_SLOTS as f64;
                return Admission::Deny {
                    retry_after_secs: retry.min(config.cap_secs).ceil().max(1.0) as u64,
                };
            }
            // 冷却已到期：若已有探测在途，继续拒；否则升级为探测放行。
            if state.probe_in_flight {
                let level = state.level_secs;
                return Admission::Deny {
                    retry_after_secs: (level / 2.0).ceil().max(1.0) as u64,
                };
            }
        }

        // 慢路径：冷却到期且需要登记探测。拿写锁重查（可能被并发请求抢先）。
        let mut states = self.states.write().await;
        let Some(state) = states.get_mut(key) else {
            return Admission::Allow;
        };
        let Some(cooldown_until) = state.cooldown_until else {
            return Admission::Allow;
        };
        if now < cooldown_until || state.probe_in_flight {
            // 并发窗口内已被别人抢先成为探测 / 冷却被 note_429 重新开启。
            let config = self.config_for(app_type).await;
            let remaining = cooldown_until
                .checked_duration_since(now)
                .unwrap_or(Duration::ZERO)
                .as_secs_f64();
            let level = state.level_secs;
            let slot = state.rejected.fetch_add(1, Ordering::Relaxed) % STAGGER_SLOTS;
            let retry = remaining.max(level / 2.0) + slot as f64 * level / STAGGER_SLOTS as f64;
            return Admission::Deny {
                retry_after_secs: retry.min(config.cap_secs).ceil().max(1.0) as u64,
            };
        }
        self.remember_owner(key, app_type, provider_id);
        state.probe_in_flight = true;
        Admission::Allow
    }

    /// 记录一次上游成功：探测成功 → 退出冷却、档位减半；非冷却期 → 档位温和衰减。
    pub async fn note_success(&self, key: &str, app_type: &str) {
        self.note_success_at(key, app_type, Instant::now()).await;
    }

    /// 探测请求以**非 429** 方式失败（5xx / 网络错误 / 超时）。
    ///
    /// 上游不是在限流，但也没证明恢复：清掉探测在途标记（否则 flag 永久卡死、
    /// 端点再也无法探测），按当前档位的一半重新冷却等下一次探测。
    /// 非探测期的失败（已有 note_429 记账）调用无副作用。
    pub async fn note_probe_failure(&self, key: &str, app_type: &str) {
        self.note_probe_failure_at(key, app_type, Instant::now())
            .await;
    }

    /// [`Self::note_probe_failure`] 的可测版本：时间由调用方注入。
    async fn note_probe_failure_at(&self, key: &str, app_type: &str, now: Instant) {
        let config = self.config_for(app_type).await;
        let mut states = self.states.write().await;
        let Some(state) = states.get_mut(key) else {
            return;
        };
        if !state.probe_in_flight {
            return;
        }
        state.probe_in_flight = false;
        state.cooldown_until =
            Some(now + Duration::from_secs_f64((state.level_secs * 0.5).max(config.floor_secs)));
    }

    /// [`Self::note_success`] 的可测版本：时间由调用方注入。
    async fn note_success_at(&self, key: &str, app_type: &str, now: Instant) {
        let config = self.config_for(app_type).await;
        let mut states = self.states.write().await;
        let Some(state) = states.get_mut(key) else {
            return;
        };

        if state.probe_in_flight {
            // 探测成功：退出冷却，档位减半（快速恢复吞吐）。
            state.probe_in_flight = false;
            state.cooldown_until = None;
            state.level_secs = (state.level_secs * 0.5).max(config.floor_secs);
            state.rejected.store(0, Ordering::Relaxed);
            return;
        }

        // 非探测成功：档位温和衰减回下限。冷却不应由非探测成功解除——
        // 那可能是另一个端点/另一把 key 的成功，不是本端点恢复的证明。
        if state.cooldown_until.is_none_or(|until| now >= until) {
            state.level_secs = (state.level_secs * 0.75).max(config.floor_secs);
        }
    }

    /// 距冷却结束的剩余时间；不在冷却期为 `None`。
    pub async fn cooldown_remaining(&self, key: &str) -> Option<Duration> {
        let states = self.states.read().await;
        states
            .get(key)
            .and_then(|state| state.cooldown_until)
            .and_then(|until| until.checked_duration_since(Instant::now()))
    }

    /// 全量快照（UI 命令用）。
    pub async fn snapshot(&self) -> Vec<RateLimitSnapshot> {
        let states = self.states.read().await;
        let owners = self.owners.read().await;
        let now = Instant::now();

        let mut out: Vec<RateLimitSnapshot> = states
            .iter()
            .map(|(key, state)| {
                let cooldown_remaining_ms = state
                    .cooldown_until
                    .and_then(|until| until.checked_duration_since(now))
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0);
                RateLimitSnapshot {
                    key: key.clone(),
                    owners: owners
                        .get(key)
                        .map(|set| {
                            set.iter()
                                .map(|(app_type, provider_id)| RateLimitOwner {
                                    app_type: app_type.clone(),
                                    provider_id: provider_id.clone(),
                                })
                                .collect()
                        })
                        .unwrap_or_default(),
                    level_secs: state.level_secs.ceil() as u64,
                    cooldown_remaining_ms,
                    recent_429_count: {
                        let mut window = state.window.clone();
                        let count = {
                            while let Some(&front) = window.front() {
                                if now.duration_since(front) <= WINDOW {
                                    break;
                                }
                                window.pop_front();
                            }
                            window.len()
                        };
                        count as u64
                    },
                }
            })
            .collect();
        out.sort_by_key(|snapshot| std::cmp::Reverse(snapshot.cooldown_remaining_ms));
        out
    }
}

impl Default for AdaptiveRateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

/// 解析上游 `Retry-After` 响应头。RFC 7231 允许两种形式：
/// - 秒数（`120`）
/// - HTTP-date（`Wed, 21 Oct 2026 07:28:00 GMT`）
///
/// 解析失败（垃圾值、过去的日期）返回 `None`，调用方回落到算法自身档位。
pub fn parse_retry_after_header(value: &str) -> Option<Duration> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    if let Ok(secs) = value.parse::<u64>() {
        return Some(Duration::from_secs(secs));
    }
    // HTTP-date：按相对时刻换算成剩余时长。
    let date = chrono::DateTime::parse_from_rfc2822(value).ok()?;
    let delta = date.timestamp() - chrono::Utc::now().timestamp();
    Some(Duration::from_secs(delta.max(0) as u64))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestHarness {
        limiter: AdaptiveRateLimiter,
        base: Instant,
    }

    impl TestHarness {
        fn new() -> Self {
            Self {
                limiter: AdaptiveRateLimiter::new(),
                base: Instant::now(),
            }
        }

        fn at(&self, secs: u64) -> Instant {
            self.base + Duration::from_secs(secs)
        }
    }

    const KEY: &str = "claude:p1";
    const APP: &str = "claude";
    const PID: &str = "p1";

    #[tokio::test]
    async fn first_429_starts_cooldown_above_floor() {
        let h = TestHarness::new();
        // 首个 429：初始档位 = floor(2) × 1.5 = 3
        let client_wait = h.limiter.note_429_at(KEY, APP, PID, None, h.at(0)).await;
        assert_eq!(client_wait, 3);

        assert_eq!(
            h.limiter.acquire_at(KEY, APP, PID, h.at(0)).await,
            Admission::Deny {
                retry_after_secs: 3
            }
        );
        // 冷却结束后放行（成为探测）。
        assert_eq!(
            h.limiter.acquire_at(KEY, APP, PID, h.at(3)).await,
            Admission::Allow
        );
    }

    #[tokio::test]
    async fn level_escalates_with_density_and_hints_as_floor() {
        let h = TestHarness::new();

        // 密度 0（首个 429）：2 × 1.5 = 3
        let wait1 = h.limiter.note_429_at(KEY, APP, PID, None, h.at(0)).await;
        assert_eq!(wait1, 3);

        // 上游 hint 比算出来的档位大 → 取 hint
        let wait2 = h
            .limiter
            .note_429_at(KEY, APP, PID, Some(Duration::from_secs(30)), h.at(1))
            .await;
        assert_eq!(wait2, 30);

        // 密度加权升级能盖过已有档位：level 已是 30，× 2 = 60（cap）
        let wait3 = h.limiter.note_429_at(KEY, APP, PID, None, h.at(2)).await;
        assert_eq!(wait3, 60);
    }

    #[tokio::test]
    async fn level_is_clamped_to_cap() {
        let h = TestHarness::new();
        for i in 0..20u64 {
            let wait = h.limiter.note_429_at(KEY, APP, PID, None, h.at(i)).await;
            assert!(wait <= 60, "第 {i} 次 429 后档位超上限: {wait}");
        }
    }

    #[tokio::test]
    async fn probe_success_halves_level_and_reopens() {
        let h = TestHarness::new();
        h.limiter
            .note_429_at(KEY, APP, PID, Some(Duration::from_secs(30)), h.at(0))
            .await;

        // 冷却内拒绝，到期成为探测
        assert!(matches!(
            h.limiter.acquire_at(KEY, APP, PID, h.at(1)).await,
            Admission::Deny { .. }
        ));
        assert_eq!(
            h.limiter.acquire_at(KEY, APP, PID, h.at(30)).await,
            Admission::Allow
        );
        // 探测在途时其余请求仍被拒
        assert!(matches!(
            h.limiter.acquire_at(KEY, APP, PID, h.at(31)).await,
            Admission::Deny { .. }
        ));

        // 探测成功 → 退出冷却、档位减半（30 → 15）
        h.limiter.note_success_at(KEY, APP, h.at(32)).await;
        assert_eq!(
            h.limiter.acquire_at(KEY, APP, PID, h.at(32)).await,
            Admission::Allow
        );
        assert!(h.limiter.cooldown_remaining(KEY).await.is_none());

        // 下次 429 从减半后的档位起步（t=70，旧 429 已滑出 60s 窗口，无密度加权）：
        // 15 × 1.5 = 22.5 → ceil 23
        let wait = h.limiter.note_429_at(KEY, APP, PID, None, h.at(70)).await;
        assert_eq!(wait, 23);
    }

    #[tokio::test]
    async fn non_probe_success_decays_level_but_keeps_cooldown() {
        let h = TestHarness::new();
        h.limiter
            .note_429_at(KEY, APP, PID, Some(Duration::from_secs(30)), h.at(0))
            .await;

        // 冷却期内的"成功"来自别处（failover 到另一家），不应解除本端点冷却
        h.limiter.note_success_at(KEY, APP, h.at(5)).await;
        assert!(matches!(
            h.limiter.acquire_at(KEY, APP, PID, h.at(5)).await,
            Admission::Deny { .. }
        ));
    }

    #[tokio::test]
    async fn stagger_spreads_parallel_denials() {
        let h = TestHarness::new();
        h.limiter.note_429_at(KEY, APP, PID, None, h.at(0)).await; // level 3

        // 同一时刻 8 个并发请求被拒：间隔应在 剩余 + {0, L/4, L/2, 3L/4} 间循环
        let mut seen = std::collections::HashSet::new();
        for _ in 0..8 {
            match h.limiter.acquire_at(KEY, APP, PID, h.at(0)).await {
                Admission::Deny { retry_after_secs } => {
                    assert!(retry_after_secs >= 3 && retry_after_secs <= 6);
                    seen.insert(retry_after_secs);
                }
                Admission::Allow => panic!("冷却期内不应放行"),
            }
        }
        assert_eq!(seen.len(), 4, "打散后应有 4 个不同槽位，实际 {seen:?}");
    }

    #[tokio::test]
    async fn success_only_traffic_never_creates_state() {
        let h = TestHarness::new();
        h.limiter.note_success_at(KEY, APP, h.at(0)).await;
        assert_eq!(
            h.limiter.acquire_at(KEY, APP, PID, h.at(0)).await,
            Admission::Allow
        );
        assert!(h.limiter.snapshot().await.is_empty());
    }

    #[tokio::test]
    async fn disabled_config_falls_back_to_hint_or_legacy_default() {
        let h = TestHarness::new();
        h.limiter
            .update_config(
                APP,
                RateLimitBackoffConfig {
                    enabled: false,
                    ..Default::default()
                },
            )
            .await;

        // 不建状态、不打冷却
        let wait = h
            .limiter
            .note_429_at(KEY, APP, PID, Some(Duration::from_secs(7)), h.at(0))
            .await;
        assert_eq!(wait, 7);
        assert_eq!(
            h.limiter.acquire_at(KEY, APP, PID, h.at(0)).await,
            Admission::Allow
        );
        // 无 hint 时回落旧固定值
        let wait = h.limiter.note_429_at(KEY, APP, PID, None, h.at(0)).await;
        assert_eq!(wait, 10);
    }

    #[tokio::test]
    async fn snapshot_reports_owners_and_window() {
        let h = TestHarness::new();
        h.limiter.note_429_at(KEY, APP, PID, None, h.at(0)).await;
        h.limiter
            .note_429_at("claude:p2", APP, "p2", None, h.at(1))
            .await;

        let snap = h.limiter.snapshot().await;
        assert_eq!(snap.len(), 2);
        let p1 = snap.iter().find(|s| s.key == KEY).unwrap();
        assert_eq!(
            p1.owners,
            vec![RateLimitOwner {
                app_type: APP.into(),
                provider_id: PID.into()
            }]
        );
        assert_eq!(p1.recent_429_count, 1);
        assert!(p1.cooldown_remaining_ms > 0);
    }

    #[tokio::test]
    async fn window_expires_old_entries() {
        let h = TestHarness::new();
        h.limiter.note_429_at(KEY, APP, PID, None, h.at(0)).await;
        // 61s 后窗口清空，密度从 0 重新计
        h.limiter.note_429_at(KEY, APP, PID, None, h.at(61)).await;
        let snap = h.limiter.snapshot().await;
        let p1 = snap.iter().find(|s| s.key == KEY).unwrap();
        // level 已是 3（2×1.5），×1.5 = 4.5 → 5
        assert_eq!(p1.level_secs, 5);
        assert_eq!(p1.recent_429_count, 1);
    }
}
