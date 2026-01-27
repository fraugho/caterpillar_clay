use axum::{
    body::Body,
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

use crate::routes::AppState;

/// Rate limiting middleware using Upstash Redis
pub async fn rate_limit_middleware(
    State(state): State<AppState>,
    req: Request<Body>,
    next: Next,
) -> Response {
    // Get client IP from X-Forwarded-For header (for proxied requests) or X-Real-IP
    let ip = req
        .headers()
        .get("x-forwarded-for")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(|s| s.trim().to_string())
        .or_else(|| {
            req.headers()
                .get("x-real-ip")
                .and_then(|h| h.to_str().ok())
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| "unknown".to_string());

    // Check rate limit if rate limiter is configured
    if let Some(ref rate_limiter) = state.rate_limiter {
        match rate_limiter.check_rate_limit(&ip).await {
            Ok(allowed) => {
                if !allowed {
                    tracing::warn!("Rate limit exceeded for IP: {}", ip);
                    return (
                        StatusCode::TOO_MANY_REQUESTS,
                        Json(json!({
                            "error": "Too many requests",
                            "retry_after": 60
                        })),
                    )
                        .into_response();
                }
            }
            Err(e) => {
                // Log error but allow request through (fail open)
                tracing::error!("Rate limiter error: {} - allowing request", e);
            }
        }
    }

    next.run(req).await
}
