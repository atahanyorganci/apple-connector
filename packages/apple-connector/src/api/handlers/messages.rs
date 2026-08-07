use axum::{
    Json,
    extract::{Query, State},
};

use super::{chats::message_page_to_dto, health::require_messages_db};
use crate::{
    api::{
        cursor::{decode_global_or_reject_for_filters, decode_search_cursor},
        dto::{MessageDetailDto, MessagePageDto, convert::message_detail_to_dto},
        error::{ApiError, ErrorResponse},
        params::{MessageGuidPath, MessageListParams},
        router::AppState,
    },
    messages::repository::{MessageListCursor, MessageRepository},
};

/// List messages
///
/// Returns messages across all chats ordered newest first with keyset pagination.
/// Supports metadata filters and bounded case-insensitive text search.
#[utoipa::path(
    get,
    path = "/v1/messages",
    operation_id = "listMessages",
    tag = "messages",
    params(MessageListParams),
    responses(
        (status = 200, description = "Paginated message summaries", body = MessagePageDto),
        (status = 400, description = "Invalid pagination or filter parameters", body = ErrorResponse),
        (status = 503, description = "Messages database is unavailable", body = ErrorResponse),
        (status = 500, description = "Unexpected server error", body = ErrorResponse),
    )
)]
pub async fn list_messages(
    State(state): State<AppState>,
    Query(params): Query<MessageListParams>,
) -> Result<Json<MessagePageDto>, ApiError> {
    let pool = require_messages_db(&state.messages_db)?;
    let limit = params.validated_limit()?;
    params.validated_cursor()?;
    let filters = params.validated_filters()?;
    let filter_snapshot = filters.snapshot();

    let (search_cursor, global_cursor) = match params.cursor.as_deref() {
        None => (None, None),
        Some(cursor) if filters.is_active() => {
            (Some(decode_search_cursor(cursor, &filter_snapshot)?), None)
        }
        Some(cursor) => (
            None,
            Some(MessageListCursor::Global(
                decode_global_or_reject_for_filters(cursor)?,
            )),
        ),
    };

    let page = MessageRepository::new(pool)
        .list_messages_filtered(&filters, limit, search_cursor, global_cursor)
        .await
        .map_err(ApiError::from_sqlx)?;

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
    axum::extract::Path(path): axum::extract::Path<MessageGuidPath>,
) -> Result<Json<MessageDetailDto>, ApiError> {
    let guid = path.validated()?;
    let pool = require_messages_db(&state.messages_db)?;
    let message = MessageRepository::new(pool)
        .get_message_by_guid(guid.as_str())
        .await
        .map_err(ApiError::from_sqlx)?
        .ok_or_else(|| ApiError::not_found(format!("message {guid} not found")))?;

    Ok(Json(message_detail_to_dto(&message)))
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use sqlx::Connection;
    use tower::ServiceExt;

    use crate::{
        api::router::{AppState, router},
        db::connect_pool,
        fixtures::FixtureDb,
    };

    const HELLO_FIXTURE: &[u8] =
        include_bytes!("../../../fixtures/messages/attributed-body-hello.bin");

    async fn seed_search_fixture() -> Result<crate::fixtures::FixtureDb, Box<dyn std::error::Error>>
    {
        let fixture = FixtureDb::empty().await?;
        let mut connection =
            sqlx::SqliteConnection::connect(fixture.path().to_str().ok_or("invalid path")?).await?;

        sqlx::query!("DROP TRIGGER IF EXISTS verify_chat_insert")
            .execute(&mut connection)
            .await?;
        sqlx::query!("DROP TRIGGER IF EXISTS verify_chat_update")
            .execute(&mut connection)
            .await?;
        sqlx::query!(
            "INSERT INTO handle (ROWID, id, service) VALUES (1, '+15550000001', 'iMessage')"
        )
        .execute(&mut connection)
        .await?;
        sqlx::query!("INSERT INTO handle (ROWID, id, service) VALUES (2, '+15550000002', 'SMS')")
            .execute(&mut connection)
            .await?;
        sqlx::query!(
            "INSERT INTO chat (ROWID, guid, style, chat_identifier, service_name) VALUES (1, 'chat-a', 45, '+15550000001', 'iMessage')"
        )
        .execute(&mut connection)
        .await?;
        sqlx::query!(
            "INSERT INTO chat (ROWID, guid, style, chat_identifier, service_name) VALUES (2, 'chat-b', 45, '+15550000002', 'SMS')"
        )
        .execute(&mut connection)
        .await?;
        sqlx::query!(
            "INSERT INTO message (ROWID, guid, text, service, is_from_me, date, handle_id, cache_has_attachments) VALUES (1, 'msg-plain', 'Hello World filter text', 'iMessage', 0, 300, 1, 0)"
        )
        .execute(&mut connection)
        .await?;
        sqlx::query!(
            "INSERT INTO message (ROWID, guid, text, service, is_from_me, date, handle_id, cache_has_attachments) VALUES (2, 'msg-attributed', NULL, 'iMessage', 0, 200, 1, 0)"
        )
        .execute(&mut connection)
        .await?;
        sqlx::query!(
            "INSERT INTO message (ROWID, guid, text, service, is_from_me, date, handle_id, cache_has_attachments) VALUES (3, 'msg-sent', 'Sent only body', 'iMessage', 1, 100, 0, 0)"
        )
        .execute(&mut connection)
        .await?;
        sqlx::query!(
            "INSERT INTO message (ROWID, guid, text, service, is_from_me, date, handle_id, cache_has_attachments, item_type) VALUES (4, 'msg-group', 'Group title', 'SMS', 0, 50, 2, 0, 2)"
        )
        .execute(&mut connection)
        .await?;
        sqlx::query!(
            "INSERT INTO message (ROWID, guid, text, service, is_from_me, date, handle_id, cache_has_attachments) VALUES (5, 'msg-noise-1', 'noise alpha', 'iMessage', 0, 40, 1, 0)"
        )
        .execute(&mut connection)
        .await?;
        sqlx::query!(
            "INSERT INTO message (ROWID, guid, text, service, is_from_me, date, handle_id, cache_has_attachments) VALUES (6, 'msg-noise-2', 'noise beta', 'iMessage', 0, 30, 1, 0)"
        )
        .execute(&mut connection)
        .await?;
        sqlx::query!(
            "INSERT INTO message (ROWID, guid, text, service, is_from_me, date, handle_id, cache_has_attachments) VALUES (7, 'msg-noise-3', 'noise gamma', 'iMessage', 0, 20, 1, 0)"
        )
        .execute(&mut connection)
        .await?;
        sqlx::query!(
            "INSERT INTO chat_message_join (chat_id, message_id, message_date) SELECT 1, message.ROWID, message.date FROM message WHERE message.ROWID IN (1,2,3,5,6,7)"
        )
        .execute(&mut connection)
        .await?;
        sqlx::query!(
            "INSERT INTO chat_message_join (chat_id, message_id, message_date) SELECT 2, message.ROWID, message.date FROM message WHERE message.ROWID IN (4)"
        )
        .execute(&mut connection)
        .await?;

        sqlx::query!(
            "UPDATE message SET attributedBody = ?1 WHERE guid = 'msg-attributed'",
            HELLO_FIXTURE
        )
        .execute(&mut connection)
        .await?;

        connection.close().await.ok();
        Ok(fixture)
    }

    async fn response_json(
        app: axum::Router,
        uri: &str,
    ) -> Result<(http::StatusCode, serde_json::Value), Box<dyn std::error::Error>> {
        let response = app
            .oneshot(Request::builder().uri(uri).body(Body::empty())?)
            .await?;
        let status = response.status();
        let body = response.into_body().collect().await?.to_bytes();
        let payload = serde_json::from_slice(&body)
            .unwrap_or(serde_json::json!({ "raw": String::from_utf8_lossy(&body) }));
        Ok((status, payload))
    }

    #[tokio::test]
    async fn get_message_returns_seeded_message() -> Result<(), Box<dyn std::error::Error>> {
        let fixture = FixtureDb::seeded().await?;
        let pool = connect_pool(fixture.path()).await?;
        let app = router(AppState::new(Some(pool), None, None, None));

        let (status, payload) = response_json(app, "/v1/messages/fixture-message-guid").await?;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            payload.get("guid").and_then(|value| value.as_str()),
            Some("fixture-message-guid")
        );
        Ok(())
    }

    #[tokio::test]
    async fn search_finds_attributed_body_only_text() -> Result<(), Box<dyn std::error::Error>> {
        let fixture = seed_search_fixture().await?;
        let pool = connect_pool(fixture.path()).await?;
        let app = router(AppState::new(Some(pool), None, None, None));

        let (status, payload) = response_json(app, "/v1/messages?q=noter").await?;
        assert_eq!(status, StatusCode::OK);
        let guids: Vec<_> = payload["items"]
            .as_array()
            .ok_or("expected array")?
            .iter()
            .filter_map(|item| item["guid"].as_str())
            .collect();
        assert_eq!(guids, vec!["msg-attributed"]);
        Ok(())
    }

    #[tokio::test]
    async fn metadata_filters_work_alone_and_in_combination()
    -> Result<(), Box<dyn std::error::Error>> {
        let fixture = seed_search_fixture().await?;
        let pool = connect_pool(fixture.path()).await?;
        let app = router(AppState::new(Some(pool), None, None, None));

        let (status, payload) = response_json(app.clone(), "/v1/messages?direction=sent").await?;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            payload["items"].as_array().ok_or("expected array")?.len(),
            1
        );
        assert_eq!(payload["items"][0]["guid"], "msg-sent");

        let (_, payload) = response_json(app.clone(), "/v1/messages?transport=sms").await?;
        assert_eq!(
            payload["items"].as_array().ok_or("expected array")?.len(),
            1
        );
        assert_eq!(payload["items"][0]["guid"], "msg-group");

        let (_, payload) = response_json(app.clone(), "/v1/messages?sender=%2B15550000001").await?;
        let guids: Vec<_> = payload["items"]
            .as_array()
            .ok_or("expected array")?
            .iter()
            .filter_map(|item| item["guid"].as_str())
            .collect();
        assert!(guids.contains(&"msg-plain"));
        assert!(!guids.contains(&"msg-sent"));

        let (_, payload) = response_json(
            app.clone(),
            "/v1/messages?chat_id=1&direction=received&transport=imessage",
        )
        .await?;
        let guids: Vec<_> = payload["items"]
            .as_array()
            .ok_or("expected array")?
            .iter()
            .filter_map(|item| item["guid"].as_str())
            .collect();
        assert!(guids.contains(&"msg-plain"));
        let (_, payload) =
            response_json(app.clone(), "/v1/messages?content_type=group_event").await?;
        assert_eq!(
            payload["items"].as_array().ok_or("expected array")?.len(),
            1
        );
        assert_eq!(payload["items"][0]["guid"], "msg-group");

        let (_, payload) =
            response_json(app.clone(), "/v1/messages?has_attachments=false&q=Hello").await?;
        assert_eq!(
            payload["items"].as_array().ok_or("expected array")?.len(),
            1
        );
        assert_eq!(payload["items"][0]["guid"], "msg-plain");
        Ok(())
    }

    #[tokio::test]
    async fn invalid_filters_return_structured_400() -> Result<(), Box<dyn std::error::Error>> {
        let fixture = seed_search_fixture().await?;
        let pool = connect_pool(fixture.path()).await?;
        let app = router(AppState::new(Some(pool), None, None, None));

        let (status, payload) = response_json(
            app.clone(),
            "/v1/messages?before=1704067200&after=1706745600",
        )
        .await?;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(payload["error"]["code"], "invalid_timestamp");

        let long_q = "a".repeat(257);
        let (status, _) = response_json(app.clone(), &format!("/v1/messages?q={long_q}")).await?;
        assert_eq!(status, StatusCode::BAD_REQUEST);

        let (status, _) =
            response_json(app.clone(), "/v1/messages?before=not-a-unix-timestamp").await?;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        Ok(())
    }

    #[tokio::test]
    async fn sparse_search_is_resumable() -> Result<(), Box<dyn std::error::Error>> {
        let fixture = seed_search_fixture().await?;
        let pool = connect_pool(fixture.path()).await?;
        let app = router(AppState::new(Some(pool), None, None, None));

        let (status, first) = response_json(app.clone(), "/v1/messages?q=noise&limit=1").await?;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(first["page"]["has_more"], true);
        let cursor = first["page"]["next_cursor"]
            .as_str()
            .ok_or("expected string")?;

        let (_, second) = response_json(
            app.clone(),
            &format!("/v1/messages?q=noise&limit=1&cursor={cursor}"),
        )
        .await?;
        assert_eq!(second["items"].as_array().ok_or("expected array")?.len(), 1);
        assert_ne!(
            first["items"][0]["guid"], second["items"][0]["guid"],
            "pages must advance"
        );
        Ok(())
    }

    #[tokio::test]
    async fn cursor_filter_mismatch_returns_400() -> Result<(), Box<dyn std::error::Error>> {
        let fixture = seed_search_fixture().await?;
        let pool = connect_pool(fixture.path()).await?;
        let app = router(AppState::new(Some(pool), None, None, None));

        let (_, first) = response_json(
            app.clone(),
            "/v1/messages?chat_id=1&direction=received&limit=1",
        )
        .await?;
        let cursor = first["page"]["next_cursor"]
            .as_str()
            .ok_or("expected string")?;

        let (status, payload) = response_json(
            app,
            &format!("/v1/messages?chat_id=2&direction=received&limit=1&cursor={cursor}"),
        )
        .await?;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(payload["error"]["code"], "invalid_cursor");

        let fixture = seed_search_fixture().await?;
        let pool = connect_pool(fixture.path()).await?;
        let app = router(AppState::new(Some(pool), None, None, None));

        let (_, before) = response_json(app.clone(), "/v1/messages?q=brand-new-term").await?;
        assert!(
            before["items"]
                .as_array()
                .ok_or("expected array")?
                .is_empty()
        );

        let mut connection =
            sqlx::SqliteConnection::connect(fixture.path().to_str().ok_or("invalid path")?).await?;
        sqlx::query!(
            "INSERT INTO message (guid, text, service, is_from_me, date) VALUES ('msg-live', 'brand-new-term appears', 'iMessage', 1, 400)"
        )
        .execute(&mut connection)
        .await?;
        connection.close().await.ok();

        let (_, after) = response_json(app, "/v1/messages?q=brand-new-term").await?;
        assert_eq!(after["items"].as_array().ok_or("expected array")?.len(), 1);
        assert_eq!(after["items"][0]["guid"], "msg-live");
        Ok(())
    }
}
