use std::sync::Arc;

use axum::Router;
use sqlx::SqlitePool;
use utoipa::{OpenApi, openapi::OpenApi as OpenApiSpec};
use utoipa_axum::{router::OpenApiRouter, routes};

use super::doc::ApiDoc;

#[derive(Clone)]
pub struct AppState {
    pub db: Option<SqlitePool>,
    pub openapi: Arc<OpenApiSpec>,
}

impl AppState {
    pub fn new(db: Option<SqlitePool>) -> Self {
        let openapi = Arc::new(build_openapi_spec());
        Self { db, openapi }
    }
}

pub fn build_openapi_spec() -> OpenApiSpec {
    openapi_router().get_openapi().clone()
}

pub fn router(state: AppState) -> Router {
    openapi_router().with_state(state).into()
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
}
