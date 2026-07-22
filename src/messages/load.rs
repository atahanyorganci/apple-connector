use std::collections::HashMap;

use sqlx::sqlite::SqliteConnection;

use super::{
    classify::classify,
    model::{Direction, Handle, Message, MessageEnvelope, Transport},
    row::{AttachmentRow, MessageRow},
};

pub async fn load_all(connection: &mut SqliteConnection) -> Result<Vec<Message>, sqlx::Error> {
    let rows = sqlx::query_as!(
        MessageRow,
        r#"
        SELECT
            message.ROWID AS "row_id!",
            message.guid AS "guid!",
            message.text,
            message.attributedBody AS attributed_body,
            message.service,
            datetime(message.date / 1000000000 + 978307200, 'unixepoch') AS sent_at,
            datetime(message.date_read / 1000000000 + 978307200, 'unixepoch') AS read_at,
            datetime(message.date_edited / 1000000000 + 978307200, 'unixepoch') AS edited_at,
            datetime(message.date_retracted / 1000000000 + 978307200, 'unixepoch') AS retracted_at,
            message.is_from_me AS "is_from_me!: bool",
            sender.id AS sender_id,
            sender.service AS sender_service,
            message.item_type AS "item_type!",
            message.associated_message_guid,
            message.associated_message_type AS "associated_message_type!",
            message.group_action_type AS "group_action_type!",
            message.group_title,
            actor.id AS other_handle_id,
            message.balloon_bundle_id,
            message.payload_data,
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
            attachment.filename,
            attachment.uti,
            attachment.mime_type,
            attachment.transfer_name,
            attachment.total_bytes AS "total_bytes!",
            attachment.is_sticker AS "is_sticker!: bool"
        FROM message_attachment_join
        JOIN attachment ON message_attachment_join.attachment_id = attachment.ROWID
        ORDER BY message_attachment_join.message_id ASC, attachment.ROWID ASC
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

    let messages = rows
        .into_iter()
        .map(|row| {
            let message_attachments = attachments_by_message
                .remove(&row.row_id)
                .unwrap_or_default();
            assemble_message(row, message_attachments)
        })
        .collect();

    Ok(messages)
}

fn assemble_message(row: MessageRow, attachments: Vec<AttachmentRow>) -> Message {
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
        sent_at: row.sent_at,
        read_at: row.read_at,
        edited_at: row.edited_at,
        retracted_at: row.retracted_at,
        reply_to_guid: row.reply_to_guid,
        thread_originator_guid: row.thread_originator_guid,
    };

    Message { envelope, content }
}
