use axum::{Json, extract::State, http::StatusCode};
use sqlx::SqlitePool;

use crate::{
    api::{
        contacts::{contacts_auth_status, contacts_status},
        dto::common::{HealthStatus, HealthStatusDto},
        eventkit::{eventkit_events_status, eventkit_reminders_status},
        params::PageParams,
        router::AppState,
    },
    db::is_pool_healthy,
};

pub(crate) fn require_messages_db(
    pool: &Option<SqlitePool>,
) -> Result<&SqlitePool, crate::api::error::ApiError> {
    pool.as_ref().ok_or_else(|| {
        crate::api::error::ApiError::service_unavailable("Messages database is unavailable")
    })
}

#[allow(dead_code)]
pub(crate) fn require_reminders_db(
    pool: &Option<SqlitePool>,
) -> Result<&SqlitePool, crate::api::error::ApiError> {
    pool.as_ref().ok_or_else(|| {
        crate::api::error::ApiError::service_unavailable("Reminders database is unavailable")
    })
}

pub(crate) fn require_calendar_db(
    pool: &Option<SqlitePool>,
) -> Result<&SqlitePool, crate::api::error::ApiError> {
    pool.as_ref().ok_or_else(|| {
        crate::api::error::ApiError::service_unavailable("Calendar database is unavailable")
    })
}

pub(crate) fn require_notes_db(
    pool: &Option<SqlitePool>,
) -> Result<&SqlitePool, crate::api::error::ApiError> {
    pool.as_ref().ok_or_else(|| {
        crate::api::error::ApiError::service_unavailable("Notes database is unavailable")
    })
}

pub(crate) fn validate_page(page: &PageParams) -> Result<u32, crate::api::error::ApiError> {
    page.validated_limit()?;
    page.validated_cursor()?;
    page.validated_limit()
}

async fn messages_status(pool: &Option<SqlitePool>) -> HealthStatus {
    match pool {
        Some(pool) if is_pool_healthy(pool).await => HealthStatus::Ok,
        _ => HealthStatus::Unavailable,
    }
}

async fn reminders_status(pool: &Option<SqlitePool>) -> HealthStatus {
    match pool {
        Some(pool) if is_pool_healthy(pool).await => HealthStatus::Ok,
        _ => HealthStatus::Unavailable,
    }
}

async fn notes_status(pool: &Option<SqlitePool>) -> HealthStatus {
    match pool {
        Some(pool) if is_pool_healthy(pool).await => HealthStatus::Ok,
        _ => HealthStatus::Unavailable,
    }
}

async fn calendar_status(pool: &Option<SqlitePool>) -> HealthStatus {
    match pool {
        Some(pool) if is_pool_healthy(pool).await => HealthStatus::Ok,
        _ => HealthStatus::Unavailable,
    }
}

/// Health check
///
/// Reports whether the read-only Messages, Reminders, Notes, Calendar, and Contacts database pools are healthy,
/// plus EventKit and Contacts authorization status for write operations.
#[utoipa::path(
    get,
    path = "/healthz",
    operation_id = "getHealth",
    tag = "health",
    responses(
        (status = 200, description = "All database pools are healthy", body = HealthStatusDto),
        (status = 503, description = "One or more database pools are unavailable", body = HealthStatusDto),
    )
)]
pub async fn healthz(
    State(state): State<AppState>,
) -> Result<(StatusCode, Json<HealthStatusDto>), (StatusCode, Json<HealthStatusDto>)> {
    let messages = messages_status(&state.messages_db).await;
    let reminders = reminders_status(&state.reminders_db).await;
    let notes = notes_status(&state.notes_db).await;
    let calendar = calendar_status(&state.calendar_db).await;
    let contacts = contacts_status(&state.contacts_sources).await;
    let eventkit_reminders = eventkit_reminders_status(&state.eventkit).await;
    let eventkit_events = eventkit_events_status(&state.eventkit).await;
    let contacts_auth = contacts_auth_status(&state.contacts_store).await;
    let body = HealthStatusDto {
        messages,
        reminders,
        notes,
        calendar,
        contacts,
        eventkit_reminders,
        eventkit_events,
        contacts_auth,
    };
    let all_ok = messages == HealthStatus::Ok
        && reminders == HealthStatus::Ok
        && notes == HealthStatus::Ok
        && calendar == HealthStatus::Ok
        && contacts == HealthStatus::Ok;

    if all_ok {
        Ok((StatusCode::OK, Json(body)))
    } else {
        Err((StatusCode::SERVICE_UNAVAILABLE, Json(body)))
    }
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    use crate::{
        api::{
            dto::common::{EventKitAuthStatusDto, HealthStatus},
            router::{AppState, router},
        },
        db::connect_pool,
        fixtures::{FixtureDb, RemindersFixtureDb},
    };

    #[tokio::test]
    async fn healthz_reports_ok_when_both_databases_are_healthy() {
        let messages_fixture = FixtureDb::empty().await.expect("messages fixture");
        let reminders_fixture = RemindersFixtureDb::empty()
            .await
            .expect("reminders fixture");
        let messages_pool = connect_pool(messages_fixture.path())
            .await
            .expect("messages pool");
        let reminders_pool = connect_pool(reminders_fixture.path())
            .await
            .expect("reminders pool");
        let app = router(AppState::new(
            Some(messages_pool),
            Some(reminders_pool),
            None,
            None,
        ));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/healthz")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);

        let body = response
            .into_body()
            .collect()
            .await
            .expect("body")
            .to_bytes();
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&body).expect("json"),
            serde_json::json!({
                "messages": "ok",
                "reminders": "ok",
                "notes": "unavailable",
                "calendar": "unavailable",
                "contacts": "unavailable",
                "eventkit_reminders": "unavailable",
                "eventkit_events": "unavailable",
                "contacts_auth": "unavailable",
            })
        );
    }

    #[tokio::test]
    async fn healthz_reports_unavailable_without_leaking_paths() {
        let app = router(AppState::new(None, None, None, None));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/healthz")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);

        let body = response
            .into_body()
            .collect()
            .await
            .expect("body")
            .to_bytes();
        let payload = String::from_utf8(body.to_vec()).expect("utf-8");
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&payload).expect("json"),
            serde_json::json!({
                "messages": "unavailable",
                "reminders": "unavailable",
                "notes": "unavailable",
                "calendar": "unavailable",
                "contacts": "unavailable",
                "eventkit_reminders": "unavailable",
                "eventkit_events": "unavailable",
                "contacts_auth": "unavailable",
            })
        );
        assert!(!payload.contains("chat.db"));
        assert!(!payload.contains("Library/Messages"));
        assert!(!payload.contains("Group Containers"));
    }

    #[test]
    fn health_status_serializes_as_snake_case() {
        assert_eq!(
            serde_json::to_string(&HealthStatus::Ok).expect("serialize"),
            "\"ok\""
        );
        assert_eq!(
            serde_json::to_string(&EventKitAuthStatusDto::Authorized).expect("serialize"),
            "\"authorized\""
        );
    }
}
