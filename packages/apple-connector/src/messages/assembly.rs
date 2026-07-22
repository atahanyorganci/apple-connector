use std::collections::HashMap;

use sqlx::{QueryBuilder, Sqlite, SqliteExecutor};

use super::{
    classify::classify,
    model::{Chat, Direction, Handle, Message, MessageEnvelope, Transport},
    row::{
        AttachmentRow, ChatHandleJoinRow, ChatMessageJoinRow, ChatRow, MessageRow,
        parse_apple_timestamp,
    },
    sql::{ATTACHMENT_SELECT, CHAT_SELECT},
    threads::build_reply_threads,
};

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

    let mut builder = QueryBuilder::<Sqlite>::new(ATTACHMENT_SELECT);
    builder.push(" WHERE message_attachment_join.message_id IN (");
    {
        let mut separated = builder.separated(", ");
        for message_id in message_ids {
            separated.push_bind(*message_id);
        }
    }
    builder.push(") ORDER BY message_attachment_join.message_id ASC, attachment.ROWID ASC");

    let rows = builder
        .build_query_as::<AttachmentRow>()
        .fetch_all(executor)
        .await?;

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

    let mut builder = QueryBuilder::<Sqlite>::new(
        "SELECT chat_message_join.chat_id AS chat_id, chat_message_join.message_id AS message_id \
         FROM chat_message_join WHERE chat_message_join.message_id IN (",
    );
    {
        let mut separated = builder.separated(", ");
        for message_id in message_ids {
            separated.push_bind(*message_id);
        }
    }
    builder.push(
        ") ORDER BY chat_message_join.chat_id ASC, chat_message_join.message_date ASC, \
         chat_message_join.message_id ASC",
    );

    let rows = builder
        .build_query_as::<ChatMessageJoinRow>()
        .fetch_all(executor)
        .await?;

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

    let mut builder = QueryBuilder::<Sqlite>::new(
        "SELECT chat_handle_join.chat_id AS chat_id, handle.id AS handle_id, \
         handle.service AS handle_service FROM chat_handle_join \
         JOIN handle ON chat_handle_join.handle_id = handle.ROWID \
         WHERE chat_handle_join.chat_id IN (",
    );
    {
        let mut separated = builder.separated(", ");
        for chat_id in chat_ids {
            separated.push_bind(*chat_id);
        }
    }
    builder.push(") ORDER BY chat_handle_join.chat_id ASC, handle.ROWID ASC");

    let rows = builder
        .build_query_as::<ChatHandleJoinRow>()
        .fetch_all(executor)
        .await?;

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
    let mut builder = QueryBuilder::<Sqlite>::new(CHAT_SELECT);
    builder.push(" WHERE chat.ROWID = ");
    builder.push_bind(chat_id);

    builder
        .build_query_as::<ChatRow>()
        .fetch_optional(executor)
        .await
}
