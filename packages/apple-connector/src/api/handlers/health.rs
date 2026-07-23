use axum::{Json, extract::State, http::StatusCode};
use sqlx::SqlitePool;

use crate::{
    api::{
        dto::common::{HealthStatus, HealthStatusDto},
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

/// Health check
///
/// Reports whether the read-only Messages and Reminders database pools are healthy.
#[utoipa::path(
    get,
    path = "/healthz",
    operation_id = "getHealth",
    tag = "health",
    responses(
        (status = 200, description = "Both database pools are healthy", body = HealthStatusDto),
        (status = 503, description = "One or both database pools are unavailable", body = HealthStatusDto),
    )
)]
pub async fn healthz(
    State(state): State<AppState>,
) -> Result<(StatusCode, Json<HealthStatusDto>), (StatusCode, Json<HealthStatusDto>)> {
    let messages = messages_status(&state.messages_db).await;
    let reminders = reminders_status(&state.reminders_db).await;
    let body = HealthStatusDto {
        messages,
        reminders,
    };
    let all_ok = messages == HealthStatus::Ok && reminders == HealthStatus::Ok;

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
            dto::common::HealthStatus,
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
        let app = router(AppState::new(Some(messages_pool), Some(reminders_pool)));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/healthz")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::OK);

        let body = response
            .into_body()
            .collect()
            .await
            .expect("body")
            .to_bytes();
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&body).expect("json"),
            serde_json::json!({ "messages": "ok", "reminders": "ok" })
        );
    }

    #[tokio::test]
    async fn healthz_reports_unavailable_without_leaking_paths() {
        let app = router(AppState::new(None, None));

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
            serde_json::json!({ "messages": "unavailable", "reminders": "unavailable" })
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
    }
}
