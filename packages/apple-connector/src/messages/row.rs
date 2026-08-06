use chrono::{DateTime, Utc};

/// Seconds between the Unix epoch (1970-01-01) and the Apple/Cocoa epoch (2001-01-01).
const APPLE_EPOCH_UNIX_SECS: i64 = 978_307_200;

#[derive(Debug, sqlx::FromRow)]
pub struct MessageRow {
    pub row_id: i64,
    pub guid: String,
    pub text: Option<String>,
    pub attributed_body: Option<Vec<u8>>,
    pub service: Option<String>,
    /// Nanoseconds since the Apple epoch (`message.date`).
    pub sent_at: i64,
    /// Nanoseconds since the Apple epoch (`message.date_read`).
    pub read_at: i64,
    /// Nanoseconds since the Apple epoch (`message.date_edited`).
    pub edited_at: i64,
    /// Nanoseconds since the Apple epoch (`message.date_retracted`).
    pub retracted_at: i64,
    pub is_from_me: bool,
    pub sender_id: Option<String>,
    pub sender_service: Option<String>,
    pub item_type: i64,
    pub associated_message_guid: Option<String>,
    pub associated_message_type: i64,
    pub group_action_type: i64,
    pub group_title: Option<String>,
    pub handle_id: i64,
    pub other_handle: i64,
    pub other_handle_id: Option<String>,
    pub share_status: bool,
    pub balloon_bundle_id: Option<String>,
    pub payload_data: Option<Vec<u8>>,
    pub is_audio_message: bool,
    pub cache_has_attachments: bool,
    pub is_forward: bool,
    pub is_auto_reply: bool,
    pub is_system_message: bool,
    pub is_service_message: bool,
    pub reply_to_guid: Option<String>,
    pub thread_originator_guid: Option<String>,
    pub expressive_send_style_id: Option<String>,
}

#[derive(Debug, sqlx::FromRow)]
pub struct ChatRow {
    pub row_id: i64,
    pub guid: String,
    pub chat_identifier: Option<String>,
    pub display_name: Option<String>,
    pub room_name: Option<String>,
    pub service_name: Option<String>,
    pub style: Option<i64>,
}

#[derive(Debug, sqlx::FromRow)]
pub struct ChatMessageJoinRow {
    pub chat_id: i64,
    pub message_id: i64,
}

#[derive(Debug, sqlx::FromRow)]
pub struct ChatHandleJoinRow {
    pub chat_id: i64,
    pub handle_id: String,
    pub handle_service: String,
}

#[derive(Debug, sqlx::FromRow)]
pub(crate) struct ChatMessagePageRow {
    pub row_id: i64,
    pub guid: String,
    pub text: Option<String>,
    pub attributed_body: Option<Vec<u8>>,
    pub service: Option<String>,
    pub sent_at: i64,
    pub read_at: i64,
    pub edited_at: i64,
    pub retracted_at: i64,
    pub is_from_me: bool,
    pub sender_id: Option<String>,
    pub sender_service: Option<String>,
    pub item_type: i64,
    pub associated_message_guid: Option<String>,
    pub associated_message_type: i64,
    pub group_action_type: i64,
    pub group_title: Option<String>,
    pub handle_id: i64,
    pub other_handle: i64,
    pub other_handle_id: Option<String>,
    pub share_status: bool,
    pub balloon_bundle_id: Option<String>,
    pub payload_data: Option<Vec<u8>>,
    pub is_audio_message: bool,
    pub cache_has_attachments: bool,
    pub is_forward: bool,
    pub is_auto_reply: bool,
    pub is_system_message: bool,
    pub is_service_message: bool,
    pub reply_to_guid: Option<String>,
    pub thread_originator_guid: Option<String>,
    pub expressive_send_style_id: Option<String>,
    pub join_message_date: i64,
}

impl ChatMessagePageRow {
    pub(crate) fn into_scoped(self) -> ChatScopedMessageRow {
        let Self {
            row_id,
            guid,
            text,
            attributed_body,
            service,
            sent_at,
            read_at,
            edited_at,
            retracted_at,
            is_from_me,
            sender_id,
            sender_service,
            item_type,
            associated_message_guid,
            associated_message_type,
            group_action_type,
            group_title,
            handle_id,
            other_handle,
            other_handle_id,
            share_status,
            balloon_bundle_id,
            payload_data,
            is_audio_message,
            cache_has_attachments,
            is_forward,
            is_auto_reply,
            is_system_message,
            is_service_message,
            reply_to_guid,
            thread_originator_guid,
            expressive_send_style_id,
            join_message_date,
        } = self;
        ChatScopedMessageRow {
            message: MessageRow {
                row_id,
                guid,
                text,
                attributed_body,
                service,
                sent_at,
                read_at,
                edited_at,
                retracted_at,
                is_from_me,
                sender_id,
                sender_service,
                item_type,
                associated_message_guid,
                associated_message_type,
                group_action_type,
                group_title,
                handle_id,
                other_handle,
                other_handle_id,
                share_status,
                balloon_bundle_id,
                payload_data,
                is_audio_message,
                cache_has_attachments,
                is_forward,
                is_auto_reply,
                is_system_message,
                is_service_message,
                reply_to_guid,
                thread_originator_guid,
                expressive_send_style_id,
            },
            join_message_date,
        }
    }
}

#[derive(Debug)]
pub(crate) struct ChatScopedMessageRow {
    pub message: MessageRow,
    pub join_message_date: i64,
}

/// Parse an Apple/Cocoa Core Data timestamp (nanoseconds since 2001-01-01 UTC).
///
/// A value of `0` means "unset" and becomes `None`.
pub fn parse_apple_timestamp(nanos_since_apple_epoch: i64) -> Option<DateTime<Utc>> {
    if nanos_since_apple_epoch == 0 {
        return None;
    }

    let secs = nanos_since_apple_epoch.div_euclid(1_000_000_000) + APPLE_EPOCH_UNIX_SECS;
    let nsecs = u32::try_from(nanos_since_apple_epoch.rem_euclid(1_000_000_000))
        .expect("nanoseconds fit in u32");
    DateTime::from_timestamp(secs, nsecs)
}

#[derive(Debug, sqlx::FromRow)]
pub struct AttachmentRow {
    pub message_id: i64,
    pub guid: String,
    pub original_guid: String,
    pub filename: Option<String>,
    pub uti: Option<String>,
    pub mime_type: Option<String>,
    pub transfer_name: Option<String>,
    pub total_bytes: i64,
    pub is_sticker: bool,
    pub transfer_state: i64,
    pub hide_attachment: bool,
    pub emoji_description: Option<String>,
}

#[derive(Debug, sqlx::FromRow)]
pub struct AttachmentByGuidRow {
    pub guid: String,
    pub original_guid: String,
    pub filename: Option<String>,
    pub uti: Option<String>,
    pub mime_type: Option<String>,
    pub transfer_name: Option<String>,
    pub total_bytes: i64,
    pub is_sticker: bool,
    pub transfer_state: i64,
    pub hide_attachment: bool,
    pub emoji_description: Option<String>,
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};

    use super::parse_apple_timestamp;

    #[test]
    fn unset_zero_is_none() {
        assert_eq!(parse_apple_timestamp(0), None);
    }

    #[test]
    fn parses_known_unix_instant() {
        // 2026-02-15 06:25:12 UTC
        let unix_secs = Utc
            .with_ymd_and_hms(2026, 2, 15, 6, 25, 12)
            .unwrap()
            .timestamp();
        let apple_nanos = (unix_secs - 978_307_200) * 1_000_000_000;

        let parsed = parse_apple_timestamp(apple_nanos).unwrap();
        assert_eq!(parsed.to_rfc3339(), "2026-02-15T06:25:12+00:00");
    }
}
