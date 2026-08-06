use std::{collections::HashMap, path::PathBuf, sync::Arc};

use apple_contacts::ContactsStore;
use apple_eventkit::EventKitStore;
use axum::{Router, middleware::from_fn};
use sqlx::SqlitePool;
use utoipa::{OpenApi, openapi::OpenApi as OpenApiSpec};
use utoipa_axum::{router::OpenApiRouter, routes};
use utoipa_scalar::{Scalar, Servable};

use super::{
    doc::ApiDoc,
    middleware::{method_not_allowed, not_found, request_timeout, security_headers, trace_request},
};
use crate::{contacts::ContactsSources, messages::attachment_path::canonicalize_attachment_root};

#[derive(Clone)]
pub struct AppState {
    pub messages_db: Option<SqlitePool>,
    pub reminders_db: Option<SqlitePool>,
    pub notes_db: Option<SqlitePool>,
    pub calendar_db: Option<SqlitePool>,
    pub contacts_sources: ContactsSources,
    pub attachment_root: Arc<PathBuf>,
    pub reminders_attachment_root: Arc<PathBuf>,
    pub notes_attachment_root: Arc<PathBuf>,
    pub calendar_attachment_root: Arc<PathBuf>,
    pub eventkit: Option<Arc<EventKitStore>>,
    pub contacts_store: Option<Arc<ContactsStore>>,
    pub openapi: Arc<OpenApiSpec>,
}

impl AppState {
    pub fn new(
        messages_db: Option<SqlitePool>,
        reminders_db: Option<SqlitePool>,
        notes_db: Option<SqlitePool>,
        calendar_db: Option<SqlitePool>,
    ) -> Self {
        Self::with_contacts(
            messages_db,
            reminders_db,
            notes_db,
            calendar_db,
            ContactsSources::new(HashMap::new()),
            None,
        )
    }

    pub fn with_contacts(
        messages_db: Option<SqlitePool>,
        reminders_db: Option<SqlitePool>,
        notes_db: Option<SqlitePool>,
        calendar_db: Option<SqlitePool>,
        contacts_sources: ContactsSources,
        contacts_store: Option<Arc<ContactsStore>>,
    ) -> Self {
        Self::with_attachment_roots(
            messages_db,
            reminders_db,
            notes_db,
            calendar_db,
            contacts_sources,
            PathBuf::from("/var/empty/apple-connector-attachments-unconfigured"),
            PathBuf::from("/var/empty/apple-connector-reminders-attachments-unconfigured"),
            PathBuf::from("/var/empty/apple-connector-notes-attachments-unconfigured"),
            PathBuf::from("/var/empty/apple-connector-calendar-attachments-unconfigured"),
            None,
            contacts_store,
        )
    }

    pub fn with_eventkit(
        messages_db: Option<SqlitePool>,
        reminders_db: Option<SqlitePool>,
        notes_db: Option<SqlitePool>,
        calendar_db: Option<SqlitePool>,
        eventkit: Option<Arc<EventKitStore>>,
    ) -> Self {
        Self::with_attachment_roots(
            messages_db,
            reminders_db,
            notes_db,
            calendar_db,
            ContactsSources::new(HashMap::new()),
            PathBuf::from("/var/empty/apple-connector-attachments-unconfigured"),
            PathBuf::from("/var/empty/apple-connector-reminders-attachments-unconfigured"),
            PathBuf::from("/var/empty/apple-connector-notes-attachments-unconfigured"),
            PathBuf::from("/var/empty/apple-connector-calendar-attachments-unconfigured"),
            eventkit,
            None,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn with_attachment_roots(
        messages_db: Option<SqlitePool>,
        reminders_db: Option<SqlitePool>,
        notes_db: Option<SqlitePool>,
        calendar_db: Option<SqlitePool>,
        contacts_sources: ContactsSources,
        attachment_root: PathBuf,
        reminders_attachment_root: PathBuf,
        notes_attachment_root: PathBuf,
        calendar_attachment_root: PathBuf,
        eventkit: Option<Arc<EventKitStore>>,
        contacts_store: Option<Arc<ContactsStore>>,
    ) -> Self {
        let attachment_root =
            canonicalize_attachment_root(&attachment_root).unwrap_or(attachment_root);
        let reminders_attachment_root = canonicalize_attachment_root(&reminders_attachment_root)
            .unwrap_or(reminders_attachment_root);
        let notes_attachment_root =
            canonicalize_attachment_root(&notes_attachment_root).unwrap_or(notes_attachment_root);
        let calendar_attachment_root = canonicalize_attachment_root(&calendar_attachment_root)
            .unwrap_or(calendar_attachment_root);
        let openapi = Arc::new(build_openapi_spec());
        Self {
            messages_db,
            reminders_db,
            notes_db,
            calendar_db,
            contacts_sources,
            attachment_root: Arc::new(attachment_root),
            reminders_attachment_root: Arc::new(reminders_attachment_root),
            notes_attachment_root: Arc::new(notes_attachment_root),
            calendar_attachment_root: Arc::new(calendar_attachment_root),
            eventkit,
            contacts_store,
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
            crate::api::handlers::reminder_mutations::create_reminder
        ))
        .routes(routes!(
            crate::api::handlers::reminder_mutations::update_reminder
        ))
        .routes(routes!(
            crate::api::handlers::reminder_mutations::delete_reminder
        ))
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
        .routes(routes!(
            crate::api::handlers::calendars::list_calendar_accounts
        ))
        .routes(routes!(crate::api::handlers::calendars::list_calendars))
        .routes(routes!(crate::api::handlers::calendars::get_calendar))
        .routes(routes!(
            crate::api::handlers::calendars::list_calendar_events_ical
        ))
        .routes(routes!(
            crate::api::handlers::calendars::list_calendar_events_caldav
        ))
        .routes(routes!(
            crate::api::handlers::calendars::list_calendar_events
        ))
        .routes(routes!(crate::api::handlers::events::list_events_ical))
        .routes(routes!(crate::api::handlers::events::list_events_caldav))
        .routes(routes!(crate::api::handlers::events::list_events))
        .routes(routes!(
            crate::api::handlers::event_attachments::get_event_attachment_content
        ))
        .routes(routes!(crate::api::handlers::events::get_event_ical))
        .routes(routes!(crate::api::handlers::events::get_event_caldav))
        .routes(routes!(crate::api::handlers::events::get_event))
        .routes(routes!(crate::api::handlers::event_mutations::create_event))
        .routes(routes!(crate::api::handlers::event_mutations::update_event))
        .routes(routes!(crate::api::handlers::event_mutations::delete_event))
        .routes(routes!(crate::api::handlers::containers::list_containers))
        .routes(routes!(crate::api::handlers::containers::get_container))
        .routes(routes!(crate::api::handlers::groups::list_groups))
        .routes(routes!(crate::api::handlers::groups::get_group))
        .routes(routes!(
            crate::api::handlers::groups::list_group_contacts_vcard
        ))
        .routes(routes!(
            crate::api::handlers::groups::list_group_contacts_carddav
        ))
        .routes(routes!(crate::api::handlers::groups::list_group_contacts))
        .routes(routes!(crate::api::handlers::contacts::list_contacts_vcard))
        .routes(routes!(
            crate::api::handlers::contacts::list_contacts_carddav
        ))
        .routes(routes!(crate::api::handlers::contacts::search_contacts))
        .routes(routes!(crate::api::handlers::contacts::list_contacts))
        .routes(routes!(crate::api::handlers::contacts::get_contact_vcard))
        .routes(routes!(crate::api::handlers::contacts::get_contact_carddav))
        .routes(routes!(crate::api::handlers::contacts::get_contact_photo))
        .routes(routes!(crate::api::handlers::contacts::get_contact))
        .routes(routes!(
            crate::api::handlers::contact_mutations::create_contact
        ))
        .routes(routes!(
            crate::api::handlers::contact_mutations::update_contact
        ))
        .routes(routes!(
            crate::api::handlers::contact_mutations::delete_contact
        ))
        .routes(routes!(
            crate::api::handlers::contact_mutations::create_group
        ))
        .routes(routes!(
            crate::api::handlers::contact_mutations::update_group
        ))
        .routes(routes!(
            crate::api::handlers::contact_mutations::delete_group
        ))
        .routes(routes!(
            crate::api::handlers::contact_mutations::add_contact_to_group
        ))
        .routes(routes!(
            crate::api::handlers::contact_mutations::remove_contact_from_group
        ))
        .routes(routes!(crate::api::handlers::openapi::get_openapi_spec))
}

#[cfg(test)]
pub(crate) mod openapi_contract {
    pub mod contract {
        use http::Method;
        use utoipa::openapi::path::Operation;

        use super::super::build_openapi_spec;

        pub fn build_spec() -> utoipa::openapi::OpenApi {
            build_openapi_spec()
        }

        pub fn operations(spec: &utoipa::openapi::OpenApi) -> Vec<(String, String, String)> {
            let mut ops = Vec::new();
            for (path, item) in &spec.paths.paths {
                push_operation(&mut ops, "get", path, item.get.as_ref());
                push_operation(&mut ops, "head", path, item.head.as_ref());
                push_operation(&mut ops, "post", path, item.post.as_ref());
                push_operation(&mut ops, "patch", path, item.patch.as_ref());
                push_operation(&mut ops, "delete", path, item.delete.as_ref());
            }
            ops.sort_by(|left, right| {
                left.1
                    .cmp(&right.1)
                    .then_with(|| left.0.cmp(&right.0))
                    .then_with(|| left.2.cmp(&right.2))
            });
            ops
        }

        pub fn route_requests(spec: &utoipa::openapi::OpenApi) -> Vec<(Method, String)> {
            operations(spec)
                .into_iter()
                .map(|(method, path, _)| {
                    (
                        method.parse::<Method>().unwrap_or_else(|error| {
                            panic!("invalid HTTP method `{method}` in OpenAPI spec: {error}")
                        }),
                        concrete_path(&path),
                    )
                })
                .collect()
        }

        pub fn operation<'a>(
            spec: &'a utoipa::openapi::OpenApi,
            method: &str,
            path: &str,
        ) -> &'a Operation {
            let item = spec
                .paths
                .paths
                .get(path)
                .unwrap_or_else(|| panic!("missing path `{path}`"));
            match method {
                "get" => item.get.as_ref(),
                "head" => item.head.as_ref(),
                "post" => item.post.as_ref(),
                "patch" => item.patch.as_ref(),
                "delete" => item.delete.as_ref(),
                _ => None,
            }
            .unwrap_or_else(|| panic!("missing `{method} {path}`"))
        }

        fn push_operation(
            ops: &mut Vec<(String, String, String)>,
            method: &str,
            path: &str,
            operation: Option<&Operation>,
        ) {
            let Some(operation) = operation else {
                return;
            };
            ops.push((
                method.to_owned(),
                path.to_owned(),
                operation
                    .operation_id
                    .clone()
                    .unwrap_or_else(|| panic!("missing operationId for `{method} {path}`")),
            ));
        }

        fn concrete_path(path: &str) -> String {
            path.replace("{chat_id}", "1")
                .replace("{folder_id}", "1")
                .replace("{list_id}", "1")
                .replace("{guid}", "test-guid")
                .replace("{reminder_id}", "test-id")
                .replace("{note_id}", "test-id")
                .replace("{calendar_id}", "test-id")
                .replace("{event_id}", "test-id")
                .replace("{container_id}", "test-id")
                .replace("{group_id}", "test-id")
                .replace("{contact_id}", "test-id")
                .replace("{attachment_id}", "test-id")
                .replace("{id}", "test-id")
        }
    }
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use http::{Request, StatusCode};
    use tower::ServiceExt;

    use super::{AppState, openapi_contract::contract, router};

    #[tokio::test]
    async fn router_registers_every_contract_route() -> Result<(), Box<dyn std::error::Error>> {
        let app = router(AppState::new(None, None, None, None));
        let spec = contract::build_spec();

        for (method, path) in contract::route_requests(&spec) {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method(method.clone())
                        .uri(path.clone())
                        .body(Body::empty())?,
                )
                .await?;

            assert_ne!(
                response.status(),
                StatusCode::NOT_FOUND,
                "missing route {method} {path}"
            );
        }
        Ok(())
    }

    #[tokio::test]
    async fn unknown_route_returns_json_404_with_security_headers()
    -> Result<(), Box<dyn std::error::Error>> {
        let app = router(AppState::new(None, None, None, None));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/secret/path?token=abc")
                    .body(Body::empty())?,
            )
            .await?;

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(
            response
                .headers()
                .get("x-content-type-options")
                .ok_or("missing nosniff header")?,
            "nosniff"
        );
        assert!(
            response
                .headers()
                .get("access-control-allow-origin")
                .is_none()
        );
        Ok(())
    }

    #[tokio::test]
    async fn unsupported_method_returns_json_405() -> Result<(), Box<dyn std::error::Error>> {
        let app = router(AppState::new(None, None, None, None));

        let response = app
            .oneshot(
                Request::builder()
                    .method(http::Method::POST)
                    .uri("/healthz")
                    .body(Body::empty())?,
            )
            .await?;

        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
        Ok(())
    }

    #[tokio::test]
    async fn api_docs_are_served_at_docs() -> Result<(), Box<dyn std::error::Error>> {
        let app = router(AppState::new(None, None, None, None));

        let response = app
            .oneshot(Request::builder().uri("/docs").body(Body::empty())?)
            .await?;

        assert_eq!(response.status(), StatusCode::OK);
        assert!(
            response
                .headers()
                .get("content-type")
                .and_then(|value| value.to_str().ok())
                .is_some_and(|value| value.starts_with("text/html"))
        );
        Ok(())
    }
}
