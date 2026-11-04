//! 错误类型到 HTTP 状态码的映射
//!
//! 将 ProxyError 映射到合适的 HTTP 状态码，用于日志记录和手动构建错误响应

use super::ProxyError;

/// 将 ProxyError 映射到 HTTP 状态码
///
/// 映射规则：
/// - 上游错误：直接使用上游返回的状态码
/// - 超时：504 Gateway Timeout
/// - 连接失败：502 Bad Gateway
/// - 无可用 Provider：503 Service Unavailable
/// - 重试耗尽：503 Service Unavailable
/// - 认证错误：401 Unauthorized
/// - 配置/请求错误：400 Bad Request
/// - 转换错误：422 Unprocessable Entity
/// - 其他错误：500 Internal Server Error
pub fn map_proxy_error_to_status(error: &ProxyError) -> u16 {
    // 上游用 2xx + 错误信封报告过载/限流时会被包成 TransformError；对客户端应按
    // 可重试的 429 处理，而不是 422（与 `ProxyError::into_response` 保持一致）。
    if let Some((status, _)) = super::error::upstream_retryable_message(error) {
        return status.as_u16();
    }
    match error {
        // 服务状态错误：与 IntoResponse 保持一致
        ProxyError::AlreadyRunning => 409,
        ProxyError::NotRunning => 503,

        // 上游错误：使用实际状态码
        ProxyError::UpstreamError { status, .. } => *status,

        // 超时错误：504 Gateway Timeout
        ProxyError::Timeout(_) | ProxyError::StreamIdleTimeout(_) => 504,

        // 转发失败/连接失败：502 Bad Gateway
        ProxyError::ForwardFailed(_) => 502,

        // 无可用 Provider：503 Service Unavailable
        ProxyError::NoAvailableProvider => 503,

        // 所有供应商已熔断：503 Service Unavailable
        ProxyError::AllProvidersCircuitOpen => 503,

        // 未配置供应商：503 Service Unavailable
        ProxyError::NoProvidersConfigured => 503,

        // 重试耗尽：503 Service Unavailable
        ProxyError::MaxRetriesExceeded => 503,

        // Provider 不健康：503 Service Unavailable
        ProxyError::ProviderUnhealthy(_) => 503,

        // 配置错误/无效请求：400 Bad Request
        ProxyError::ConfigError(_) | ProxyError::InvalidRequest(_) => 400,

        // 认证错误：401 Unauthorized
        ProxyError::AuthError(_) => 401,
        ProxyError::CodexQuotaProtection(_) => 403,

        // 数据库错误：500 Internal Server Error
        ProxyError::DatabaseError(_) => 500,

        // 转换错误：422 Unprocessable Entity
        ProxyError::TransformError(_) => 422,

        // 其他未知错误：500 Internal Server Error
        _ => 500,
    }
}

/// 将 ProxyError 转换为用户友好的错误消息
pub fn get_error_message(error: &ProxyError) -> String {
    match error {
        ProxyError::UpstreamError { status, body } => {
            // 上游没给内容时要明说，否则用户会以为是 CC Switch 把错误内容吞了（#6895）
            let body = body
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .unwrap_or("上游未返回错误内容");
            format!("上游错误 ({status}): {body}")
        }
        ProxyError::Timeout(msg) => format!("请求超时: {msg}"),
        ProxyError::ForwardFailed(msg) => format!("转发失败: {msg}"),
        ProxyError::NoAvailableProvider => "无可用 Provider".to_string(),
        ProxyError::AllProvidersCircuitOpen => "所有供应商已熔断，无可用渠道".to_string(),
        ProxyError::NoProvidersConfigured => "未配置供应商".to_string(),
        ProxyError::MaxRetriesExceeded => "所有 Provider 都失败，重试耗尽".to_string(),
        ProxyError::ProviderUnhealthy(msg) => format!("Provider 不健康: {msg}"),
        ProxyError::DatabaseError(msg) => format!("数据库错误: {msg}"),
        ProxyError::TransformError(msg) => format!("请求/响应转换错误: {msg}"),
        _ => error.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_upstream_error() {
        let error = ProxyError::UpstreamError {
            status: 401,
            body: Some("Unauthorized".to_string()),
        };
        assert_eq!(map_proxy_error_to_status(&error), 401);
    }

    #[test]
    fn test_map_timeout_error() {
        let error = ProxyError::Timeout("Request timeout".to_string());
        assert_eq!(map_proxy_error_to_status(&error), 504);
    }

    #[test]
    fn test_map_connection_error() {
        let error = ProxyError::ForwardFailed("Connection refused".to_string());
        assert_eq!(map_proxy_error_to_status(&error), 502);
    }

    #[test]
    fn test_map_no_provider_error() {
        let error = ProxyError::NoAvailableProvider;
        assert_eq!(map_proxy_error_to_status(&error), 503);
    }

    #[test]
    fn test_map_status_matches_proxy_error_response_semantics() {
        assert_eq!(
            map_proxy_error_to_status(&ProxyError::CodexQuotaProtection("protected".to_string())),
            403
        );
        assert_eq!(
            map_proxy_error_to_status(&ProxyError::AuthError("bad token".to_string())),
            401
        );
        assert_eq!(
            map_proxy_error_to_status(&ProxyError::ConfigError("bad config".to_string())),
            400
        );
        assert_eq!(
            map_proxy_error_to_status(&ProxyError::InvalidRequest("bad request".to_string())),
            400
        );
        assert_eq!(
            map_proxy_error_to_status(&ProxyError::TransformError("bad transform".to_string())),
            422
        );
        assert_eq!(
            map_proxy_error_to_status(&ProxyError::StreamIdleTimeout(30)),
            504
        );
    }

    #[test]
    fn test_get_error_message() {
        let error = ProxyError::UpstreamError {
            status: 500,
            body: Some("Internal Server Error".to_string()),
        };
        let msg = get_error_message(&error);
        assert!(msg.contains("上游错误"));
        assert!(msg.contains("500"));
        assert!(msg.contains("Internal Server Error"));
    }

    #[test]
    fn test_get_error_message_empty_body() {
        let error_empty = ProxyError::UpstreamError {
            status: 502,
            body: Some("   ".to_string()),
        };
        assert_eq!(
            get_error_message(&error_empty),
            "上游错误 (502): 上游未返回错误内容"
        );

        let error_none = ProxyError::UpstreamError {
            status: 502,
            body: None,
        };
        assert_eq!(
            get_error_message(&error_none),
            "上游错误 (502): 上游未返回错误内容"
        );
    }
}

#[cfg(test)]
mod overload_mapping_tests {
    use super::*;
    use axum::response::IntoResponse;

    #[test]
    fn overload_transform_error_is_retryable_429() {
        let overload = ProxyError::TransformError(
            "Responses upstream server_error: Our servers are currently overloaded. \
             Please try again later."
                .to_string(),
        );
        assert_eq!(map_proxy_error_to_status(&overload), 429);
        let response = overload.into_response();
        assert_eq!(response.status(), 429);
        // 客户端认这两个头：等 5 秒再重试，并强制按可重试处理
        assert_eq!(
            response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok()),
            Some("10")
        );
        assert_eq!(
            response
                .headers()
                .get("x-should-retry")
                .and_then(|v| v.to_str().ok()),
            Some("true")
        );

        // 真正不可重试的转换错误（上下文超限）仍然是 422
        let context = ProxyError::TransformError(
            "Your input exceeds the context window of this model.".to_string(),
        );
        assert_eq!(map_proxy_error_to_status(&context), 422);
        assert_eq!(context.into_response().status(), 422);
    }

    #[test]
    fn upstream_429_passthrough_carries_retry_headers() {
        let upstream = ProxyError::UpstreamError {
            status: 429,
            body: Some("{\"detail\":\"Rate limit exceeded\"}".to_string()),
        };
        let response = upstream.into_response();
        assert_eq!(response.status(), 429);
        assert_eq!(
            response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok()),
            Some("10")
        );
        assert_eq!(
            response
                .headers()
                .get("x-should-retry")
                .and_then(|v| v.to_str().ok()),
            Some("true")
        );

        // 非 429 的上游错误不补退避头
        let bad_request = ProxyError::UpstreamError {
            status: 400,
            body: None,
        };
        assert!(bad_request
            .into_response()
            .headers()
            .get("retry-after")
            .is_none());
    }

    #[test]
    fn generic_upstream_error_is_retryable_503() {
        let error = ProxyError::TransformError(
            "An error occurred while processing your request. You can retry your \
             request, or contact us through our help center at help.openai.com if \
             the error persists. Please include the request ID \
             772c14f2-2ebe-46ec-b174-e41f0b43a10b in your message."
                .to_string(),
        );
        assert_eq!(map_proxy_error_to_status(&error), 503);
        let response = error.into_response();
        assert_eq!(response.status(), 503);
        assert_eq!(
            response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok()),
            Some("10")
        );
        assert_eq!(
            response
                .headers()
                .get("x-should-retry")
                .and_then(|v| v.to_str().ok()),
            Some("true")
        );
    }
}
