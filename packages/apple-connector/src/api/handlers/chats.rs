use axum::{
    Json,
    extract::{Query, State},
};

use super::health::{empty_chat_page, require_db, validate_page};
use crate::api::{
    dto::{ChatDetailDto, ChatPageDto, MessagePageDto, PageMetaDto},
    error::{ApiError, ErrorResponse},
    params::{ChatIdPath, PageParams},
    router::AppState,
};

/// List chats
///
/// Returns chats ordered by recent activity. Uses keyset pagination (newest first).
#[utoipa::path(
    get,
    path = "/v1/chats",
    operation_id = "listChats",
    tag = "chats",
    params(PageParams),
    responses(
        (status = 200, description = "Paginated chat summaries", body = ChatPageDto),
        (status = 400, description = "Invalid pagination parameters", body = ErrorResponse),
        (status = 503, description = "Messages database is unavailable", body = ErrorResponse),
        (status = 500, description = "Unexpected server error", body = ErrorResponse),
    )
)]
pub async fn list_chats(
    State(state): State<AppState>,
    Query(page): Query<PageParams>,
) -> Result<Json<ChatPageDto>, ApiError> {
    require_db(&state.db)?;
    let limit = validate_page(&page)?;
    let _ = limit;
    Ok(empty_chat_page(limit))
}

/// Get chat
///
/// Returns metadata and participants for a single chat.
#[utoipa::path(
    get,
    path = "/v1/chats/{chat_id}",
    operation_id = "getChat",
    tag = "chats",
    params(ChatIdPath),
    responses(
        (status = 200, description = "Chat details", body = ChatDetailDto),
        (status = 404, description = "Chat not found", body = ErrorResponse),
        (status = 503, description = "Messages database is unavailable", body = ErrorResponse),
        (status = 500, description = "Unexpected server error", body = ErrorResponse),
    )
)]
pub async fn get_chat(
    State(state): State<AppState>,
    axum::extract::Path(ChatIdPath { chat_id }): axum::extract::Path<ChatIdPath>,
) -> Result<Json<ChatDetailDto>, ApiError> {
    require_db(&state.db)?;
    Err(ApiError::not_found(format!("chat {chat_id} not found")))
}

/// List chat messages
///
/// Returns messages for a chat ordered newest first with keyset pagination.
#[utoipa::path(
    get,
    path = "/v1/chats/{chat_id}/messages",
    operation_id = "listChatMessages",
    tag = "chats",
    params(
        ChatIdPath,
        PageParams,
    ),
    responses(
        (status = 200, description = "Paginated messages for the chat", body = MessagePageDto),
        (status = 400, description = "Invalid pagination parameters", body = ErrorResponse),
        (status = 404, description = "Chat not found", body = ErrorResponse),
        (status = 503, description = "Messages database is unavailable", body = ErrorResponse),
        (status = 500, description = "Unexpected server error", body = ErrorResponse),
    )
)]
pub async fn list_chat_messages(
    State(state): State<AppState>,
    axum::extract::Path(ChatIdPath { chat_id }): axum::extract::Path<ChatIdPath>,
    Query(page): Query<PageParams>,
) -> Result<Json<MessagePageDto>, ApiError> {
    require_db(&state.db)?;
    let limit = validate_page(&page)?;
    let _ = (chat_id, limit);
    Err(ApiError::not_found(format!("chat {chat_id} not found")))
}

pub(crate) fn empty_message_page(limit: u32) -> Json<MessagePageDto> {
    Json(MessagePageDto {
        items: Vec::new(),
        page: PageMetaDto::empty(limit),
    })
}
