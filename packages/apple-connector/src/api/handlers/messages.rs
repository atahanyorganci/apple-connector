use axum::{
    Json,
    extract::{Query, State},
};

use super::{
    chats::empty_message_page,
    health::{require_db, validate_page},
};
use crate::api::{
    dto::{MessageDetailDto, MessagePageDto},
    error::{ApiError, ErrorResponse},
    params::{MessageGuidPath, PageParams},
    router::AppState,
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
    require_db(&state.db)?;
    let limit = validate_page(&page)?;
    Ok(empty_message_page(limit))
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
    require_db(&state.db)?;
    Err(ApiError::not_found(format!("message {guid} not found")))
}
