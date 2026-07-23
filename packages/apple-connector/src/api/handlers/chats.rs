use axum::{
    Json,
    extract::{Query, State},
};

use super::health::{require_messages_db, validate_page};
use crate::{
    api::{
        cursor::{ChatListCursor, ChatMessageCursor, decode},
        dto::{
            ChatDetailDto, ChatPageDto, MessagePageDto, PageMetaDto,
            convert::{chat_detail_to_dto, chat_summary_to_dto, message_summary_to_dto},
        },
        error::{ApiError, ErrorResponse},
        params::{ChatIdPath, PageParams},
        router::AppState,
    },
    messages::repository::{ChatLookupError, MessageRepository},
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
    let pool = require_messages_db(&state.messages_db)?;
    let limit = validate_page(&page)?;
    let cursor = page
        .validated_cursor()?
        .map(decode::<ChatListCursor>)
        .transpose()?;

    let page = MessageRepository::new(pool)
        .list_chats(limit, cursor)
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;

    Ok(Json(ChatPageDto {
        items: page.items.iter().map(chat_summary_to_dto).collect(),
        page: PageMetaDto {
            limit,
            has_more: page.has_more,
            next_cursor: page.next_cursor,
        },
    }))
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
    let pool = require_messages_db(&state.messages_db)?;
    let chat = MessageRepository::new(pool)
        .get_chat(chat_id)
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?
        .ok_or_else(|| ApiError::not_found(format!("chat {chat_id} not found")))?;

    Ok(Json(chat_detail_to_dto(&chat)))
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
    let pool = require_messages_db(&state.messages_db)?;
    let limit = validate_page(&page)?;
    let cursor = page
        .validated_cursor()?
        .map(decode::<ChatMessageCursor>)
        .transpose()?;

    let page = MessageRepository::new(pool)
        .list_chat_messages(chat_id, limit, cursor)
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;

    match page {
        Err(ChatLookupError::NotFound) => {
            Err(ApiError::not_found(format!("chat {chat_id} not found")))
        }
        Ok(page) => Ok(Json(message_page_to_dto(page, limit))),
    }
}

pub(crate) fn message_page_to_dto(
    page: crate::messages::repository::Page<crate::messages::Message>,
    limit: u32,
) -> MessagePageDto {
    MessagePageDto {
        items: page.items.iter().map(message_summary_to_dto).collect(),
        page: PageMetaDto {
            limit,
            has_more: page.has_more,
            next_cursor: page.next_cursor,
        },
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
    async fn list_chats_returns_seeded_chat() {
        let fixture = FixtureDb::seeded().await.expect("seeded fixture");
        let pool = connect_pool(fixture.path()).await.expect("connect pool");
        let app = router(AppState::new(Some(pool), None));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/v1/chats")
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
            payload["items"].as_array().map(|items| items.len()),
            Some(1)
        );
    }
}
