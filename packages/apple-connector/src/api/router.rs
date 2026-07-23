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
    pub messages_db: Option<SqlitePool>,
    pub reminders_db: Option<SqlitePool>,
    pub notes_db: Option<SqlitePool>,
    pub attachment_root: Arc<PathBuf>,
    pub reminders_attachment_root: Arc<PathBuf>,
    pub notes_attachment_root: Arc<PathBuf>,
    pub openapi: Arc<OpenApiSpec>,
}

impl AppState {
    pub fn new(
        messages_db: Option<SqlitePool>,
        reminders_db: Option<SqlitePool>,
        notes_db: Option<SqlitePool>,
    ) -> Self {
        Self::with_attachment_roots(
            messages_db,
            reminders_db,
            notes_db,
            PathBuf::from("/var/empty/apple-connector-attachments-unconfigured"),
            PathBuf::from("/var/empty/apple-connector-reminders-attachments-unconfigured"),
            PathBuf::from("/var/empty/apple-connector-notes-attachments-unconfigured"),
        )
    }

    pub fn with_attachment_roots(
        messages_db: Option<SqlitePool>,
        reminders_db: Option<SqlitePool>,
        notes_db: Option<SqlitePool>,
        attachment_root: PathBuf,
        reminders_attachment_root: PathBuf,
        notes_attachment_root: PathBuf,
    ) -> Self {
        let attachment_root =
            canonicalize_attachment_root(&attachment_root).unwrap_or(attachment_root);
        let reminders_attachment_root = canonicalize_attachment_root(&reminders_attachment_root)
            .unwrap_or(reminders_attachment_root);
        let notes_attachment_root =
            canonicalize_attachment_root(&notes_attachment_root).unwrap_or(notes_attachment_root);
        let openapi = Arc::new(build_openapi_spec());
        Self {
            messages_db,
            reminders_db,
            notes_db,
            attachment_root: Arc::new(attachment_root),
            reminders_attachment_root: Arc::new(reminders_attachment_root),
            notes_attachment_root: Arc::new(notes_attachment_root),
            openapi,
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
        .routes(routes!(
            crate::api::handlers::reminder_lists::list_reminder_lists
        ))
        .routes(routes!(
            crate::api::handlers::reminder_lists::get_reminder_list
        ))
        .routes(routes!(
            crate::api::handlers::reminder_lists::list_reminder_list_reminders
        ))
        .routes(routes!(crate::api::handlers::reminders::list_reminders))
        .routes(routes!(crate::api::handlers::reminders::get_reminder))
        .routes(routes!(
            crate::api::handlers::reminder_attachments::get_reminder_attachment
        ))
        .routes(routes!(
            crate::api::handlers::reminder_attachments::get_reminder_attachment_content,
            crate::api::handlers::reminder_attachments::head_reminder_attachment_content,
        ))
        .routes(routes!(
            crate::api::handlers::note_folders::list_note_folders
        ))
        .routes(routes!(crate::api::handlers::note_folders::get_note_folder))
        .routes(routes!(
            crate::api::handlers::note_folders::list_folder_notes
        ))
        .routes(routes!(crate::api::handlers::notes::list_notes))
        .routes(routes!(crate::api::handlers::notes::get_note_contents))
        .routes(routes!(crate::api::handlers::notes::get_note))
        .routes(routes!(
            crate::api::handlers::note_attachments::get_note_attachment
        ))
        .routes(routes!(
            crate::api::handlers::note_attachments::get_note_attachment_content,
            crate::api::handlers::note_attachments::head_note_attachment_content,
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
        ("GET", "/v1/reminder-lists"),
        ("GET", "/v1/reminder-lists/1"),
        ("GET", "/v1/reminder-lists/1/reminders"),
        ("GET", "/v1/reminders"),
        ("GET", "/v1/reminders/test-id"),
        ("GET", "/v1/reminder-attachments/test-id"),
        ("GET", "/v1/reminder-attachments/test-id/content"),
        ("HEAD", "/v1/reminder-attachments/test-id/content"),
        ("GET", "/v1/note-folders"),
        ("GET", "/v1/note-folders/1"),
        ("GET", "/v1/note-folders/1/notes"),
        ("GET", "/v1/notes"),
        ("GET", "/v1/notes/test-id"),
        ("GET", "/v1/notes/test-id/contents"),
        ("GET", "/v1/note-attachments/test-id"),
        ("GET", "/v1/note-attachments/test-id/content"),
        ("HEAD", "/v1/note-attachments/test-id/content"),
        ("GET", "/openapi.json"),
    ];

    #[tokio::test]
    async fn router_registers_every_contract_route() {
        let app = router(AppState::new(None, None, None));

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
        let app = router(AppState::new(None, None, None));

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
        let app = router(AppState::new(None, None, None));

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
        let app = router(AppState::new(None, None, None));

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
