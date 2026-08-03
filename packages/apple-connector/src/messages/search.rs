//! Metadata filters and bounded text search for global message listing.

use super::row::MessageRow;

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

/// Bind parameters for the compile-time filtered message listing query.
#[derive(Debug, Clone)]
pub struct MessageFilterBinds {
    pub chat_id: Option<i64>,
    pub sender: Option<String>,
    pub after: Option<i64>,
    pub before: Option<i64>,
    pub direction: Option<i64>,
    pub transport: Option<i64>,
    pub content_type: Option<i64>,
    pub has_attachments: Option<i64>,
    pub cursor_date: Option<i64>,
    pub cursor_row_id: Option<i64>,
    pub limit: i64,
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

    pub fn bind_values(
        &self,
        cursor_date: Option<i64>,
        cursor_row_id: Option<i64>,
        limit: i64,
    ) -> MessageFilterBinds {
        MessageFilterBinds {
            chat_id: self.chat_id,
            sender: self.sender.clone(),
            after: self.after,
            before: self.before,
            direction: self.direction.map(|value| match value {
                DirectionFilter::Sent => 1,
                DirectionFilter::Received => 0,
            }),
            transport: self.transport.map(transport_code),
            content_type: self.content_type.map(content_type_code),
            has_attachments: self.has_attachments.map(i64::from),
            cursor_date,
            cursor_row_id,
            limit,
        }
    }
}

fn transport_code(filter: TransportFilter) -> i64 {
    match filter {
        TransportFilter::Imessage => 1,
        TransportFilter::Sms => 2,
        TransportFilter::Rcs => 3,
        TransportFilter::Unknown => 4,
    }
}

fn content_type_code(filter: ContentTypeFilter) -> i64 {
    match filter {
        ContentTypeFilter::Text => 1,
        ContentTypeFilter::Audio => 2,
        ContentTypeFilter::Attachment => 3,
        ContentTypeFilter::Reaction => 4,
        ContentTypeFilter::GroupEvent => 5,
        ContentTypeFilter::AppBalloon => 6,
        ContentTypeFilter::SharePlay => 7,
        ContentTypeFilter::ShareMyLocation => 8,
        ContentTypeFilter::System => 9,
        ContentTypeFilter::Unknown => 10,
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
        .and_then(|data| super::attributed_body::decode(data).ok())
        .and_then(|body| body.text)
}

pub fn text_matches(row: &MessageRow, query: &str) -> bool {
    let needle = query.to_lowercase();
    searchable_text(row).is_some_and(|text| text.to_lowercase().contains(&needle))
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
