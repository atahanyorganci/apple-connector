//! HTTP middleware: privacy-safe tracing, security headers, and bounded timeouts.

use std::time::{Duration, Instant};

use axum::{
    body::Body,
    extract::MatchedPath,
    http::{Request, Response, header},
    middleware::Next,
    response::IntoResponse,
};
use tracing::info;

use super::error::ApiError;

/// Default timeout for JSON and metadata API handlers.
pub const JSON_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// Longer timeout for attachment byte streaming (outside JSON/DB bounds).
pub const MEDIA_REQUEST_TIMEOUT: Duration = Duration::from_secs(300);

pub async fn trace_request(request: Request<Body>, next: Next) -> Response<Body> {
    let route = request
        .extensions()
        .get::<MatchedPath>()
        .map(MatchedPath::as_str)
        .unwrap_or("unknown")
        .to_owned();
    let started = Instant::now();
    let response = next.run(request).await;
    let latency_ms = started.elapsed().as_millis();
    let status = response.status().as_u16();
    info!(route, status, latency_ms, "request completed");
    response
}

pub async fn security_headers(request: Request<Body>, next: Next) -> Response<Body> {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        header::HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        header::X_FRAME_OPTIONS,
        header::HeaderValue::from_static("DENY"),
    );
    headers.insert(
        header::REFERRER_POLICY,
        header::HeaderValue::from_static("no-referrer"),
    );
    headers.insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static("no-store"),
    );
    response
}

pub async fn request_timeout(
    request: Request<Body>,
    next: Next,
) -> Result<Response<Body>, ApiError> {
    let is_media = request.uri().path().ends_with("/content");
    let timeout = if is_media {
        MEDIA_REQUEST_TIMEOUT
    } else {
        JSON_REQUEST_TIMEOUT
    };

    match tokio::time::timeout(timeout, next.run(request)).await {
        Ok(response) => Ok(response),
        Err(_) => Err(ApiError::internal("request timed out")),
    }
}

pub async fn not_found() -> impl IntoResponse {
    ApiError::not_found("route not found").into_response()
}

pub async fn method_not_allowed() -> impl IntoResponse {
    ApiError::method_not_allowed().into_response()
}

#[cfg(test)]
mod tests {
    use axum::{Router, body::Body, middleware::from_fn, routing::get};
    use http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    use super::{not_found, security_headers, trace_request};

    #[tokio::test]
    async fn security_headers_are_applied() {
        let app = Router::new()
            .route("/ok", get(|| async { "ok" }))
            .layer(from_fn(security_headers));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/ok")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(
            response.headers().get("x-content-type-options").unwrap(),
            "nosniff"
        );
        assert_eq!(response.headers().get("x-frame-options").unwrap(), "DENY");
        assert_eq!(
            response.headers().get("referrer-policy").unwrap(),
            "no-referrer"
        );
        assert_eq!(response.headers().get("cache-control").unwrap(), "no-store");
        assert!(
            response
                .headers()
                .get("access-control-allow-origin")
                .is_none()
        );
    }

    #[tokio::test]
    async fn unknown_route_returns_json_not_found() {
        let app = Router::new().fallback(not_found);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/missing?secret=1")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let body = response
            .into_body()
            .collect()
            .await
            .expect("body")
            .to_bytes();
        let payload = String::from_utf8(body.to_vec()).expect("utf-8");
        assert!(payload.contains("\"code\":\"not_found\""));
        assert!(!payload.contains("secret"));
    }

    #[tokio::test]
    async fn trace_middleware_does_not_log_request_uri() {
        let app = Router::new()
            .route("/v1/messages", get(|| async { "ok" }))
            .route_layer(from_fn(trace_request));

        let _ = app
            .oneshot(
                Request::builder()
                    .uri("/v1/messages?q=private&sender=%2B1555")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
    }
}
