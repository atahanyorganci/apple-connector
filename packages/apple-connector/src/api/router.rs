use std::{path::PathBuf, sync::Arc};

use axum::{Router, middleware::from_fn};
use sqlx::SqlitePool;
use utoipa::{OpenApi, openapi::OpenApi as OpenApiSpec};
use utoipa_axum::{router::OpenApiRouter, routes};
use utoipa_scalar::{Scalar, Servable};

use super::{
    doc::ApiDoc,
    middleware::{method_not_allowed, not_found, request_timeout, security_headers, trace_request},
};
use crate::messages::attachment_path::canonicalize_attachment_root;

#[derive(Clone)]
pub struct AppState {
    pub db: Option<SqlitePool>,
    pub openapi: Arc<OpenApiSpec>,
    pub attachment_root: Arc<PathBuf>,
}

impl AppState {
    pub fn new(db: Option<SqlitePool>) -> Self {
        Self::with_attachment_root(
            db,
            PathBuf::from("/var/empty/apple-connector-attachments-unconfigured"),
        )
    }

    pub fn with_attachment_root(db: Option<SqlitePool>, attachment_root: PathBuf) -> Self {
        let attachment_root =
            canonicalize_attachment_root(&attachment_root).unwrap_or(attachment_root);
        let openapi = Arc::new(build_openapi_spec());
        Self {
            db,
            openapi,
            attachment_root: Arc::new(attachment_root),
        }
    }
}

pub fn build_openapi_spec() -> OpenApiSpec {
    openapi_router().get_openapi().clone()
}

pub fn router(state: AppState) -> Router {
    let api: Router = openapi_router().with_state(state).into();
    api.merge(Scalar::with_url("/docs", build_openapi_spec()))
        .fallback(not_found)
        .method_not_allowed_fallback(method_not_allowed)
        .layer(from_fn(request_timeout))
        .layer(from_fn(security_headers))
        .route_layer(from_fn(trace_request))
}

fn openapi_router() -> OpenApiRouter<AppState> {
    OpenApiRouter::with_openapi(ApiDoc::openapi())
        .routes(routes!(crate::api::handlers::health::healthz))
        .routes(routes!(crate::api::handlers::chats::list_chats))
        .routes(routes!(crate::api::handlers::chats::get_chat))
        .routes(routes!(crate::api::handlers::chats::list_chat_messages))
        .routes(routes!(crate::api::handlers::messages::list_messages))
        .routes(routes!(crate::api::handlers::messages::get_message))
        .routes(routes!(crate::api::handlers::attachments::get_attachment))
        .routes(routes!(
            crate::api::handlers::attachments::get_attachment_content,
            crate::api::handlers::attachments::head_attachment_content,
        ))
        .routes(routes!(crate::api::handlers::openapi::get_openapi_spec))
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use http::{Method, Request, StatusCode};
    use tower::ServiceExt;

    use super::{AppState, router};

    const ROUTES: &[(&str, &str)] = &[
        ("GET", "/healthz"),
        ("GET", "/v1/chats"),
        ("GET", "/v1/chats/1"),
        ("GET", "/v1/chats/1/messages"),
        ("GET", "/v1/messages"),
        ("GET", "/v1/messages/test-guid"),
        ("GET", "/v1/attachments/test-guid"),
        ("GET", "/v1/attachments/test-guid/content"),
        ("HEAD", "/v1/attachments/test-guid/content"),
        ("GET", "/openapi.json"),
    ];

    #[tokio::test]
    async fn router_registers_every_contract_route() {
        let app = router(AppState::new(None));

        for (method, path) in ROUTES {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method(method.parse::<Method>().expect("method"))
                        .uri(*path)
                        .body(Body::empty())
                        .expect("request"),
                )
                .await
                .expect("response");

            assert_ne!(
                response.status(),
                StatusCode::NOT_FOUND,
                "missing route {method} {path}"
            );
        }
    }

    #[tokio::test]
    async fn unknown_route_returns_json_404_with_security_headers() {
        let app = router(AppState::new(None));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/secret/path?token=abc")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(
            response.headers().get("x-content-type-options").unwrap(),
            "nosniff"
        );
        assert!(
            response
                .headers()
                .get("access-control-allow-origin")
                .is_none()
        );
    }

    #[tokio::test]
    async fn unsupported_method_returns_json_405() {
        let app = router(AppState::new(None));

        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/healthz")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
    }

    #[tokio::test]
    async fn api_docs_are_served_at_docs() {
        let app = router(AppState::new(None));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/docs")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::OK);
        assert!(
            response
                .headers()
                .get("content-type")
                .and_then(|value| value.to_str().ok())
                .is_some_and(|value| value.starts_with("text/html"))
        );
    }
}
