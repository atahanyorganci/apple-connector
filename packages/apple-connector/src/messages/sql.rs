//! Shared SQL fragments for message and chat queries.

pub const MESSAGE_SELECT_ORDERED_ASC: &str = r#"
SELECT
    message.ROWID AS row_id,
    message.guid AS guid,
    message.text,
    message.attributedBody AS attributed_body,
    message.service,
    message.date AS sent_at,
    message.date_read AS read_at,
    message.date_edited AS edited_at,
    message.date_retracted AS retracted_at,
    message.is_from_me AS is_from_me,
    sender.id AS sender_id,
    sender.service AS sender_service,
    message.item_type AS item_type,
    message.associated_message_guid,
    message.associated_message_type AS associated_message_type,
    message.group_action_type AS group_action_type,
    message.group_title,
    message.handle_id AS handle_id,
    message.other_handle AS other_handle,
    actor.id AS other_handle_id,
    message.share_status AS share_status,
    message.balloon_bundle_id,
    message.payload_data,
    message.is_audio_message AS is_audio_message,
    message.cache_has_attachments AS cache_has_attachments,
    message.is_forward AS is_forward,
    message.is_auto_reply AS is_auto_reply,
    message.is_system_message AS is_system_message,
    message.is_service_message AS is_service_message,
    message.reply_to_guid,
    message.thread_originator_guid,
    message.expressive_send_style_id
FROM message
LEFT JOIN handle AS sender ON message.handle_id = sender.ROWID
LEFT JOIN handle AS actor ON message.other_handle = actor.ROWID
ORDER BY message.date ASC, message.ROWID ASC
"#;

pub const GLOBAL_MESSAGE_PAGE: &str = r#"
SELECT
    message.ROWID AS row_id,
    message.guid AS guid,
    message.text,
    message.attributedBody AS attributed_body,
    message.service,
    message.date AS sent_at,
    message.date_read AS read_at,
    message.date_edited AS edited_at,
    message.date_retracted AS retracted_at,
    message.is_from_me AS is_from_me,
    sender.id AS sender_id,
    sender.service AS sender_service,
    message.item_type AS item_type,
    message.associated_message_guid,
    message.associated_message_type AS associated_message_type,
    message.group_action_type AS group_action_type,
    message.group_title,
    message.handle_id AS handle_id,
    message.other_handle AS other_handle,
    actor.id AS other_handle_id,
    message.share_status AS share_status,
    message.balloon_bundle_id,
    message.payload_data,
    message.is_audio_message AS is_audio_message,
    message.cache_has_attachments AS cache_has_attachments,
    message.is_forward AS is_forward,
    message.is_auto_reply AS is_auto_reply,
    message.is_system_message AS is_system_message,
    message.is_service_message AS is_service_message,
    message.reply_to_guid,
    message.thread_originator_guid,
    message.expressive_send_style_id
FROM message
LEFT JOIN handle AS sender ON message.handle_id = sender.ROWID
LEFT JOIN handle AS actor ON message.other_handle = actor.ROWID
WHERE (?1 IS NULL OR message.date < ?1 OR (message.date = ?1 AND message.ROWID < ?2))
ORDER BY message.date DESC, message.ROWID DESC
LIMIT ?3
"#;

pub const MESSAGE_BY_GUID: &str = r#"
SELECT
    message.ROWID AS row_id,
    message.guid AS guid,
    message.text,
    message.attributedBody AS attributed_body,
    message.service,
    message.date AS sent_at,
    message.date_read AS read_at,
    message.date_edited AS edited_at,
    message.date_retracted AS retracted_at,
    message.is_from_me AS is_from_me,
    sender.id AS sender_id,
    sender.service AS sender_service,
    message.item_type AS item_type,
    message.associated_message_guid,
    message.associated_message_type AS associated_message_type,
    message.group_action_type AS group_action_type,
    message.group_title,
    message.handle_id AS handle_id,
    message.other_handle AS other_handle,
    actor.id AS other_handle_id,
    message.share_status AS share_status,
    message.balloon_bundle_id,
    message.payload_data,
    message.is_audio_message AS is_audio_message,
    message.cache_has_attachments AS cache_has_attachments,
    message.is_forward AS is_forward,
    message.is_auto_reply AS is_auto_reply,
    message.is_system_message AS is_system_message,
    message.is_service_message AS is_service_message,
    message.reply_to_guid,
    message.thread_originator_guid,
    message.expressive_send_style_id
FROM message
LEFT JOIN handle AS sender ON message.handle_id = sender.ROWID
LEFT JOIN handle AS actor ON message.other_handle = actor.ROWID
WHERE message.guid = ?1
"#;

pub const CHAT_MESSAGE_PAGE: &str = r#"
SELECT
    message.ROWID AS row_id,
    message.guid AS guid,
    message.text,
    message.attributedBody AS attributed_body,
    message.service,
    message.date AS sent_at,
    message.date_read AS read_at,
    message.date_edited AS edited_at,
    message.date_retracted AS retracted_at,
    message.is_from_me AS is_from_me,
    sender.id AS sender_id,
    sender.service AS sender_service,
    message.item_type AS item_type,
    message.associated_message_guid,
    message.associated_message_type AS associated_message_type,
    message.group_action_type AS group_action_type,
    message.group_title,
    message.handle_id AS handle_id,
    message.other_handle AS other_handle,
    actor.id AS other_handle_id,
    message.share_status AS share_status,
    message.balloon_bundle_id,
    message.payload_data,
    message.is_audio_message AS is_audio_message,
    message.cache_has_attachments AS cache_has_attachments,
    message.is_forward AS is_forward,
    message.is_auto_reply AS is_auto_reply,
    message.is_system_message AS is_system_message,
    message.is_service_message AS is_service_message,
    message.reply_to_guid,
    message.thread_originator_guid,
    message.expressive_send_style_id,
    cmj.message_date AS join_message_date
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
"#;

pub const ATTACHMENT_SELECT: &str = r#"
SELECT
    message_attachment_join.message_id AS message_id,
    attachment.guid AS guid,
    attachment.original_guid AS original_guid,
    attachment.filename,
    attachment.uti,
    attachment.mime_type,
    attachment.transfer_name,
    attachment.total_bytes AS total_bytes,
    attachment.is_sticker AS is_sticker,
    attachment.transfer_state AS transfer_state,
    attachment.hide_attachment AS hide_attachment,
    attachment.emoji_image_short_description AS emoji_description
FROM message_attachment_join
JOIN attachment ON message_attachment_join.attachment_id = attachment.ROWID
"#;

pub const ATTACHMENT_SELECT_ORDERED: &str = r#"
SELECT
    message_attachment_join.message_id AS message_id,
    attachment.guid AS guid,
    attachment.original_guid AS original_guid,
    attachment.filename,
    attachment.uti,
    attachment.mime_type,
    attachment.transfer_name,
    attachment.total_bytes AS total_bytes,
    attachment.is_sticker AS is_sticker,
    attachment.transfer_state AS transfer_state,
    attachment.hide_attachment AS hide_attachment,
    attachment.emoji_image_short_description AS emoji_description
FROM message_attachment_join
JOIN attachment ON message_attachment_join.attachment_id = attachment.ROWID
ORDER BY message_attachment_join.message_id ASC, attachment.ROWID ASC
"#;

pub const CHAT_SELECT: &str = r#"
SELECT
    chat.ROWID AS row_id,
    chat.guid AS guid,
    chat.chat_identifier,
    chat.display_name,
    chat.room_name,
    chat.service_name,
    chat.style
FROM chat
"#;

pub const CHAT_SELECT_ORDERED: &str = r#"
SELECT
    chat.ROWID AS row_id,
    chat.guid AS guid,
    chat.chat_identifier,
    chat.display_name,
    chat.room_name,
    chat.service_name,
    chat.style
FROM chat
ORDER BY chat.ROWID ASC
"#;
