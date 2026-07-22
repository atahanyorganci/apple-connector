use axum::{Json, extract::State, http::StatusCode};
use sqlx::SqlitePool;

use crate::{
    api::{dto::common::HealthStatusDto, params::PageParams, router::AppState},
    db::is_pool_healthy,
};

pub(crate) fn require_db(
    pool: &Option<SqlitePool>,
) -> Result<&SqlitePool, crate::api::error::ApiError> {
    pool.as_ref().ok_or_else(|| {
        crate::api::error::ApiError::service_unavailable("Messages database is unavailable")
    })
}

pub(crate) fn validate_page(page: &PageParams) -> Result<u32, crate::api::error::ApiError> {
    page.validated_limit()?;
    page.validated_cursor()?;
    page.validated_limit()
}

/// Health check
///
/// Reports whether the read-only Messages database pool is healthy.
#[utoipa::path(
    get,
    path = "/healthz",
    operation_id = "getHealth",
    tag = "health",
    responses(
        (status = 200, description = "Database pool is healthy", body = HealthStatusDto),
        (status = 503, description = "Database pool is unavailable", body = HealthStatusDto),
    )
)]
pub async fn healthz(
    State(state): State<AppState>,
) -> Result<(StatusCode, Json<HealthStatusDto>), (StatusCode, Json<HealthStatusDto>)> {
    let healthy = match &state.db {
        Some(pool) => is_pool_healthy(pool).await,
        None => false,
    };

    if healthy {
        Ok((
            StatusCode::OK,
            Json(HealthStatusDto {
                status: "ok".to_owned(),
            }),
        ))
    } else {
        Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(HealthStatusDto {
                status: "unavailable".to_owned(),
            }),
        ))
    }
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    use crate::{
        api::router::{AppState, router},
        db::connect_pool,
        fixtures::FixtureDb,
    };

    #[tokio::test]
    async fn healthz_reports_ok_for_healthy_database() {
        let fixture = FixtureDb::empty().await.expect("fixture database");
        let pool = connect_pool(fixture.path())
            .await
            .expect("connect read-only pool");
        let app = router(AppState::new(Some(pool)));

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
            serde_json::json!({ "status": "ok" })
        );
    }

    #[tokio::test]
    async fn healthz_reports_unavailable_without_leaking_paths() {
        let app = router(AppState::new(None));

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
            serde_json::json!({ "status": "unavailable" })
        );
        assert!(!payload.contains("chat.db"));
        assert!(!payload.contains("Library/Messages"));
    }
}
