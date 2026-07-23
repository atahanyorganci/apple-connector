use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::{Deserialize, Serialize};

use super::{error::ApiError, params::CURSOR_VERSION};
use crate::{
    messages::search::MessageFiltersSnapshot, notes::search::NoteFiltersSnapshot,
    reminders::ReminderFiltersSnapshot,
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

pub fn encode<T: Serialize>(payload: &T) -> Result<String, ApiError> {
    let json = serde_json::to_vec(payload)
        .map_err(|_| ApiError::internal("failed to encode pagination cursor"))?;
    Ok(format!("{CURSOR_VERSION}.{}", URL_SAFE_NO_PAD.encode(json)))
}

pub fn decode<T: for<'de> Deserialize<'de>>(cursor: &str) -> Result<T, ApiError> {
    let encoded = cursor
        .strip_prefix(&format!("{CURSOR_VERSION}."))
        .ok_or_else(|| {
            ApiError::validation_with_details(
                format!("cursor must start with `{CURSOR_VERSION}.`"),
                serde_json::json!({
                    "field": "cursor",
                    "expected_prefix": format!("{CURSOR_VERSION}."),
                }),
            )
        })?;

    let bytes = URL_SAFE_NO_PAD.decode(encoded).map_err(|_| {
        ApiError::validation_with_details(
            "cursor is not valid base64url",
            serde_json::json!({ "field": "cursor" }),
        )
    })?;

    serde_json::from_slice(&bytes).map_err(|_| {
        ApiError::validation_with_details(
            "cursor payload is invalid",
            serde_json::json!({ "field": "cursor" }),
        )
    })
}

pub fn decode_search_cursor(
    cursor: &str,
    expected_filters: &MessageFiltersSnapshot,
) -> Result<MessageSearchCursor, ApiError> {
    let decoded = decode::<MessageSearchCursor>(cursor)?;
    if decoded.filters != *expected_filters {
        return Err(ApiError::validation_with_details(
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
            ApiError::validation_with_details(
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
            ApiError::validation_with_details(
                "cursor is not valid base64url",
                serde_json::json!({ "field": "cursor" }),
            )
        })?;

    if serde_json::from_slice::<MessageSearchCursor>(&raw).is_ok() {
        return Err(ApiError::validation_with_details(
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
        return Err(ApiError::validation_with_details(
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
        return Err(ApiError::validation_with_details(
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
    fn round_trips_global_message_cursor() {
        let cursor = GlobalMessageCursor {
            date: 123,
            row_id: 456,
        };
        let encoded = encode(&cursor).expect("encode");
        assert!(encoded.starts_with("v1."));
        assert_eq!(
            decode::<GlobalMessageCursor>(&encoded).expect("decode"),
            cursor
        );
    }

    #[test]
    fn round_trips_chat_list_cursor() {
        let cursor = ChatListCursor {
            message_date: 100,
            message_id: 200,
            chat_id: 3,
        };
        let encoded = encode(&cursor).expect("encode");
        assert_eq!(decode::<ChatListCursor>(&encoded).expect("decode"), cursor);
    }
}
