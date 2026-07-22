use std::collections::HashMap;

use sqlx::sqlite::SqliteConnection;

use super::{
    assembly::{assemble_message, chat_from_row},
    model::{Chat, Handle, Message},
    row::{AttachmentRow, ChatHandleJoinRow, ChatMessageJoinRow, ChatRow, MessageRow},
    sql::{ATTACHMENT_SELECT_ORDERED, CHAT_SELECT_ORDERED, MESSAGE_SELECT_ORDERED_ASC},
};

/// Load every message as a flat list. Each envelope includes `chat_ids` from
/// `chat_message_join`.
pub async fn load_all(connection: &mut SqliteConnection) -> Result<Vec<Message>, sqlx::Error> {
    let (messages, _) = load_library(connection).await?;
    Ok(messages)
}

/// Load chats with participants, member messages, and reply threads.
pub async fn load_chats(connection: &mut SqliteConnection) -> Result<Vec<Chat>, sqlx::Error> {
    let (_, chats) = load_library(connection).await?;
    Ok(chats)
}

async fn load_library(
    connection: &mut SqliteConnection,
) -> Result<(Vec<Message>, Vec<Chat>), sqlx::Error> {
    let message_rows = sqlx::query_as::<_, MessageRow>(MESSAGE_SELECT_ORDERED_ASC)
        .fetch_all(&mut *connection)
        .await?;

    let attachment_rows = sqlx::query_as::<_, AttachmentRow>(ATTACHMENT_SELECT_ORDERED)
        .fetch_all(&mut *connection)
        .await?;

    let chat_rows = sqlx::query_as::<_, ChatRow>(CHAT_SELECT_ORDERED)
        .fetch_all(&mut *connection)
        .await?;

    let chat_message_joins = sqlx::query_as!(
        ChatMessageJoinRow,
        r#"
        SELECT
            chat_message_join.chat_id AS "chat_id!",
            chat_message_join.message_id AS "message_id!"
        FROM chat_message_join
        ORDER BY
            chat_message_join.chat_id ASC,
            chat_message_join.message_date ASC,
            chat_message_join.message_id ASC
        "#,
    )
    .fetch_all(&mut *connection)
    .await?;

    let chat_handle_joins = sqlx::query_as!(
        ChatHandleJoinRow,
        r#"
        SELECT
            chat_handle_join.chat_id AS "chat_id!",
            handle.id AS "handle_id!",
            handle.service AS "handle_service!"
        FROM chat_handle_join
        JOIN handle ON chat_handle_join.handle_id = handle.ROWID
        ORDER BY chat_handle_join.chat_id ASC, handle.ROWID ASC
        "#,
    )
    .fetch_all(&mut *connection)
    .await?;

    let mut attachments_by_message = HashMap::<i64, Vec<AttachmentRow>>::new();
    for attachment in attachment_rows {
        attachments_by_message
            .entry(attachment.message_id)
            .or_default()
            .push(attachment);
    }

    let mut chat_ids_by_message = HashMap::<i64, Vec<i64>>::new();
    let mut message_ids_by_chat = HashMap::<i64, Vec<i64>>::new();
    for join in chat_message_joins {
        chat_ids_by_message
            .entry(join.message_id)
            .or_default()
            .push(join.chat_id);
        message_ids_by_chat
            .entry(join.chat_id)
            .or_default()
            .push(join.message_id);
    }

    let mut participants_by_chat = HashMap::<i64, Vec<Handle>>::new();
    for join in chat_handle_joins {
        participants_by_chat
            .entry(join.chat_id)
            .or_default()
            .push(Handle {
                id: join.handle_id,
                service: join.handle_service,
            });
    }

    let messages: Vec<Message> = message_rows
        .into_iter()
        .map(|row| {
            let message_id = row.row_id;
            let message_attachments = attachments_by_message
                .remove(&message_id)
                .unwrap_or_default();
            let chat_ids = chat_ids_by_message.remove(&message_id).unwrap_or_default();
            assemble_message(row, message_attachments, chat_ids)
        })
        .collect();

    let messages_by_id: HashMap<i64, &Message> = messages
        .iter()
        .map(|message| (message.envelope.row_id, message))
        .collect();

    let chats = chat_rows
        .into_iter()
        .map(|chat| {
            let chat_messages: Vec<Message> = message_ids_by_chat
                .remove(&chat.row_id)
                .unwrap_or_default()
                .into_iter()
                .filter_map(|message_id| messages_by_id.get(&message_id).copied().cloned())
                .collect();
            let participants = participants_by_chat
                .remove(&chat.row_id)
                .unwrap_or_default();
            chat_from_row(chat, participants, chat_messages)
        })
        .collect();

    Ok((messages, chats))
}
