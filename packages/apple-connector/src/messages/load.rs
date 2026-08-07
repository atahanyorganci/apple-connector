use std::{collections::HashMap, sync::Arc};

use sqlx::sqlite::SqliteConnection;

use super::{
    assembly::{assemble_message, chat_from_row},
    model::{Chat, Handle, Message},
    row::{AttachmentRow, ChatHandleJoinRow, ChatMessageJoinRow, ChatRow, MessageRow},
};
use crate::apple_types::HandleId;

/// Load every message as a flat list. Each envelope includes `chat_ids` from
/// `chat_message_join`.
pub async fn load_all(connection: &mut SqliteConnection) -> Result<Vec<Message>, sqlx::Error> {
    let (messages, _) = load_library(connection).await?;
    Ok(messages
        .into_iter()
        .map(|message| (*message).clone())
        .collect())
}

/// Load chats with participants, member messages, and reply threads.
pub async fn load_chats(connection: &mut SqliteConnection) -> Result<Vec<Chat>, sqlx::Error> {
    let (_, chats) = load_library(connection).await?;
    Ok(chats)
}

async fn load_library(
    connection: &mut SqliteConnection,
) -> Result<(Vec<Arc<Message>>, Vec<Chat>), sqlx::Error> {
    let message_rows = sqlx::query_as!(
        MessageRow,
        r#"
        SELECT
            message.ROWID AS "row_id!",
            message.guid AS "guid!",
            message.text,
            message.attributedBody AS "attributed_body: Vec<u8>",
            message.service,
            message.date AS "sent_at!",
            message.date_read AS "read_at!",
            message.date_edited AS "edited_at!",
            message.date_retracted AS "retracted_at!",
            message.is_from_me AS "is_from_me!: bool",
            sender.id AS sender_id,
            sender.service AS sender_service,
            message.item_type AS "item_type!",
            message.associated_message_guid,
            message.associated_message_type AS "associated_message_type!",
            message.group_action_type AS "group_action_type!",
            message.group_title,
            message.handle_id AS "handle_id!",
            message.other_handle AS "other_handle!",
            actor.id AS other_handle_id,
            message.share_status AS "share_status!: bool",
            message.balloon_bundle_id,
            message.payload_data AS "payload_data: Vec<u8>",
            message.is_audio_message AS "is_audio_message!: bool",
            message.cache_has_attachments AS "cache_has_attachments!: bool",
            message.is_forward AS "is_forward!: bool",
            message.is_auto_reply AS "is_auto_reply!: bool",
            message.is_system_message AS "is_system_message!: bool",
            message.is_service_message AS "is_service_message!: bool",
            message.reply_to_guid,
            message.thread_originator_guid,
            message.expressive_send_style_id
        FROM message
        LEFT JOIN handle AS sender ON message.handle_id = sender.ROWID
        LEFT JOIN handle AS actor ON message.other_handle = actor.ROWID
        ORDER BY message.date ASC, message.ROWID ASC
        "#,
    )
    .fetch_all(&mut *connection)
    .await?;

    let attachment_rows = sqlx::query_as!(
        AttachmentRow,
        r#"
        SELECT
            message_attachment_join.message_id AS "message_id!",
            attachment.guid AS "guid!",
            attachment.original_guid AS "original_guid!",
            attachment.filename,
            attachment.uti,
            attachment.mime_type,
            attachment.transfer_name,
            attachment.total_bytes AS "total_bytes!",
            attachment.is_sticker AS "is_sticker!: bool",
            attachment.transfer_state AS "transfer_state!",
            attachment.hide_attachment AS "hide_attachment!: bool",
            attachment.emoji_image_short_description AS emoji_description
        FROM message_attachment_join
        JOIN attachment ON message_attachment_join.attachment_id = attachment.ROWID
        ORDER BY message_attachment_join.message_id ASC, attachment.ROWID ASC
        "#,
    )
    .fetch_all(&mut *connection)
    .await?;

    let chat_rows = sqlx::query_as!(
        ChatRow,
        r#"
        SELECT
            chat.ROWID AS "row_id!",
            chat.guid AS "guid!",
            chat.chat_identifier,
            chat.display_name,
            chat.room_name,
            chat.service_name,
            chat.style
        FROM chat
        ORDER BY chat.ROWID ASC
        "#,
    )
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
                id: HandleId::new(join.handle_id),
                service: join.handle_service,
            });
    }

    let messages: Vec<Arc<Message>> = message_rows
        .into_iter()
        .map(|row| {
            let message_id = row.row_id;
            let message_attachments = attachments_by_message
                .remove(&message_id)
                .unwrap_or_default();
            let chat_ids = chat_ids_by_message.remove(&message_id).unwrap_or_default();
            Arc::new(assemble_message(row, message_attachments, chat_ids))
        })
        .collect();

    let messages_by_id: HashMap<i64, Arc<Message>> = messages
        .iter()
        .map(|message| (message.envelope.row_id.get(), Arc::clone(message)))
        .collect();

    let chats = chat_rows
        .into_iter()
        .map(|chat| {
            let chat_messages: Vec<Arc<Message>> = message_ids_by_chat
                .remove(&chat.row_id)
                .unwrap_or_default()
                .into_iter()
                .filter_map(|message_id| messages_by_id.get(&message_id).cloned())
                .collect();
            let participants = participants_by_chat
                .remove(&chat.row_id)
                .unwrap_or_default();
            chat_from_row(chat, participants, chat_messages)
        })
        .collect();

    Ok((messages, chats))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::assemble_message;
    use crate::messages::row::{AttachmentRow, MessageRow};

    #[test]
    fn shared_message_membership_uses_single_allocation() {
        let row = MessageRow {
            row_id: 1,
            guid: "shared-guid".to_owned(),
            text: Some("hello".to_owned()),
            attributed_body: None,
            service: Some("iMessage".to_owned()),
            sent_at: 1,
            read_at: 0,
            edited_at: 0,
            retracted_at: 0,
            is_from_me: true,
            sender_id: None,
            sender_service: None,
            item_type: 0,
            associated_message_guid: None,
            associated_message_type: 0,
            group_action_type: 0,
            group_title: None,
            handle_id: 0,
            other_handle: 0,
            other_handle_id: None,
            share_status: false,
            balloon_bundle_id: None,
            payload_data: None,
            is_audio_message: false,
            cache_has_attachments: false,
            is_forward: false,
            is_auto_reply: false,
            is_system_message: false,
            is_service_message: false,
            reply_to_guid: None,
            thread_originator_guid: None,
            expressive_send_style_id: None,
        };
        let message = Arc::new(assemble_message(
            row,
            Vec::<AttachmentRow>::new(),
            vec![1, 2],
        ));
        let first = Arc::clone(&message);
        let second = Arc::clone(&message);
        assert!(Arc::ptr_eq(&first, &second));
    }
}
