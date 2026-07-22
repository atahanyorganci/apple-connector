use axum::{
    Json,
    extract::{Query, State},
};

use super::{
    chats::message_page_to_dto,
    health::{require_db, validate_page},
};
use crate::{
    api::{
        cursor::{GlobalMessageCursor, decode},
        dto::{MessageDetailDto, MessagePageDto, convert::message_detail_to_dto},
        error::{ApiError, ErrorResponse},
        params::{MessageGuidPath, PageParams},
        router::AppState,
    },
    messages::repository::MessageRepository,
};

/// List messages
///
/// Returns messages across all chats ordered newest first with keyset pagination.
#[utoipa::path(
    get,
    path = "/v1/messages",
    operation_id = "listMessages",
    tag = "messages",
    params(PageParams),
    responses(
        (status = 200, description = "Paginated message summaries", body = MessagePageDto),
        (status = 400, description = "Invalid pagination parameters", body = ErrorResponse),
        (status = 503, description = "Messages database is unavailable", body = ErrorResponse),
        (status = 500, description = "Unexpected server error", body = ErrorResponse),
    )
)]
pub async fn list_messages(
    State(state): State<AppState>,
    Query(page): Query<PageParams>,
) -> Result<Json<MessagePageDto>, ApiError> {
    let pool = require_db(&state.db)?;
    let limit = validate_page(&page)?;
    let cursor = page
        .validated_cursor()?
        .map(decode::<GlobalMessageCursor>)
        .transpose()?;

    let page = MessageRepository::new(pool)
        .list_messages(limit, cursor)
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;

    Ok(Json(message_page_to_dto(page, limit)))
}

/// Get message
///
/// Returns a single message with full envelope metadata and tagged content.
#[utoipa::path(
    get,
    path = "/v1/messages/{guid}",
    operation_id = "getMessage",
    tag = "messages",
    params(MessageGuidPath),
    responses(
        (status = 200, description = "Message details", body = MessageDetailDto),
        (status = 404, description = "Message not found", body = ErrorResponse),
        (status = 503, description = "Messages database is unavailable", body = ErrorResponse),
        (status = 500, description = "Unexpected server error", body = ErrorResponse),
    )
)]
pub async fn get_message(
    State(state): State<AppState>,
    axum::extract::Path(MessageGuidPath { guid }): axum::extract::Path<MessageGuidPath>,
) -> Result<Json<MessageDetailDto>, ApiError> {
    let pool = require_db(&state.db)?;
    let message = MessageRepository::new(pool)
        .get_message_by_guid(&guid)
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?
        .ok_or_else(|| ApiError::not_found(format!("message {guid} not found")))?;

    Ok(Json(message_detail_to_dto(&message)))
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
    async fn get_message_returns_seeded_message() {
        let fixture = FixtureDb::seeded().await.expect("seeded fixture");
        let pool = connect_pool(fixture.path()).await.expect("connect pool");
        let app = router(AppState::new(Some(pool)));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/v1/messages/fixture-message-guid")
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
        let payload: serde_json::Value = serde_json::from_slice(&body).expect("json");
        assert_eq!(
            payload.get("guid").and_then(|value| value.as_str()),
            Some("fixture-message-guid")
        );
    }
}
