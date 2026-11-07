use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProxyError {
    #[error("上游响应体超过大小上限: {0} 字节")]
    ResponseBodyTooLarge(usize),

    #[error("服务器已在运行")]
    AlreadyRunning,

    #[error("服务器未运行")]
    NotRunning,

    #[error("地址绑定失败: {0}")]
    BindFailed(String),

    #[error("停止超时")]
    StopTimeout,

    #[error("停止失败: {0}")]
    StopFailed(String),

    #[error("请求转发失败: {0}")]
    ForwardFailed(String),

    #[error("无可用的Provider")]
    NoAvailableProvider,

    #[error("所有供应商已熔断，无可用渠道")]
    AllProvidersCircuitOpen,

    #[error("所有供应商都已停用，无可用渠道")]
    AllProvidersDisabled,

    #[error("未配置供应商")]
    NoProvidersConfigured,

    #[allow(dead_code)]
    #[error("Provider不健康: {0}")]
    ProviderUnhealthy(String),

    #[error("上游错误 (状态码 {status}): {body:?}")]
    UpstreamError { status: u16, body: Option<String> },

    /// 上游 429（限流/额度）。与 [`ProxyError::UpstreamError`] 的区别：
    /// 转发层此时已把这次 429 喂给自适应退避控制器
    /// （[`crate::proxy::adaptive_rate_limiter::AdaptiveRateLimiter`]），
    /// `retry_after_secs` 是算出来的客户端重试间隔——上游 `Retry-After` 头、
    /// 近 60s 的 429 密度、当前退避档位共同决定，取代旧的固定 10 秒。
    #[error("上游限流 (429)，建议 {retry_after_secs}s 后重试: {body:?}")]
    RateLimited {
        retry_after_secs: u64,
        body: Option<String>,
    },

    #[error("超过最大重试次数")]
    MaxRetriesExceeded,

    #[error("数据库错误: {0}")]
    DatabaseError(String),

    #[error("配置错误: {0}")]
    ConfigError(String),

    #[allow(dead_code)]
    #[error("格式转换错误: {0}")]
    TransformError(String),

    #[allow(dead_code)]
    #[error("无效的请求: {0}")]
    InvalidRequest(String),

    #[error("超时: {0}")]
    Timeout(String),

    /// 流式响应空闲超时
    #[allow(dead_code)]
    #[error("流式响应空闲超时: {0}秒无数据")]
    StreamIdleTimeout(u64),

    /// 认证错误
    #[error("认证失败: {0}")]
    AuthError(String),

    /// Local pre-exhaustion policy, never an upstream usage-limit response.
    #[error("Codex 官方额度保护 (quota protection): {0}")]
    CodexQuotaProtection(String),

    #[allow(dead_code)]
    #[error("内部错误: {0}")]
    Internal(String),
}

impl IntoResponse for ProxyError {
    fn into_response(self) -> Response {
        // 上游用 HTTP 2xx 带错误信封（`response.failed` / `{"error":{...}}`）报告
        // 过载、限流或额度用尽时，上游路径会把它包成 TransformError，客户端拿到的
        // 是 422「格式转换错误」——一个不可重试的客户端错误，回合直接失败。
        // 这类失败其实是上游暂时不可用，改写成 429，让 Claude / Codex 用它们自己的
        // 退避重试继续（Claude 对 429 会退避重试多次）。
        if let Some((status, message)) = upstream_retryable_message(&self) {
            log::warn!(
                "[proxy] 上游暂时失败，改写为 {} 交由客户端退避重试: {message}",
                status.as_u16()
            );
            // 限流/过载用 rate_limit_error；上游内部错误（503）用 api_error。
            let error_type = if status == StatusCode::TOO_MANY_REQUESTS {
                "rate_limit_error"
            } else {
                "api_error"
            };
            let mut response = (
                status,
                Json(json!({
                    "type": "error",
                    "error": {
                        "type": error_type,
                        "message": message,
                    }
                })),
            )
                .into_response();
            apply_busy_retry_headers(&mut response, None);
            return response;
        }

        let mut retry_after_override: Option<u64> = None;
        let (status, body) = match &self {
            ProxyError::UpstreamError {
                status: upstream_status,
                body: upstream_body,
            } => (
                StatusCode::from_u16(*upstream_status).unwrap_or(StatusCode::BAD_GATEWAY),
                upstream_error_body(upstream_body, *upstream_status),
            ),
            ProxyError::RateLimited {
                retry_after_secs,
                body: upstream_body,
            } => {
                // 上游 429 走自适应退避：透传上游错误体，retry-after 用算出的值。
                retry_after_override = Some(*retry_after_secs);
                (
                    StatusCode::TOO_MANY_REQUESTS,
                    upstream_error_body(upstream_body, 429),
                )
            }
            _ => {
                let (http_status, message) = match &self {
                    ProxyError::AlreadyRunning => (StatusCode::CONFLICT, self.to_string()),
                    ProxyError::NotRunning => (StatusCode::SERVICE_UNAVAILABLE, self.to_string()),
                    ProxyError::BindFailed(_) => {
                        (StatusCode::INTERNAL_SERVER_ERROR, self.to_string())
                    }
                    ProxyError::StopTimeout => {
                        (StatusCode::INTERNAL_SERVER_ERROR, self.to_string())
                    }
                    ProxyError::StopFailed(_) => {
                        (StatusCode::INTERNAL_SERVER_ERROR, self.to_string())
                    }
                    ProxyError::ForwardFailed(_) => (StatusCode::BAD_GATEWAY, self.to_string()),
                    ProxyError::NoAvailableProvider => {
                        (StatusCode::SERVICE_UNAVAILABLE, self.to_string())
                    }
                    ProxyError::AllProvidersCircuitOpen => {
                        (StatusCode::SERVICE_UNAVAILABLE, self.to_string())
                    }
                    ProxyError::AllProvidersDisabled => {
                        (StatusCode::SERVICE_UNAVAILABLE, self.to_string())
                    }
                    ProxyError::NoProvidersConfigured => {
                        (StatusCode::SERVICE_UNAVAILABLE, self.to_string())
                    }
                    ProxyError::ProviderUnhealthy(_) => {
                        (StatusCode::SERVICE_UNAVAILABLE, self.to_string())
                    }
                    ProxyError::MaxRetriesExceeded => {
                        (StatusCode::SERVICE_UNAVAILABLE, self.to_string())
                    }
                    ProxyError::DatabaseError(_) => {
                        (StatusCode::INTERNAL_SERVER_ERROR, self.to_string())
                    }
                    ProxyError::ConfigError(_) => (StatusCode::BAD_REQUEST, self.to_string()),
                    ProxyError::TransformError(_) => {
                        (StatusCode::UNPROCESSABLE_ENTITY, self.to_string())
                    }
                    ProxyError::InvalidRequest(_) => (StatusCode::BAD_REQUEST, self.to_string()),
                    ProxyError::Timeout(_) => (StatusCode::GATEWAY_TIMEOUT, self.to_string()),
                    ProxyError::StreamIdleTimeout(_) => {
                        (StatusCode::GATEWAY_TIMEOUT, self.to_string())
                    }
                    ProxyError::AuthError(_) => (StatusCode::UNAUTHORIZED, self.to_string()),
                    ProxyError::CodexQuotaProtection(_) => {
                        (StatusCode::FORBIDDEN, self.to_string())
                    }
                    ProxyError::Internal(_) => {
                        (StatusCode::INTERNAL_SERVER_ERROR, self.to_string())
                    }
                    ProxyError::ResponseBodyTooLarge(_) => {
                        (StatusCode::BAD_GATEWAY, self.to_string())
                    }
                    ProxyError::UpstreamError { .. } | ProxyError::RateLimited { .. } => {
                        unreachable!()
                    }
                };

                let error_body = json!({
                    "error": {
                        "message": message,
                        "type": "proxy_error",
                    }
                });

                (http_status, error_body)
            }
        };

        let mut response = (status, Json(body)).into_response();
        // 上游自己回的 429（限流/额度）原来原样透传、没有 retry-after，客户端就会
        // 用默认的 500ms 起指数重试，几秒一次把上游锤得更狠。统一补上退避头；
        // 走自适应退避控制器（RateLimited）时用算出的间隔，其余路径保持旧固定值。
        if status == StatusCode::TOO_MANY_REQUESTS {
            apply_busy_retry_headers(&mut response, retry_after_override);
        }
        response
    }
}

/// 把上游错误体规范化成响应 JSON：JSON 透传，非 JSON 包一层，空体给兜底文案。
fn upstream_error_body(upstream_body: &Option<String>, status: u16) -> serde_json::Value {
    if let Some(body_str) = upstream_body {
        if let Ok(json_body) = serde_json::from_str::<serde_json::Value>(body_str) {
            // 上游返回的是 JSON，直接透传
            return json_body;
        }
        // 上游返回的不是 JSON，包装为错误消息
        return json!({
            "error": {
                "message": body_str,
                "type": "upstream_error",
            }
        });
    }
    json!({
        "error": {
            "message": format!("Upstream error (status {})", status),
            "type": "upstream_error",
        }
    })
}

/// 给 429 补上客户端认得的退避头：
/// - `retry-after`：下一次重试前至少等多久（秒，也接受 HTTP-date）
/// - `x-should-retry: true`：强制按可重试处理
///
/// `retry_after_secs` 来自自适应退避控制器（`RateLimited` 携带）；没有上下文的
/// 路径（如 2xx 错误信封改写）保持旧的固定值兜底。
///
/// 注意别把 retry-after 设太大：Claude 客户端对 429/529 有一条保护，等待时间
/// 超过它的阈值（60 秒）就把响应标成 x-cc-retry-after-too-long 并设
/// x-should-retry: false，直接放弃重试。自适应控制器的档位上限默认就是 60，
/// 配置层保证这个约束，这里不再二次钳制。
const BUSY_RETRY_AFTER_SECS: u64 = 10;

pub(crate) fn apply_busy_retry_headers(response: &mut Response, retry_after_secs: Option<u64>) {
    let secs = retry_after_secs.unwrap_or(BUSY_RETRY_AFTER_SECS);
    if let Ok(value) = axum::http::HeaderValue::from_str(&secs.to_string()) {
        response
            .headers_mut()
            .insert(axum::http::header::RETRY_AFTER, value);
    }
    response.headers_mut().insert(
        axum::http::HeaderName::from_static("x-should-retry"),
        axum::http::HeaderValue::from_static("true"),
    );
}

/// 上游「暂时性失败」类错误：过载、限流、额度用尽，以及上游自己的内部错误。
///
/// 上游用 2xx + 错误信封报告这些状态时，代理会把它包成 `TransformError`（422）。
/// 对客户端来说它们本来是暂时性的、可重试的：
/// - 限流 / 过载 / 额度用尽 → 429（rate_limit_error）
/// - 上游内部错误（含 OpenAI 的 "An error occurred while processing your request"）→ 503
pub(crate) fn upstream_retryable_message(error: &ProxyError) -> Option<(StatusCode, String)> {
    let ProxyError::TransformError(message) = error else {
        return None;
    };
    let lower = message.to_ascii_lowercase();
    const RATE_LIMIT: [&str; 8] = [
        "overloaded",
        "over capacity",
        "try again later",
        "rate limit",
        "usage limit",
        "usage_limit",
        "server_error",
        "temporarily unavailable",
    ];
    // 上游崩溃/内部错误的兜底文案。OpenAI 自己的说法是「可以重试」。
    const SERVER_ERROR: [&str; 2] = [
        "an error occurred while processing your request",
        "internal server error",
    ];
    if RATE_LIMIT.iter().any(|needle| lower.contains(needle)) {
        return Some((StatusCode::TOO_MANY_REQUESTS, message.clone()));
    }
    if SERVER_ERROR.iter().any(|needle| lower.contains(needle)) {
        return Some((StatusCode::SERVICE_UNAVAILABLE, message.clone()));
    }
    None
}

/// 错误分类
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    /// 可重试错误（网络问题、5xx）
    Retryable, // 网络超时、5xx 错误
    /// 不可重试错误（4xx、认证失败）
    NonRetryable, // 认证失败、参数错误、4xx 错误
    #[allow(dead_code)]
    ClientAbort, // 客户端主动中断
}

/// 把错误连同 `source()` 原因链拼成一行。
///
/// reqwest / hyper 的外层文本只有「error sending request」「error decoding response body」
/// 这类描述，真正能区分 DNS、TCP、TLS、连接被重置的信息都在原因链里。
pub fn describe_error_chain(error: &(dyn std::error::Error + 'static)) -> String {
    let mut text = error.to_string();
    let mut source = error.source();
    while let Some(cause) = source {
        let cause_text = cause.to_string();
        // 有的错误会把原因也写进自己的文本，重复的就不再追加
        if !cause_text.is_empty() && !text.ends_with(&cause_text) {
            text.push_str(": ");
            text.push_str(&cause_text);
        }
        source = cause.source();
    }
    text
}

/// reqwest 错误的文本带完整 URL（查询参数里可能有 Key），先去掉 URL 再展开原因链。
pub fn describe_reqwest_error(error: reqwest::Error) -> String {
    describe_error_chain(&error.without_url())
}

/// 判断错误是否可重试
#[allow(dead_code)]
pub fn categorize_error(error: &reqwest::Error) -> ErrorCategory {
    if error.is_timeout() || error.is_connect() {
        return ErrorCategory::Retryable;
    }

    if let Some(status) = error.status() {
        if status.is_server_error() {
            ErrorCategory::Retryable
        } else if status.is_client_error() {
            ErrorCategory::NonRetryable
        } else {
            ErrorCategory::Retryable
        }
    } else {
        ErrorCategory::Retryable
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct Outer(std::io::Error);

    impl std::fmt::Display for Outer {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("error sending request")
        }
    }

    impl std::error::Error for Outer {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            Some(&self.0)
        }
    }

    #[test]
    fn describe_error_chain_appends_every_cause() {
        let error = Outer(std::io::Error::new(
            std::io::ErrorKind::ConnectionRefused,
            "tcp connect error",
        ));
        assert_eq!(
            describe_error_chain(&error),
            "error sending request: tcp connect error"
        );
    }

    #[tokio::test]
    async fn describe_reqwest_error_keeps_cause_and_drops_url() {
        // 先占一个端口再释放，保证连接被拒绝
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);

        let error = reqwest::Client::new()
            .get(format!(
                "http://127.0.0.1:{port}/v1/responses?key=secret-token"
            ))
            .send()
            .await
            .expect_err("connection should be refused");
        assert!(error.is_connect());

        let text = describe_reqwest_error(error);
        assert!(text.starts_with("error sending request: "), "{text}");
        assert!(!text.contains("secret-token"), "{text}");
    }
}
