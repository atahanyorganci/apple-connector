use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::{Deserialize, Serialize};

use super::{
    error::{ApiError, ErrorCode},
    params::CURSOR_VERSION,
};
use crate::{
    calendar::EventFiltersSnapshot, messages::search::MessageFiltersSnapshot,
    notes::search::NoteFiltersSnapshot, reminders::ReminderFiltersSnapshot,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListCursor {
    pub row_id: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GlobalReminderCursor {
    pub modified_at: f64,
    pub row_id: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ListReminderCursor {
    pub modified_at: f64,
    pub row_id: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReminderSearchCursor {
    pub modified_at: f64,
    pub row_id: i64,
    pub filters: ReminderFiltersSnapshot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FolderListCursor {
    pub row_id: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GlobalNoteCursor {
    pub modified_at: f64,
    pub row_id: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct FolderNoteCursor {
    pub modified_at: f64,
    pub row_id: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NoteSearchCursor {
    pub modified_at: f64,
    pub row_id: i64,
    pub filters: NoteFiltersSnapshot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GlobalMessageCursor {
    pub date: i64,
    pub row_id: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageSearchCursor {
    pub date: i64,
    pub row_id: i64,
    pub filters: MessageFiltersSnapshot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatMessageCursor {
    pub message_date: i64,
    pub message_id: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatListCursor {
    pub message_date: i64,
    pub message_id: i64,
    pub chat_id: i64,
}

fn invalid_cursor_key(field: &'static str) -> ApiError {
    ApiError::with_details(
        ErrorCode::InvalidCursor,
        "cursor pagination key is invalid",
        serde_json::json!({ "field": field }),
    )
}

fn invalid_cursor(message: impl Into<String>, details: serde_json::Value) -> ApiError {
    ApiError::with_details(ErrorCode::InvalidCursor, message, details)
}

pub trait ValidatedCursor: for<'de> Deserialize<'de> {
    fn validate(&self) -> Result<(), ApiError>;
}

macro_rules! impl_row_id_cursor {
    ($ty:ty) => {
        impl ValidatedCursor for $ty {
            fn validate(&self) -> Result<(), ApiError> {
                if self.row_id <= 0 {
                    return Err(invalid_cursor_key("row_id"));
                }
                Ok(())
            }
        }
    };
}

pub fn encode<T: Serialize>(payload: &T) -> Result<String, ApiError> {
    let json = serde_json::to_vec(payload)
        .map_err(|_| ApiError::internal("failed to encode pagination cursor"))?;
    Ok(format!("{CURSOR_VERSION}.{}", URL_SAFE_NO_PAD.encode(json)))
}

pub fn decode<T: ValidatedCursor>(cursor: &str) -> Result<T, ApiError> {
    let encoded = cursor
        .strip_prefix(&format!("{CURSOR_VERSION}."))
        .ok_or_else(|| {
            invalid_cursor(
                format!("cursor must start with `{CURSOR_VERSION}.`"),
                serde_json::json!({
                    "field": "cursor",
                    "expected_prefix": format!("{CURSOR_VERSION}."),
                }),
            )
        })?;

    let bytes = URL_SAFE_NO_PAD.decode(encoded).map_err(|_| {
        invalid_cursor(
            "cursor is not valid base64url",
            serde_json::json!({ "field": "cursor" }),
        )
    })?;

    let payload: T = serde_json::from_slice(&bytes).map_err(|_| {
        invalid_cursor(
            "cursor payload is invalid",
            serde_json::json!({ "field": "cursor" }),
        )
    })?;
    payload.validate()?;
    Ok(payload)
}

pub fn decode_search_cursor(
    cursor: &str,
    expected_filters: &MessageFiltersSnapshot,
) -> Result<MessageSearchCursor, ApiError> {
    let decoded = decode::<MessageSearchCursor>(cursor)?;
    if decoded.filters != *expected_filters {
        return Err(invalid_cursor(
            "cursor does not match the active filters",
            serde_json::json!({ "field": "cursor" }),
        ));
    }
    Ok(decoded)
}

pub fn decode_global_or_reject_for_filters(cursor: &str) -> Result<GlobalMessageCursor, ApiError> {
    let bytes = cursor
        .strip_prefix(&format!("{CURSOR_VERSION}."))
        .ok_or_else(|| {
            invalid_cursor(
                format!("cursor must start with `{CURSOR_VERSION}.`"),
                serde_json::json!({
                    "field": "cursor",
                    "expected_prefix": format!("{CURSOR_VERSION}."),
                }),
            )
        })?;

    let raw = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(bytes)
        .map_err(|_| {
            invalid_cursor(
                "cursor is not valid base64url",
                serde_json::json!({ "field": "cursor" }),
            )
        })?;

    if serde_json::from_slice::<MessageSearchCursor>(&raw).is_ok() {
        return Err(invalid_cursor(
            "cursor does not match the active filters",
            serde_json::json!({ "field": "cursor" }),
        ));
    }

    decode::<GlobalMessageCursor>(cursor)
}

pub fn decode_reminder_search_cursor(
    cursor: &str,
    expected_filters: &ReminderFiltersSnapshot,
) -> Result<ReminderSearchCursor, ApiError> {
    let decoded = decode::<ReminderSearchCursor>(cursor)?;
    if decoded.filters != *expected_filters {
        return Err(invalid_cursor(
            "cursor does not match the active filters",
            serde_json::json!({ "field": "cursor" }),
        ));
    }
    Ok(decoded)
}

pub fn decode_note_search_cursor(
    cursor: &str,
    expected_filters: &NoteFiltersSnapshot,
) -> Result<NoteSearchCursor, ApiError> {
    let decoded = decode::<NoteSearchCursor>(cursor)?;
    if decoded.filters != *expected_filters {
        return Err(invalid_cursor(
            "cursor does not match the active filters",
            serde_json::json!({ "field": "cursor" }),
        ));
    }
    Ok(decoded)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalendarListCursor {
    pub row_id: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GlobalEventCursor {
    pub modified_at: f64,
    pub row_id: i64,
}

/// Cursor for globally paginating Contacts/Groups across every configured
/// AddressBook source.
///
/// Sources are consumed one at a time in a stable order (ascending by
/// `source_id`); `row_id` is the underlying source's own `Z_PK`-based
/// pagination cursor. A `row_id` of `i64::MAX` means "start this source
/// from the beginning" (used when resuming at the first item of the next
/// source in the sequence).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContactListCursor {
    pub source_id: String,
    pub row_id: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroupContactCursor {
    pub row_id: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CalendarEventCursor {
    pub start_at: f64,
    pub row_id: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventSearchCursor {
    pub start_at: f64,
    pub row_id: i64,
    pub filters: EventFiltersSnapshot,
}

impl_row_id_cursor!(ListCursor);
impl_row_id_cursor!(FolderListCursor);
impl_row_id_cursor!(CalendarListCursor);
impl_row_id_cursor!(GroupContactCursor);

impl ValidatedCursor for ContactListCursor {
    fn validate(&self) -> Result<(), ApiError> {
        if self.source_id.is_empty() {
            return Err(invalid_cursor_key("source_id"));
        }
        if self.row_id <= 0 {
            return Err(invalid_cursor_key("row_id"));
        }
        Ok(())
    }
}

impl ValidatedCursor for GlobalReminderCursor {
    fn validate(&self) -> Result<(), ApiError> {
        if self.row_id <= 0 {
            return Err(invalid_cursor_key("row_id"));
        }
        if self.modified_at == 0.0 {
            return Err(invalid_cursor_key("modified_at"));
        }
        Ok(())
    }
}

impl ValidatedCursor for ListReminderCursor {
    fn validate(&self) -> Result<(), ApiError> {
        if self.row_id <= 0 {
            return Err(invalid_cursor_key("row_id"));
        }
        if self.modified_at == 0.0 {
            return Err(invalid_cursor_key("modified_at"));
        }
        Ok(())
    }
}

impl ValidatedCursor for ReminderSearchCursor {
    fn validate(&self) -> Result<(), ApiError> {
        if self.row_id <= 0 {
            return Err(invalid_cursor_key("row_id"));
        }
        if self.modified_at == 0.0 {
            return Err(invalid_cursor_key("modified_at"));
        }
        Ok(())
    }
}

impl ValidatedCursor for GlobalNoteCursor {
    fn validate(&self) -> Result<(), ApiError> {
        if self.row_id <= 0 {
            return Err(invalid_cursor_key("row_id"));
        }
        if self.modified_at == 0.0 {
            return Err(invalid_cursor_key("modified_at"));
        }
        Ok(())
    }
}

impl ValidatedCursor for FolderNoteCursor {
    fn validate(&self) -> Result<(), ApiError> {
        if self.row_id <= 0 {
            return Err(invalid_cursor_key("row_id"));
        }
        if self.modified_at == 0.0 {
            return Err(invalid_cursor_key("modified_at"));
        }
        Ok(())
    }
}

impl ValidatedCursor for NoteSearchCursor {
    fn validate(&self) -> Result<(), ApiError> {
        if self.row_id <= 0 {
            return Err(invalid_cursor_key("row_id"));
        }
        if self.modified_at == 0.0 {
            return Err(invalid_cursor_key("modified_at"));
        }
        Ok(())
    }
}

impl ValidatedCursor for GlobalMessageCursor {
    fn validate(&self) -> Result<(), ApiError> {
        if self.row_id <= 0 {
            return Err(invalid_cursor_key("row_id"));
        }
        if self.date <= 0 {
            return Err(invalid_cursor_key("date"));
        }
        Ok(())
    }
}

impl ValidatedCursor for MessageSearchCursor {
    fn validate(&self) -> Result<(), ApiError> {
        if self.row_id <= 0 {
            return Err(invalid_cursor_key("row_id"));
        }
        if self.date <= 0 {
            return Err(invalid_cursor_key("date"));
        }
        Ok(())
    }
}

impl ValidatedCursor for ChatMessageCursor {
    fn validate(&self) -> Result<(), ApiError> {
        if self.message_id <= 0 {
            return Err(invalid_cursor_key("message_id"));
        }
        if self.message_date <= 0 {
            return Err(invalid_cursor_key("message_date"));
        }
        Ok(())
    }
}

impl ValidatedCursor for ChatListCursor {
    fn validate(&self) -> Result<(), ApiError> {
        if self.message_id <= 0 {
            return Err(invalid_cursor_key("message_id"));
        }
        if self.message_date <= 0 {
            return Err(invalid_cursor_key("message_date"));
        }
        if self.chat_id <= 0 {
            return Err(invalid_cursor_key("chat_id"));
        }
        Ok(())
    }
}

impl ValidatedCursor for GlobalEventCursor {
    fn validate(&self) -> Result<(), ApiError> {
        if self.row_id <= 0 {
            return Err(invalid_cursor_key("row_id"));
        }
        if self.modified_at == 0.0 {
            return Err(invalid_cursor_key("modified_at"));
        }
        Ok(())
    }
}

impl ValidatedCursor for CalendarEventCursor {
    fn validate(&self) -> Result<(), ApiError> {
        if self.row_id <= 0 {
            return Err(invalid_cursor_key("row_id"));
        }
        if self.start_at == 0.0 {
            return Err(invalid_cursor_key("start_at"));
        }
        Ok(())
    }
}

impl ValidatedCursor for EventSearchCursor {
    fn validate(&self) -> Result<(), ApiError> {
        if self.row_id <= 0 {
            return Err(invalid_cursor_key("row_id"));
        }
        if self.start_at == 0.0 {
            return Err(invalid_cursor_key("start_at"));
        }
        Ok(())
    }
}

pub fn decode_event_search_cursor(
    cursor: &str,
    expected_filters: &EventFiltersSnapshot,
) -> Result<EventSearchCursor, ApiError> {
    let decoded = decode::<EventSearchCursor>(cursor)?;
    if decoded.filters != *expected_filters {
        return Err(invalid_cursor(
            "cursor does not match the active filters",
            serde_json::json!({ "field": "cursor" }),
        ));
    }
    Ok(decoded)
}

#[cfg(test)]
mod tests {
    use super::{ChatListCursor, GlobalMessageCursor, decode, encode};

    #[test]
    fn round_trips_global_message_cursor() -> Result<(), Box<dyn std::error::Error>> {
        let cursor = GlobalMessageCursor {
            date: 123,
            row_id: 456,
        };
        let encoded = encode(&cursor).map_err(|e| -> Box<dyn std::error::Error> {
            Box::new(std::io::Error::other(format!("{e:?}")))
        })?;
        assert!(encoded.starts_with("v1."));
        assert_eq!(
            decode::<GlobalMessageCursor>(&encoded).map_err(|e| -> Box<dyn std::error::Error> {
                Box::new(std::io::Error::other(format!("{e:?}")))
            })?,
            cursor
        );
        Ok(())
    }

    #[test]
    fn round_trips_chat_list_cursor() -> Result<(), Box<dyn std::error::Error>> {
        let cursor = ChatListCursor {
            message_date: 100,
            message_id: 200,
            chat_id: 3,
        };
        let encoded = encode(&cursor).map_err(|e| -> Box<dyn std::error::Error> {
            Box::new(std::io::Error::other(format!("{e:?}")))
        })?;
        assert_eq!(
            decode::<ChatListCursor>(&encoded).map_err(|e| -> Box<dyn std::error::Error> {
                Box::new(std::io::Error::other(format!("{e:?}")))
            })?,
            cursor
        );
        Ok(())
    }

    #[test]
    fn rejects_cursor_with_zero_row_id() -> Result<(), Box<dyn std::error::Error>> {
        let cursor = GlobalMessageCursor {
            date: 123,
            row_id: 0,
        };
        let encoded = encode(&cursor).map_err(|e| -> Box<dyn std::error::Error> {
            Box::new(std::io::Error::other(format!("{e:?}")))
        })?;
        match decode::<GlobalMessageCursor>(&encoded) {
            Err(err) => assert!(format!("{err:?}").contains("row_id")),
            Ok(_) => return Err("zero row_id cursor should be rejected".into()),
        }
        Ok(())
    }
}
