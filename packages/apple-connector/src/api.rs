use axum::{Json, Router, extract::State, http::StatusCode, routing::get};
use serde::Serialize;
use sqlx::SqlitePool;

use crate::db::is_pool_healthy;

#[derive(Clone)]
pub struct AppState {
    pub db: Option<SqlitePool>,
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .with_state(state)
}

async fn healthz(State(state): State<AppState>) -> (StatusCode, Json<HealthResponse>) {
    let healthy = match &state.db {
        Some(pool) => is_pool_healthy(pool).await,
        None => false,
    };

    if healthy {
        (StatusCode::OK, Json(HealthResponse { status: "ok" }))
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(HealthResponse {
                status: "unavailable",
            }),
        )
    }
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    use super::{AppState, router};
    use crate::{db::connect_pool, fixtures::FixtureDb};

    #[tokio::test]
    async fn healthz_reports_ok_for_healthy_database() {
        let fixture = FixtureDb::empty().await.expect("fixture database");
        let pool = connect_pool(fixture.path())
            .await
            .expect("connect read-only pool");
        let app = router(AppState { db: Some(pool) });

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
        let app = router(AppState { db: None });

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

    #[tokio::test]
    async fn healthz_reports_unavailable_for_missing_database_path() {
        let app = router(AppState { db: None });

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
    }

    #[test]
    fn health_response_never_contains_database_path() {
        let serialized = serde_json::to_string(&super::HealthResponse {
            status: "unavailable",
        })
        .expect("serialize");
        assert!(!serialized.contains("/Users/test/Library/Messages/chat.db"));
    }
}
