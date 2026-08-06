use std::collections::HashMap;

use sqlx::SqliteExecutor;

use super::{
    classify::classify,
    model::{Chat, Direction, Handle, Message, MessageEnvelope, Transport},
    queries::{
        fetch_attachments_for_message_ids, fetch_chat_ids_for_message_ids,
        fetch_chat_row_by_id as query_chat_row_by_id, fetch_participants_for_chat_ids,
    },
    row::{AttachmentRow, ChatRow, MessageRow, parse_apple_timestamp},
    threads::build_reply_threads,
};
use crate::sqlx_util::json_ids;

pub fn assemble_message(
    row: MessageRow,
    attachments: Vec<AttachmentRow>,
    chat_ids: Vec<i64>,
) -> Message {
    let content = classify(&row, &attachments);
    let envelope = MessageEnvelope {
        row_id: row.row_id,
        guid: row.guid,
        direction: if row.is_from_me {
            Direction::Sent
        } else {
            Direction::Received
        },
        transport: Transport::from_service(row.service.as_deref()),
        sender: row.sender_id.map(|id| Handle {
            id,
            service: row.sender_service.unwrap_or_default(),
        }),
        sent_at: parse_apple_timestamp(row.sent_at),
        read_at: parse_apple_timestamp(row.read_at),
        edited_at: parse_apple_timestamp(row.edited_at),
        retracted_at: parse_apple_timestamp(row.retracted_at),
        reply_to_guid: row.reply_to_guid,
        thread_originator_guid: row.thread_originator_guid,
        chat_ids,
    };

    Message { envelope, content }
}

pub fn assemble_messages(
    rows: Vec<MessageRow>,
    mut attachments_by_message: HashMap<i64, Vec<AttachmentRow>>,
    mut chat_ids_by_message: HashMap<i64, Vec<i64>>,
) -> Vec<Message> {
    rows.into_iter()
        .map(|row| {
            let message_id = row.row_id;
            let attachments = attachments_by_message
                .remove(&message_id)
                .unwrap_or_default();
            let chat_ids = chat_ids_by_message.remove(&message_id).unwrap_or_default();
            assemble_message(row, attachments, chat_ids)
        })
        .collect()
}

pub fn chat_from_row(row: ChatRow, participants: Vec<Handle>, messages: Vec<Message>) -> Chat {
    let reply_threads = build_reply_threads(&messages);
    Chat {
        row_id: row.row_id,
        guid: row.guid,
        identifier: row.chat_identifier,
        display_name: row.display_name,
        room_name: row.room_name,
        transport: Transport::from_service(row.service_name.as_deref()),
        is_group: row.style == Some(43),
        participants,
        messages,
        reply_threads,
    }
}

pub fn chat_summary_from_row(row: ChatRow, participants: Vec<Handle>) -> Chat {
    chat_from_row(row, participants, Vec::new())
}

pub async fn fetch_attachments_for_messages<'e, E>(
    executor: E,
    message_ids: &[i64],
) -> Result<HashMap<i64, Vec<AttachmentRow>>, sqlx::Error>
where
    E: SqliteExecutor<'e>,
{
    let mut attachments_by_message = HashMap::<i64, Vec<AttachmentRow>>::new();
    if message_ids.is_empty() {
        return Ok(attachments_by_message);
    }

    let rows = fetch_attachments_for_message_ids(executor, &json_ids(message_ids)).await?;
    for attachment in rows {
        attachments_by_message
            .entry(attachment.message_id)
            .or_default()
            .push(attachment);
    }

    Ok(attachments_by_message)
}

pub async fn fetch_chat_ids_for_messages<'e, E>(
    executor: E,
    message_ids: &[i64],
) -> Result<HashMap<i64, Vec<i64>>, sqlx::Error>
where
    E: SqliteExecutor<'e>,
{
    let mut chat_ids_by_message = HashMap::<i64, Vec<i64>>::new();
    if message_ids.is_empty() {
        return Ok(chat_ids_by_message);
    }

    let rows = fetch_chat_ids_for_message_ids(executor, &json_ids(message_ids)).await?;
    for join in rows {
        chat_ids_by_message
            .entry(join.message_id)
            .or_default()
            .push(join.chat_id);
    }

    Ok(chat_ids_by_message)
}

pub async fn fetch_participants_for_chats<'e, E>(
    executor: E,
    chat_ids: &[i64],
) -> Result<HashMap<i64, Vec<Handle>>, sqlx::Error>
where
    E: SqliteExecutor<'e>,
{
    let mut participants_by_chat = HashMap::<i64, Vec<Handle>>::new();
    if chat_ids.is_empty() {
        return Ok(participants_by_chat);
    }

    let rows = fetch_participants_for_chat_ids(executor, &json_ids(chat_ids)).await?;
    for join in rows {
        participants_by_chat
            .entry(join.chat_id)
            .or_default()
            .push(Handle {
                id: join.handle_id,
                service: join.handle_service,
            });
    }

    Ok(participants_by_chat)
}

pub async fn fetch_chat_row_by_id<'e, E>(
    executor: E,
    chat_id: i64,
) -> Result<Option<ChatRow>, sqlx::Error>
where
    E: SqliteExecutor<'e>,
{
    query_chat_row_by_id(executor, chat_id).await
}
