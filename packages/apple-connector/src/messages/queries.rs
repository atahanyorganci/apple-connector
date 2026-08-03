//! Compile-time checked message queries.

use sqlx::{SqliteExecutor, SqlitePool};

use super::{
    row::{AttachmentByGuidRow, AttachmentRow, ChatMessageJoinRow, ChatMessagePageRow, ChatRow, MessageRow},
    search::MessageFilterBinds,
};

pub async fn fetch_filtered_messages<'e, E>(
    executor: E,
    binds: &MessageFilterBinds,
) -> Result<Vec<MessageRow>, sqlx::Error>
where
    E: SqliteExecutor<'e>,
{
    let transport = binds.transport;
    let content_type = binds.content_type;
    sqlx::query_as!(
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
        WHERE 1=1
          AND (
            ?1 IS NULL
            OR EXISTS (
                SELECT 1 FROM chat_message_join cmj
                WHERE cmj.message_id = message.ROWID AND cmj.chat_id = ?1
            )
          )
          AND (
            ?2 IS NULL
            OR (message.is_from_me = 0 AND sender.id = ?2)
          )
          AND (?3 IS NULL OR message.date > ?3)
          AND (?4 IS NULL OR message.date < ?4)
          AND (?5 IS NULL OR message.is_from_me = ?5)
          AND (
            ?6 IS NULL
            OR (?6 = 1 AND message.service = 'iMessage')
            OR (?6 = 2 AND message.service = 'SMS')
            OR (?6 = 3 AND message.service = 'RCS')
            OR (
                ?6 = 4
                AND (
                    message.service IS NULL
                    OR message.service NOT IN ('iMessage', 'SMS', 'RCS')
                )
            )
          )
          AND (?7 IS NULL OR message.cache_has_attachments = ?7)
          AND (
            ?8 IS NULL
            OR (
                ?8 = 1
                AND message.item_type = 0
                AND message.associated_message_type = 0
                AND message.is_system_message = 0
                AND message.is_service_message = 0
                AND message.is_audio_message = 0
                AND message.cache_has_attachments = 0
                AND (message.balloon_bundle_id IS NULL OR message.balloon_bundle_id = '')
            )
            OR (?8 = 2 AND message.item_type = 0 AND message.is_audio_message = 1)
            OR (
                ?8 = 3
                AND message.item_type = 0
                AND message.cache_has_attachments = 1
                AND message.is_audio_message = 0
                AND (message.balloon_bundle_id IS NULL OR message.balloon_bundle_id = '')
            )
            OR (?8 = 4 AND message.associated_message_type != 0)
            OR (?8 = 5 AND message.item_type IN (1, 2, 3))
            OR (
                ?8 = 6
                AND message.item_type = 0
                AND message.balloon_bundle_id IS NOT NULL
                AND message.balloon_bundle_id != ''
            )
            OR (?8 = 7 AND message.item_type = 6)
            OR (?8 = 8 AND message.item_type = 4)
            OR (?8 = 9 AND (message.is_system_message = 1 OR message.is_service_message = 1))
            OR (
                ?8 = 10
                AND message.associated_message_type = 0
                AND message.is_system_message = 0
                AND message.is_service_message = 0
                AND message.item_type NOT IN (1, 2, 3, 4, 6)
                AND NOT (
                    message.item_type = 0
                    AND message.is_audio_message = 0
                    AND message.cache_has_attachments = 0
                    AND (message.balloon_bundle_id IS NULL OR message.balloon_bundle_id = '')
                )
                AND NOT (
                    message.item_type = 0
                    AND message.is_audio_message = 0
                    AND message.cache_has_attachments = 1
                    AND (message.balloon_bundle_id IS NULL OR message.balloon_bundle_id = '')
                )
                AND NOT (message.item_type = 0 AND message.is_audio_message = 1)
                AND NOT (
                    message.item_type = 0
                    AND message.balloon_bundle_id IS NOT NULL
                    AND message.balloon_bundle_id != ''
                )
            )
          )
          AND (
            ?9 IS NULL
            OR message.date < ?9
            OR (message.date = ?9 AND message.ROWID < ?10)
          )
        ORDER BY message.date DESC, message.ROWID DESC
        LIMIT ?11
        "#,
        binds.chat_id,
        binds.sender,
        binds.after,
        binds.before,
        binds.direction,
        transport,
        binds.has_attachments,
        content_type,
        binds.cursor_date,
        binds.cursor_row_id,
        binds.limit,
    )
    .fetch_all(executor)
    .await
}

pub async fn fetch_chat_message_page(
    pool: &SqlitePool,
    chat_id: i64,
    cursor_date: Option<i64>,
    cursor_message_id: Option<i64>,
    limit: i64,
) -> Result<Vec<super::row::ChatScopedMessageRow>, sqlx::Error> {
    let rows = sqlx::query_as!(
        ChatMessagePageRow,
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
            message.expressive_send_style_id,
            cmj.message_date AS "join_message_date!"
        FROM message
        LEFT JOIN handle AS sender ON message.handle_id = sender.ROWID
        LEFT JOIN handle AS actor ON message.other_handle = actor.ROWID
        INNER JOIN chat_message_join AS cmj ON cmj.message_id = message.ROWID
        WHERE cmj.chat_id = ?1
          AND (
            ?2 IS NULL
            OR cmj.message_date < ?2
            OR (cmj.message_date = ?2 AND cmj.message_id < ?3)
          )
        ORDER BY cmj.message_date DESC, cmj.message_id DESC
        LIMIT ?4
        "#,
        chat_id,
        cursor_date,
        cursor_message_id,
        limit,
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(ChatMessagePageRow::into_scoped).collect())
}

pub async fn fetch_message_by_guid(
    pool: &SqlitePool,
    guid: &str,
) -> Result<Option<MessageRow>, sqlx::Error> {
    sqlx::query_as!(
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
        WHERE message.guid = ?1
        "#,
        guid,
    )
    .fetch_optional(pool)
    .await
}

pub async fn fetch_attachment_by_guid(
    pool: &SqlitePool,
    guid: &str,
) -> Result<Option<AttachmentByGuidRow>, sqlx::Error> {
    sqlx::query_as!(
        AttachmentByGuidRow,
        r#"
        SELECT
            attachment.guid AS "guid!",
            attachment.original_guid AS original_guid,
            attachment.filename,
            attachment.uti,
            attachment.mime_type,
            attachment.transfer_name,
            attachment.total_bytes AS "total_bytes!",
            attachment.is_sticker AS "is_sticker!: bool",
            attachment.transfer_state AS "transfer_state!",
            attachment.hide_attachment AS "hide_attachment!: bool",
            attachment.emoji_image_short_description AS emoji_description
        FROM attachment
        WHERE attachment.guid = ?1
        "#,
        guid,
    )
    .fetch_optional(pool)
    .await
}

pub async fn fetch_attachments_for_message_ids<'e, E>(
    executor: E,
    ids_json: &str,
) -> Result<Vec<AttachmentRow>, sqlx::Error>
where
    E: SqliteExecutor<'e>,
{
    sqlx::query_as!(
        AttachmentRow,
        r#"
        SELECT
            message_attachment_join.message_id AS "message_id!",
            attachment.guid AS "guid!",
            attachment.original_guid AS original_guid,
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
        WHERE message_attachment_join.message_id IN (SELECT value FROM json_each(?1))
        ORDER BY message_attachment_join.message_id ASC, attachment.ROWID ASC
        "#,
        ids_json,
    )
    .fetch_all(executor)
    .await
}

pub async fn fetch_chat_ids_for_message_ids<'e, E>(
    executor: E,
    ids_json: &str,
) -> Result<Vec<ChatMessageJoinRow>, sqlx::Error>
where
    E: SqliteExecutor<'e>,
{
    sqlx::query_as!(
        ChatMessageJoinRow,
        r#"
        SELECT
            chat_message_join.chat_id AS "chat_id!",
            chat_message_join.message_id AS "message_id!"
        FROM chat_message_join
        WHERE chat_message_join.message_id IN (SELECT value FROM json_each(?1))
        ORDER BY chat_message_join.chat_id ASC, chat_message_join.message_date ASC, chat_message_join.message_id ASC
        "#,
        ids_json,
    )
    .fetch_all(executor)
    .await
}

pub async fn fetch_participants_for_chat_ids<'e, E>(
    executor: E,
    ids_json: &str,
) -> Result<Vec<super::row::ChatHandleJoinRow>, sqlx::Error>
where
    E: SqliteExecutor<'e>,
{
    sqlx::query_as!(
        super::row::ChatHandleJoinRow,
        r#"
        SELECT
            chat_handle_join.chat_id AS "chat_id!",
            handle.id AS "handle_id!",
            handle.service AS "handle_service!"
        FROM chat_handle_join
        JOIN handle ON chat_handle_join.handle_id = handle.ROWID
        WHERE chat_handle_join.chat_id IN (SELECT value FROM json_each(?1))
        ORDER BY chat_handle_join.chat_id ASC, handle.ROWID ASC
        "#,
        ids_json,
    )
    .fetch_all(executor)
    .await
}

pub async fn fetch_chat_row_by_id<'e, E>(
    executor: E,
    chat_id: i64,
) -> Result<Option<ChatRow>, sqlx::Error>
where
    E: SqliteExecutor<'e>,
{
    sqlx::query_as!(
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
        WHERE chat.ROWID = ?1
        "#,
        chat_id,
    )
    .fetch_optional(executor)
    .await
}
