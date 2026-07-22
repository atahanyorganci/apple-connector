//! Metadata filters and bounded text search for global message listing.

use sqlx::{QueryBuilder, Sqlite};

use super::{attributed_body, row::MessageRow, sql::MESSAGE_SELECT_CORE};

pub const MESSAGE_SCAN_BUDGET: u32 = 500;
pub const CANDIDATE_CHUNK_SIZE: u32 = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DirectionFilter {
    Sent,
    Received,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransportFilter {
    Imessage,
    Sms,
    Rcs,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentTypeFilter {
    Text,
    Audio,
    Attachment,
    Reaction,
    GroupEvent,
    AppBalloon,
    SharePlay,
    ShareMyLocation,
    System,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MessageFilters {
    pub q: Option<String>,
    pub chat_id: Option<i64>,
    pub sender: Option<String>,
    pub before: Option<i64>,
    pub after: Option<i64>,
    pub direction: Option<DirectionFilter>,
    pub transport: Option<TransportFilter>,
    pub content_type: Option<ContentTypeFilter>,
    pub has_attachments: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MessageFiltersSnapshot {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub q: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chat_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sender: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub direction: Option<DirectionFilter>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transport: Option<TransportFilter>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_type: Option<ContentTypeFilter>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_attachments: Option<bool>,
}

impl MessageFilters {
    pub fn is_active(&self) -> bool {
        self.q.is_some()
            || self.chat_id.is_some()
            || self.sender.is_some()
            || self.before.is_some()
            || self.after.is_some()
            || self.direction.is_some()
            || self.transport.is_some()
            || self.content_type.is_some()
            || self.has_attachments.is_some()
    }

    pub fn requires_text_scan(&self) -> bool {
        self.q.is_some()
    }

    pub fn snapshot(&self) -> MessageFiltersSnapshot {
        MessageFiltersSnapshot {
            q: self.q.clone(),
            chat_id: self.chat_id,
            sender: self.sender.clone(),
            before: self.before,
            after: self.after,
            direction: self.direction,
            transport: self.transport,
            content_type: self.content_type,
            has_attachments: self.has_attachments,
        }
    }
}

pub fn searchable_text(row: &MessageRow) -> Option<String> {
    if let Some(text) = row
        .text
        .as_deref()
        .map(str::trim)
        .filter(|text| !text.is_empty())
    {
        return Some(text.to_owned());
    }

    row.attributed_body
        .as_deref()
        .and_then(|data| attributed_body::decode(data).ok())
        .and_then(|body| body.text)
}

pub fn text_matches(row: &MessageRow, query: &str) -> bool {
    let needle = query.to_lowercase();
    searchable_text(row).is_some_and(|text| text.to_lowercase().contains(&needle))
}

pub fn build_filtered_select(filters: &MessageFilters) -> QueryBuilder<Sqlite> {
    let mut builder = QueryBuilder::new(MESSAGE_SELECT_CORE);
    builder.push(" WHERE 1=1 ");
    push_metadata_filters(&mut builder, filters);
    builder
}

pub fn push_metadata_filters(builder: &mut QueryBuilder<Sqlite>, filters: &MessageFilters) {
    if let Some(chat_id) = filters.chat_id {
        builder.push(
            " AND EXISTS (\
                SELECT 1 FROM chat_message_join cmj \
                WHERE cmj.message_id = message.ROWID AND cmj.chat_id = ",
        );
        builder.push_bind(chat_id);
        builder.push(")");
    }

    if let Some(sender) = &filters.sender {
        builder.push(" AND message.is_from_me = 0 AND sender.id = ");
        builder.push_bind(sender);
    }

    if let Some(after) = filters.after {
        builder.push(" AND message.date > ");
        builder.push_bind(after);
    }

    if let Some(before) = filters.before {
        builder.push(" AND message.date < ");
        builder.push_bind(before);
    }

    if let Some(direction) = filters.direction {
        builder.push(" AND message.is_from_me = ");
        builder.push_bind(match direction {
            DirectionFilter::Sent => 1_i64,
            DirectionFilter::Received => 0_i64,
        });
    }

    if let Some(transport) = filters.transport {
        match transport {
            TransportFilter::Imessage => {
                builder.push(" AND message.service = ");
                builder.push_bind("iMessage");
            }
            TransportFilter::Sms => {
                builder.push(" AND message.service = ");
                builder.push_bind("SMS");
            }
            TransportFilter::Rcs => {
                builder.push(" AND message.service = ");
                builder.push_bind("RCS");
            }
            TransportFilter::Unknown => {
                builder.push(
                    " AND (message.service IS NULL OR message.service NOT IN ('iMessage', 'SMS', 'RCS'))",
                );
            }
        }
    }

    if let Some(has_attachments) = filters.has_attachments {
        builder.push(" AND message.cache_has_attachments = ");
        builder.push_bind(i64::from(has_attachments));
    }

    if let Some(content_type) = filters.content_type {
        push_content_type_filter(builder, content_type);
    }
}

pub fn push_scan_cursor(
    builder: &mut QueryBuilder<Sqlite>,
    cursor_date: Option<i64>,
    cursor_row_id: Option<i64>,
) {
    if cursor_date.is_some() {
        builder.push(" AND (message.date < ");
        builder.push_bind(cursor_date);
        builder.push(" OR (message.date = ");
        builder.push_bind(cursor_date);
        builder.push(" AND message.ROWID < ");
        builder.push_bind(cursor_row_id);
        builder.push("))");
    }
}

fn push_content_type_filter(builder: &mut QueryBuilder<Sqlite>, content_type: ContentTypeFilter) {
    match content_type {
        ContentTypeFilter::Text => {
            builder.push(
                " AND message.item_type = 0 \
                  AND message.associated_message_type = 0 \
                  AND message.is_system_message = 0 \
                  AND message.is_service_message = 0 \
                  AND message.is_audio_message = 0 \
                  AND message.cache_has_attachments = 0 \
                  AND (message.balloon_bundle_id IS NULL OR message.balloon_bundle_id = '')",
            );
        }
        ContentTypeFilter::Audio => {
            builder.push(" AND message.item_type = 0 AND message.is_audio_message = 1");
        }
        ContentTypeFilter::Attachment => {
            builder.push(
                " AND message.item_type = 0 \
                  AND message.cache_has_attachments = 1 \
                  AND message.is_audio_message = 0 \
                  AND (message.balloon_bundle_id IS NULL OR message.balloon_bundle_id = '')",
            );
        }
        ContentTypeFilter::Reaction => {
            builder.push(" AND message.associated_message_type != 0");
        }
        ContentTypeFilter::GroupEvent => {
            builder.push(" AND message.item_type IN (1, 2, 3)");
        }
        ContentTypeFilter::AppBalloon => {
            builder.push(
                " AND message.item_type = 0 \
                  AND message.balloon_bundle_id IS NOT NULL \
                  AND message.balloon_bundle_id != ''",
            );
        }
        ContentTypeFilter::SharePlay => {
            builder.push(" AND message.item_type = 6");
        }
        ContentTypeFilter::ShareMyLocation => {
            builder.push(" AND message.item_type = 4");
        }
        ContentTypeFilter::System => {
            builder.push(" AND (message.is_system_message = 1 OR message.is_service_message = 1)");
        }
        ContentTypeFilter::Unknown => {
            builder.push(
                " AND message.associated_message_type = 0 \
                  AND message.is_system_message = 0 \
                  AND message.is_service_message = 0 \
                  AND message.item_type NOT IN (1, 2, 3, 4, 6) \
                  AND NOT (\
                    message.item_type = 0 \
                    AND message.is_audio_message = 0 \
                    AND message.cache_has_attachments = 0 \
                    AND (message.balloon_bundle_id IS NULL OR message.balloon_bundle_id = '')\
                  ) \
                  AND NOT (\
                    message.item_type = 0 \
                    AND message.is_audio_message = 0 \
                    AND message.cache_has_attachments = 1 \
                    AND (message.balloon_bundle_id IS NULL OR message.balloon_bundle_id = '')\
                  ) \
                  AND NOT (\
                    message.item_type = 0 \
                    AND message.is_audio_message = 1\
                  ) \
                  AND NOT (\
                    message.item_type = 0 \
                    AND message.balloon_bundle_id IS NOT NULL \
                    AND message.balloon_bundle_id != ''\
                  )",
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{MessageFilters, searchable_text, text_matches};
    use crate::messages::row::MessageRow;

    const HELLO_FIXTURE: &[u8] =
        include_bytes!("../../fixtures/messages/attributed-body-hello.bin");

    fn empty_row() -> MessageRow {
        MessageRow {
            row_id: 1,
            guid: "g".to_owned(),
            text: None,
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
        }
    }

    #[test]
    fn attributed_body_only_text_is_searchable() {
        let mut row = empty_row();
        row.attributed_body = Some(HELLO_FIXTURE.to_vec());

        assert_eq!(searchable_text(&row).as_deref(), Some("Noter test"));
        assert!(text_matches(&row, "noter"));
        assert!(text_matches(&row, "TEST"));
        assert!(!text_matches(&row, "missing"));
    }

    #[test]
    fn plain_text_preferred_over_attributed_body() {
        let mut row = empty_row();
        row.text = Some("plain hello".to_owned());
        row.attributed_body = Some(HELLO_FIXTURE.to_vec());

        assert_eq!(searchable_text(&row).as_deref(), Some("plain hello"));
    }

    #[test]
    fn inactive_filters_detect_empty_state() {
        assert!(!MessageFilters::default().is_active());
        assert!(
            MessageFilters {
                q: Some("x".to_owned()),
                ..Default::default()
            }
            .is_active()
        );
    }
}
