//! Query, path, and header parameter types for the OpenAPI contract.

#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use super::{
    dto::pagination::{DEFAULT_PAGE_LIMIT, MAX_PAGE_LIMIT},
    error::{ApiError, ErrorCode},
};
use crate::apple_types::{
    AttachmentId, CalendarAttachmentId, CalendarId, ChatId, ContactId, ContainerId, EventId,
    GroupId, IdValidationError, MessageId, NoteAttachmentId, NoteFolderId, NoteId,
    ReminderAttachmentId, ReminderId, ReminderListId,
};

fn id_validation_error(error: IdValidationError) -> ApiError {
    ApiError::with_details(
        ErrorCode::InvalidParameter,
        error.message,
        serde_json::json!({ "field": error.kind }),
    )
}

fn invalid_limit(message: impl Into<String>, details: serde_json::Value) -> ApiError {
    ApiError::with_details(ErrorCode::InvalidLimit, message, details)
}

fn invalid_cursor(message: impl Into<String>, details: serde_json::Value) -> ApiError {
    ApiError::with_details(ErrorCode::InvalidCursor, message, details)
}

fn invalid_timestamp(message: impl Into<String>, details: serde_json::Value) -> ApiError {
    ApiError::with_details(ErrorCode::InvalidTimestamp, message, details)
}

fn invalid_parameter(message: impl Into<String>, details: serde_json::Value) -> ApiError {
    ApiError::with_details(ErrorCode::InvalidParameter, message, details)
}

pub const CURSOR_VERSION: &str = "v1";
pub const MAX_SEARCH_QUERY_LEN: usize = 256;

#[derive(Debug, Clone, Deserialize, IntoParams, ToSchema)]
#[into_params(parameter_in = Query, style = Form)]
pub struct PageParams {
    /// Maximum number of items to return. Defaults to 50. Keyset pagination only; no offsets.
    #[param(minimum = 1, maximum = 200, default = 50, example = 50)]
    pub limit: Option<u32>,

    /// URL-safe versioned cursor for the next page. Results are ordered newest first.
    #[param(example = "v1.eyJkYXRlIjoxNzA0MDk2MDAwfQ")]
    pub cursor: Option<String>,
}

impl PageParams {
    pub fn validated_limit(&self) -> Result<u32, ApiError> {
        let limit = self.limit.unwrap_or(DEFAULT_PAGE_LIMIT);
        if !(1..=MAX_PAGE_LIMIT).contains(&limit) {
            return Err(invalid_limit(
                format!("limit must be between 1 and {MAX_PAGE_LIMIT}"),
                serde_json::json!({
                    "field": "limit",
                    "minimum": 1,
                    "maximum": MAX_PAGE_LIMIT,
                    "default": DEFAULT_PAGE_LIMIT,
                }),
            ));
        }
        Ok(limit)
    }

    pub fn validated_cursor(&self) -> Result<Option<&str>, ApiError> {
        match &self.cursor {
            None => Ok(None),
            Some(cursor) if cursor.starts_with(&format!("{CURSOR_VERSION}.")) => Ok(Some(cursor)),
            Some(_) => Err(invalid_cursor(
                format!("cursor must start with `{CURSOR_VERSION}.`"),
                serde_json::json!({
                    "field": "cursor",
                    "expected_prefix": format!("{CURSOR_VERSION}."),
                }),
            )),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum DirectionFilterDto {
    Sent,
    Received,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum TransportFilterDto {
    Imessage,
    Sms,
    Rcs,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ContentTypeFilterDto {
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

impl From<DirectionFilterDto> for crate::messages::search::DirectionFilter {
    fn from(value: DirectionFilterDto) -> Self {
        match value {
            DirectionFilterDto::Sent => Self::Sent,
            DirectionFilterDto::Received => Self::Received,
        }
    }
}

impl From<TransportFilterDto> for crate::messages::search::TransportFilter {
    fn from(value: TransportFilterDto) -> Self {
        match value {
            TransportFilterDto::Imessage => Self::Imessage,
            TransportFilterDto::Sms => Self::Sms,
            TransportFilterDto::Rcs => Self::Rcs,
            TransportFilterDto::Unknown => Self::Unknown,
        }
    }
}

impl From<ContentTypeFilterDto> for crate::messages::search::ContentTypeFilter {
    fn from(value: ContentTypeFilterDto) -> Self {
        match value {
            ContentTypeFilterDto::Text => Self::Text,
            ContentTypeFilterDto::Audio => Self::Audio,
            ContentTypeFilterDto::Attachment => Self::Attachment,
            ContentTypeFilterDto::Reaction => Self::Reaction,
            ContentTypeFilterDto::GroupEvent => Self::GroupEvent,
            ContentTypeFilterDto::AppBalloon => Self::AppBalloon,
            ContentTypeFilterDto::SharePlay => Self::SharePlay,
            ContentTypeFilterDto::ShareMyLocation => Self::ShareMyLocation,
            ContentTypeFilterDto::System => Self::System,
            ContentTypeFilterDto::Unknown => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone, Deserialize, IntoParams, ToSchema)]
#[into_params(parameter_in = Query, style = Form)]
pub struct MessageListParams {
    /// Maximum number of items to return. Defaults to 50. Keyset pagination only; no offsets.
    #[param(minimum = 1, maximum = 200, default = 50, example = 50)]
    pub limit: Option<u32>,

    /// URL-safe versioned cursor for the next page. Results are ordered newest first.
    #[param(example = "v1.eyJkYXRlIjoxNzA0MDk2MDAwfQ")]
    pub cursor: Option<String>,

    /// Case-insensitive search over plain text and decoded attributed-body text.
    #[param(max_length = 256, example = "hello")]
    pub q: Option<String>,

    /// Restrict results to messages in this chat.
    #[param(example = 42)]
    pub chat_id: Option<ChatId>,

    /// Restrict results to messages from this handle identifier.
    #[param(example = "+15551234567")]
    pub sender: Option<String>,

    /// Return messages sent strictly before this Unix timestamp (UTC seconds).
    #[param(example = 1_705_320_000)]
    pub before: Option<i64>,

    /// Return messages sent strictly after this Unix timestamp (UTC seconds).
    #[param(example = 1_704_067_200)]
    pub after: Option<i64>,

    /// Restrict results to sent or received messages.
    pub direction: Option<DirectionFilterDto>,

    /// Restrict results to a transport/service.
    pub transport: Option<TransportFilterDto>,

    /// Restrict results to a coarse message content category.
    pub content_type: Option<ContentTypeFilterDto>,

    /// Restrict results to messages with or without attachments.
    pub has_attachments: Option<bool>,
}

impl MessageListParams {
    pub fn validated_limit(&self) -> Result<u32, ApiError> {
        let limit = self.limit.unwrap_or(DEFAULT_PAGE_LIMIT);
        if !(1..=MAX_PAGE_LIMIT).contains(&limit) {
            return Err(invalid_limit(
                format!("limit must be between 1 and {MAX_PAGE_LIMIT}"),
                serde_json::json!({
                    "field": "limit",
                    "minimum": 1,
                    "maximum": MAX_PAGE_LIMIT,
                    "default": DEFAULT_PAGE_LIMIT,
                }),
            ));
        }
        Ok(limit)
    }

    pub fn validated_cursor(&self) -> Result<Option<&str>, ApiError> {
        match &self.cursor {
            None => Ok(None),
            Some(cursor) if cursor.starts_with(&format!("{CURSOR_VERSION}.")) => Ok(Some(cursor)),
            Some(_) => Err(invalid_cursor(
                format!("cursor must start with `{CURSOR_VERSION}.`"),
                serde_json::json!({
                    "field": "cursor",
                    "expected_prefix": format!("{CURSOR_VERSION}."),
                }),
            )),
        }
    }

    pub fn validated_filters(&self) -> Result<crate::messages::search::MessageFilters, ApiError> {
        let q = match self.q.as_deref().map(str::trim) {
            None | Some("") => None,
            Some(query) if query.len() > MAX_SEARCH_QUERY_LEN => {
                return Err(invalid_parameter(
                    format!("q must be at most {MAX_SEARCH_QUERY_LEN} characters"),
                    serde_json::json!({
                        "field": "q",
                        "maximum": MAX_SEARCH_QUERY_LEN,
                    }),
                ));
            }
            Some(query) => Some(query.to_owned()),
        };

        let sender = match self.sender.as_deref().map(str::trim) {
            None | Some("") => None,
            Some(value) => Some(value.to_owned()),
        };

        let before = self.before.map(unix_secs_to_apple_nanos);
        let after = self.after.map(unix_secs_to_apple_nanos);

        if let (Some(before), Some(after)) = (before, after)
            && before <= after
        {
            return Err(invalid_timestamp(
                "before must be later than after",
                serde_json::json!({
                    "field": "before",
                    "related_field": "after",
                }),
            ));
        }

        let chat_id = match self.chat_id {
            None => None,
            Some(id) => Some(
                ChatId::try_new(id.get())
                    .map_err(id_validation_error)?
                    .get(),
            ),
        };

        Ok(crate::messages::search::MessageFilters {
            q,
            chat_id,
            sender,
            before,
            after,
            direction: self.direction.map(Into::into),
            transport: self.transport.map(Into::into),
            content_type: self.content_type.map(Into::into),
            has_attachments: self.has_attachments,
        })
    }
}

fn unix_secs_to_apple_nanos(unix_secs: i64) -> i64 {
    (unix_secs - 978_307_200) * 1_000_000_000
}

fn unix_secs_to_core_data_secs(unix_secs: i64) -> i64 {
    unix_secs - 978_307_200
}

fn unix_secs_to_core_data_timestamp(unix_secs: i64) -> f64 {
    unix_secs_to_core_data_secs(unix_secs) as f64
}

#[derive(Debug, Clone, Deserialize, IntoParams, ToSchema)]
#[into_params(parameter_in = Path)]
pub struct ChatIdPath {
    /// Internal chat row identifier.
    #[param(example = 42)]
    pub chat_id: ChatId,
}

impl ChatIdPath {
    pub fn validated(&self) -> Result<ChatId, ApiError> {
        ChatId::try_new(self.chat_id.get()).map_err(id_validation_error)
    }
}

#[derive(Debug, Clone, Deserialize, IntoParams, ToSchema)]
#[into_params(parameter_in = Path)]
pub struct MessageGuidPath {
    /// Message GUID.
    #[param(example = "A1B2C3D4-E5F6-7890-ABCD-EF1234567890")]
    pub guid: MessageId,
}

impl MessageGuidPath {
    pub fn validated(&self) -> Result<MessageId, ApiError> {
        MessageId::try_new(self.guid.as_str()).map_err(id_validation_error)
    }
}

#[derive(Debug, Clone, Deserialize, IntoParams, ToSchema)]
#[into_params(parameter_in = Path)]
pub struct AttachmentGuidPath {
    /// Attachment GUID.
    #[param(example = "at_0_1234567890ABCDEF")]
    pub guid: AttachmentId,
}

impl AttachmentGuidPath {
    pub fn validated(&self) -> Result<AttachmentId, ApiError> {
        AttachmentId::try_new(self.guid.as_str()).map_err(id_validation_error)
    }
}

#[derive(Debug, Clone, Deserialize, IntoParams, ToSchema)]
#[into_params(parameter_in = Header)]
pub struct ConditionalRequestHeaders {
    /// Validator for conditional GET/HEAD requests.
    #[param(rename = "If-None-Match", example = "\"abc123\"")]
    pub if_none_match: Option<String>,

    /// Timestamp validator for conditional GET/HEAD requests.
    #[param(
        rename = "If-Modified-Since",
        example = "Mon, 01 Jan 2024 12:00:00 GMT"
    )]
    pub if_modified_since: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::PageParams;

    #[test]
    fn default_limit_is_50_and_max_is_200() -> Result<(), Box<dyn std::error::Error>> {
        let page = PageParams {
            limit: None,
            cursor: None,
        };
        assert_eq!(page.validated_limit()?, 50);

        let invalid_high = PageParams {
            limit: Some(201),
            cursor: None,
        };
        assert!(invalid_high.validated_limit().is_err());

        let invalid_zero = PageParams {
            limit: Some(0),
            cursor: None,
        };
        assert!(invalid_zero.validated_limit().is_err());
        Ok(())
    }

    #[test]
    fn note_folder_key_accepts_row_id_and_identifier() -> Result<(), Box<dyn std::error::Error>> {
        use super::NoteFolderKey;

        assert!(matches!(
            NoteFolderKey::parse("42")?,
            NoteFolderKey::Row(42)
        ));
        assert!(matches!(
            NoteFolderKey::parse("DefaultFolder-CloudKit")?,
            NoteFolderKey::Id(id) if id == "DefaultFolder-CloudKit"
        ));
        assert!(NoteFolderKey::parse("").is_err());
        assert!(NoteFolderKey::parse("0").is_err());
        Ok(())
    }

    #[test]
    fn cursor_must_be_versioned_and_url_safe_prefix() -> Result<(), Box<dyn std::error::Error>> {
        let valid = PageParams {
            limit: None,
            cursor: Some("v1.c2afe".to_owned()),
        };
        assert_eq!(valid.validated_cursor()?, Some("v1.c2afe"));

        let invalid = PageParams {
            limit: None,
            cursor: Some("offset:10".to_owned()),
        };
        assert!(invalid.validated_cursor().is_err());
        Ok(())
    }

    #[test]
    fn search_query_length_is_bounded() {
        use super::{MAX_SEARCH_QUERY_LEN, MessageListParams};

        let params = MessageListParams {
            limit: None,
            cursor: None,
            q: Some("a".repeat(MAX_SEARCH_QUERY_LEN + 1)),
            chat_id: None,
            sender: None,
            before: None,
            after: None,
            direction: None,
            transport: None,
            content_type: None,
            has_attachments: None,
        };
        assert!(params.validated_filters().is_err());
    }

    #[test]
    fn before_must_be_after_after_timestamp() {
        use super::MessageListParams;

        let params = MessageListParams {
            limit: None,
            cursor: None,
            q: None,
            chat_id: None,
            sender: None,
            before: Some(1_704_067_200),
            after: Some(1_706_745_600),
            direction: None,
            transport: None,
            content_type: None,
            has_attachments: None,
        };
        assert!(params.validated_filters().is_err());
    }
}

#[derive(Debug, Clone, Deserialize, IntoParams, ToSchema)]
#[into_params(parameter_in = Header)]
pub struct RangeRequestHeader {
    /// Byte range for partial content requests.
    #[param(rename = "Range", example = "bytes=0-1023")]
    pub range: Option<String>,
}

#[derive(Debug, Clone)]
pub enum ReminderListKey {
    Row(i64),
    Id(String),
}

impl ReminderListKey {
    pub fn parse(raw: &str) -> Result<Self, ApiError> {
        if let Ok(row_id) = raw.parse::<i64>() {
            if row_id <= 0 {
                return Err(invalid_parameter(
                    "list_id must be a positive integer or UUID",
                    serde_json::json!({ "field": "list_id" }),
                ));
            }
            return Ok(Self::Row(row_id));
        }
        if is_uuid(raw) {
            return Ok(Self::Id(raw.to_lowercase()));
        }
        Err(invalid_parameter(
            "list_id must be a positive integer or UUID",
            serde_json::json!({ "field": "list_id" }),
        ))
    }
}

#[derive(Debug, Clone, Deserialize, IntoParams, ToSchema)]
#[into_params(parameter_in = Path)]
pub struct ReminderListIdPath {
    pub list_id: ReminderListId,
}

impl ReminderListIdPath {
    pub fn validated_key(&self) -> Result<ReminderListKey, ApiError> {
        ReminderListKey::parse(self.list_id.as_str())
    }
}

#[derive(Debug, Clone, Deserialize, IntoParams, ToSchema)]
#[into_params(parameter_in = Path)]
pub struct ReminderIdPath {
    pub reminder_id: ReminderId,
}

impl ReminderIdPath {
    pub fn validated(&self) -> Result<ReminderId, ApiError> {
        ReminderId::try_new(self.reminder_id.as_str()).map_err(id_validation_error)
    }
}

#[derive(Debug, Clone, Deserialize, IntoParams, ToSchema)]
#[into_params(parameter_in = Path)]
pub struct ReminderAttachmentIdPath {
    pub id: ReminderAttachmentId,
}

impl ReminderAttachmentIdPath {
    pub fn validated(&self) -> Result<ReminderAttachmentId, ApiError> {
        ReminderAttachmentId::try_new(self.id.as_str()).map_err(id_validation_error)
    }
}

#[derive(Debug, Clone, Deserialize, IntoParams, ToSchema)]
#[into_params(parameter_in = Query, style = Form)]
pub struct ReminderListParams {
    #[param(minimum = 1, maximum = 200, default = 50, example = 50)]
    pub limit: Option<u32>,
    #[param(example = "v1.eyJyb3dfaWQiOjF9")]
    pub cursor: Option<String>,
    pub completed: Option<bool>,
    pub flagged: Option<bool>,
    pub has_due_date: Option<bool>,
    #[param(example = 1_737_072_000)]
    pub due_before: Option<i64>,
    #[param(example = 1_735_689_600)]
    pub due_after: Option<i64>,
    pub priority_min: Option<i32>,
    pub has_notes: Option<bool>,
    pub top_level_only: Option<bool>,
    pub include_subtasks: Option<bool>,
    pub include_tags: Option<bool>,
    #[param(max_length = 256, example = "groceries")]
    pub q: Option<String>,
    pub list_id: Option<String>,
}

impl ReminderListParams {
    pub fn validated_limit(&self) -> Result<u32, ApiError> {
        let limit = self.limit.unwrap_or(DEFAULT_PAGE_LIMIT);
        if !(1..=MAX_PAGE_LIMIT).contains(&limit) {
            return Err(invalid_limit(
                format!("limit must be between 1 and {MAX_PAGE_LIMIT}"),
                serde_json::json!({
                    "field": "limit",
                    "minimum": 1,
                    "maximum": MAX_PAGE_LIMIT,
                    "default": DEFAULT_PAGE_LIMIT,
                }),
            ));
        }
        Ok(limit)
    }

    pub fn validated_cursor(&self) -> Result<Option<&str>, ApiError> {
        match &self.cursor {
            None => Ok(None),
            Some(cursor) if cursor.starts_with(&format!("{CURSOR_VERSION}.")) => Ok(Some(cursor)),
            Some(_) => Err(invalid_cursor(
                format!("cursor must start with `{CURSOR_VERSION}.`"),
                serde_json::json!({
                    "field": "cursor",
                    "expected_prefix": format!("{CURSOR_VERSION}."),
                }),
            )),
        }
    }

    pub fn validated_filters(&self) -> Result<crate::reminders::ReminderFilters, ApiError> {
        let q = match self.q.as_deref().map(str::trim) {
            None | Some("") => None,
            Some(query) if query.len() > MAX_SEARCH_QUERY_LEN => {
                return Err(invalid_parameter(
                    format!("q must be at most {MAX_SEARCH_QUERY_LEN} characters"),
                    serde_json::json!({
                        "field": "q",
                        "maximum": MAX_SEARCH_QUERY_LEN,
                    }),
                ));
            }
            Some(query) => Some(query.to_owned()),
        };

        let list_id = match self.list_id.as_deref().map(str::trim) {
            None | Some("") => None,
            Some(value) => match value.parse::<i64>() {
                Ok(row_id) if row_id > 0 => Some(crate::reminders::ListIdFilter::RowId(row_id)),
                Ok(_) => {
                    return Err(invalid_parameter(
                        "list_id must be a positive integer or UUID",
                        serde_json::json!({ "field": "list_id" }),
                    ));
                }
                Err(_) if is_uuid(value) => {
                    Some(crate::reminders::ListIdFilter::Uuid(value.to_lowercase()))
                }
                Err(_) => {
                    return Err(invalid_parameter(
                        "list_id must be a positive integer or UUID",
                        serde_json::json!({ "field": "list_id" }),
                    ));
                }
            },
        };

        let due_before = self.due_before.map(unix_secs_to_core_data_secs);
        let due_after = self.due_after.map(unix_secs_to_core_data_secs);

        Ok(crate::reminders::ReminderFilters {
            q,
            completed: self.completed,
            flagged: self.flagged,
            list_id,
            has_due_date: self.has_due_date,
            due_before,
            due_after,
            priority_min: self.priority_min,
            has_notes: self.has_notes,
            top_level_only: self.top_level_only,
        })
    }
}

fn is_uuid(value: &str) -> bool {
    value.len() == 36 && value.chars().all(|ch| ch.is_ascii_hexdigit() || ch == '-')
}

#[derive(Debug, Clone)]
pub enum NoteFolderKey {
    Row(i64),
    Id(String),
}

impl NoteFolderKey {
    pub fn parse(raw: &str) -> Result<Self, ApiError> {
        let raw = raw.trim();
        if raw.is_empty() {
            return Err(invalid_parameter(
                "folder_id must be a positive integer or Notes folder identifier",
                serde_json::json!({ "field": "folder_id" }),
            ));
        }
        if let Ok(row_id) = raw.parse::<i64>() {
            if row_id <= 0 {
                return Err(invalid_parameter(
                    "folder_id must be a positive integer or Notes folder identifier",
                    serde_json::json!({ "field": "folder_id" }),
                ));
            }
            return Ok(Self::Row(row_id));
        }
        Ok(Self::Id(raw.to_owned()))
    }
}

#[derive(Debug, Clone, Deserialize, IntoParams, ToSchema)]
#[into_params(parameter_in = Path)]
pub struct NoteFolderIdPath {
    pub folder_id: NoteFolderId,
}

impl NoteFolderIdPath {
    pub fn validated_key(&self) -> Result<NoteFolderKey, ApiError> {
        NoteFolderKey::parse(self.folder_id.as_str())
    }
}

#[derive(Debug, Clone, Deserialize, IntoParams, ToSchema)]
#[into_params(parameter_in = Path)]
pub struct NoteIdPath {
    pub note_id: NoteId,
}

impl NoteIdPath {
    pub fn validated(&self) -> Result<NoteId, ApiError> {
        NoteId::try_new(self.note_id.as_str()).map_err(id_validation_error)
    }
}

#[derive(Debug, Clone, Deserialize, IntoParams, ToSchema)]
#[into_params(parameter_in = Path)]
pub struct NoteAttachmentIdPath {
    pub id: NoteAttachmentId,
}

impl NoteAttachmentIdPath {
    pub fn validated(&self) -> Result<NoteAttachmentId, ApiError> {
        NoteAttachmentId::try_new(self.id.as_str()).map_err(id_validation_error)
    }
}

#[derive(Debug, Clone, Deserialize, IntoParams, ToSchema)]
#[into_params(parameter_in = Query, style = Form)]
pub struct NoteListParams {
    #[param(minimum = 1, maximum = 200, default = 50, example = 50)]
    pub limit: Option<u32>,
    #[param(example = "v1.eyJtb2RpZmllZF9hdCI6MTAwfQ")]
    pub cursor: Option<String>,
    #[param(max_length = 256, example = "groceries")]
    pub q: Option<String>,
    pub folder_id: Option<String>,
    pub is_pinned: Option<bool>,
    pub is_locked: Option<bool>,
    pub has_checklist: Option<bool>,
    pub has_attachments: Option<bool>,
    pub include_deleted: Option<bool>,
    #[param(example = 1_737_072_000)]
    pub modified_before: Option<i64>,
    #[param(example = 1_735_689_600)]
    pub modified_after: Option<i64>,
}

impl NoteListParams {
    pub fn validated_limit(&self) -> Result<u32, ApiError> {
        let limit = self.limit.unwrap_or(DEFAULT_PAGE_LIMIT);
        if !(1..=MAX_PAGE_LIMIT).contains(&limit) {
            return Err(invalid_limit(
                format!("limit must be between 1 and {MAX_PAGE_LIMIT}"),
                serde_json::json!({
                    "field": "limit",
                    "minimum": 1,
                    "maximum": MAX_PAGE_LIMIT,
                    "default": DEFAULT_PAGE_LIMIT,
                }),
            ));
        }
        Ok(limit)
    }

    pub fn validated_cursor(&self) -> Result<Option<&str>, ApiError> {
        match &self.cursor {
            None => Ok(None),
            Some(cursor) if cursor.starts_with(&format!("{CURSOR_VERSION}.")) => Ok(Some(cursor)),
            Some(_) => Err(invalid_cursor(
                format!("cursor must start with `{CURSOR_VERSION}.`"),
                serde_json::json!({
                    "field": "cursor",
                    "expected_prefix": format!("{CURSOR_VERSION}."),
                }),
            )),
        }
    }

    pub fn validated_filters(&self) -> Result<crate::notes::NoteFilters, ApiError> {
        let q = match self.q.as_deref().map(str::trim) {
            None | Some("") => None,
            Some(query) if query.len() > MAX_SEARCH_QUERY_LEN => {
                return Err(invalid_parameter(
                    format!("q must be at most {MAX_SEARCH_QUERY_LEN} characters"),
                    serde_json::json!({
                        "field": "q",
                        "maximum": MAX_SEARCH_QUERY_LEN,
                    }),
                ));
            }
            Some(query) => Some(query.to_owned()),
        };

        let folder_id = match self.folder_id.as_deref().map(str::trim) {
            None | Some("") => None,
            Some(value) => match value.parse::<i64>() {
                Ok(row_id) if row_id > 0 => Some(crate::notes::FolderIdFilter::RowId(row_id)),
                Ok(_) => {
                    return Err(invalid_parameter(
                        "folder_id must be a positive integer or Notes folder identifier",
                        serde_json::json!({ "field": "folder_id" }),
                    ));
                }
                Err(_) => Some(crate::notes::FolderIdFilter::Identifier(value.to_owned())),
            },
        };

        let modified_before = self.modified_before.map(unix_secs_to_core_data_timestamp);
        let modified_after = self.modified_after.map(unix_secs_to_core_data_timestamp);

        if let (Some(before), Some(after)) = (modified_before, modified_after)
            && before <= after
        {
            return Err(invalid_timestamp(
                "modified_before must be later than modified_after",
                serde_json::json!({
                    "field": "modified_before",
                    "related_field": "modified_after",
                }),
            ));
        }

        Ok(crate::notes::NoteFilters {
            q,
            folder_id,
            is_pinned: self.is_pinned,
            is_locked: self.is_locked,
            has_checklist: self.has_checklist,
            has_attachments: self.has_attachments,
            include_deleted: self.include_deleted,
            modified_before,
            modified_after,
        })
    }
}

#[derive(Debug, Clone, Deserialize, IntoParams, ToSchema)]
#[into_params(parameter_in = Query, style = Form)]
pub struct EventListParams {
    #[param(minimum = 1, maximum = 200, default = 50, example = 50)]
    pub limit: Option<u32>,
    #[param(example = "v1.eyJyb3dfaWQiOjF9")]
    pub cursor: Option<String>,
    #[param(max_length = 256, example = "standup")]
    pub q: Option<String>,
    pub calendar_id: Option<String>,
    pub account_id: Option<String>,
    #[param(example = 1704067200)]
    pub start: Option<i64>,
    #[param(example = 1735689600)]
    pub end: Option<i64>,
    pub include_hidden: Option<bool>,
    pub include_cancelled: Option<bool>,
}

impl EventListParams {
    pub fn validated_limit(&self) -> Result<u32, ApiError> {
        let limit = self.limit.unwrap_or(DEFAULT_PAGE_LIMIT);
        if !(1..=MAX_PAGE_LIMIT).contains(&limit) {
            return Err(invalid_limit(
                format!("limit must be between 1 and {MAX_PAGE_LIMIT}"),
                serde_json::json!({
                    "field": "limit",
                    "minimum": 1,
                    "maximum": MAX_PAGE_LIMIT,
                    "default": DEFAULT_PAGE_LIMIT,
                }),
            ));
        }
        Ok(limit)
    }

    pub fn validated_cursor(&self) -> Result<Option<&str>, ApiError> {
        match &self.cursor {
            None => Ok(None),
            Some(cursor) if cursor.starts_with(&format!("{CURSOR_VERSION}.")) => Ok(Some(cursor)),
            Some(_) => Err(invalid_cursor(
                format!("cursor must start with `{CURSOR_VERSION}.`"),
                serde_json::json!({
                    "field": "cursor",
                    "expected_prefix": format!("{CURSOR_VERSION}."),
                }),
            )),
        }
    }

    pub fn validated_filters(&self) -> Result<crate::calendar::EventFilters, ApiError> {
        let q = match self.q.as_deref().map(str::trim) {
            None | Some("") => None,
            Some(query) if query.len() > MAX_SEARCH_QUERY_LEN => {
                return Err(invalid_parameter(
                    format!("q must be at most {MAX_SEARCH_QUERY_LEN} characters"),
                    serde_json::json!({
                        "field": "q",
                        "maximum": MAX_SEARCH_QUERY_LEN,
                    }),
                ));
            }
            Some(query) => Some(query.to_owned()),
        };

        if let (Some(start), Some(end)) = (self.start, self.end)
            && start > end
        {
            return Err(invalid_timestamp(
                "start must be before or equal to end",
                serde_json::json!({
                    "field": "start",
                    "related_field": "end",
                }),
            ));
        }

        Ok(crate::calendar::EventFilters {
            q,
            calendar_id: self.calendar_id.clone(),
            account_id: self.account_id.clone(),
            start_after: self.start.map(crate::calendar::unix_to_core_data_secs),
            start_before: self.end.map(crate::calendar::unix_to_core_data_secs),
            include_hidden: self.include_hidden.unwrap_or(false),
            include_cancelled: self.include_cancelled.unwrap_or(false),
        })
    }
}

#[derive(Debug, Clone, Deserialize, IntoParams, ToSchema)]
#[into_params(parameter_in = Path)]
pub struct CalendarIdPath {
    pub calendar_id: CalendarId,
}

impl CalendarIdPath {
    pub fn validated(&self) -> Result<CalendarId, ApiError> {
        CalendarId::try_new(self.calendar_id.as_str()).map_err(id_validation_error)
    }
}

#[derive(Debug, Clone, Deserialize, IntoParams, ToSchema)]
#[into_params(parameter_in = Path)]
pub struct EventIdPath {
    pub event_id: EventId,
}

impl EventIdPath {
    pub fn validated(&self) -> Result<EventId, ApiError> {
        EventId::try_new(self.event_id.as_str()).map_err(id_validation_error)
    }
}

#[derive(Debug, Clone, Deserialize, IntoParams, ToSchema)]
#[into_params(parameter_in = Path)]
pub struct EventAttachmentIdPath {
    pub event_id: EventId,
    pub attachment_id: CalendarAttachmentId,
}

impl EventAttachmentIdPath {
    pub fn validated(&self) -> Result<(EventId, CalendarAttachmentId), ApiError> {
        Ok((
            EventId::try_new(self.event_id.as_str()).map_err(id_validation_error)?,
            CalendarAttachmentId::try_new(self.attachment_id.as_str())
                .map_err(id_validation_error)?,
        ))
    }
}

#[derive(Debug, Clone, Deserialize, IntoParams, ToSchema)]
#[into_params(parameter_in = Path)]
pub struct ContainerIdPath {
    pub container_id: ContainerId,
}

impl ContainerIdPath {
    pub fn validated(&self) -> Result<ContainerId, ApiError> {
        ContainerId::try_new(self.container_id.as_str()).map_err(id_validation_error)
    }
}

#[derive(Debug, Clone, Deserialize, IntoParams, ToSchema)]
#[into_params(parameter_in = Path)]
pub struct GroupIdPath {
    pub group_id: GroupId,
}

impl GroupIdPath {
    pub fn validated(&self) -> Result<GroupId, ApiError> {
        GroupId::try_new(self.group_id.as_str()).map_err(id_validation_error)
    }
}

#[derive(Debug, Clone, Deserialize, IntoParams, ToSchema)]
#[into_params(parameter_in = Path)]
pub struct ContactIdPath {
    pub contact_id: ContactId,
}

impl ContactIdPath {
    pub fn validated(&self) -> Result<ContactId, ApiError> {
        ContactId::try_new(self.contact_id.as_str()).map_err(id_validation_error)
    }
}

#[derive(Debug, Clone, Deserialize, IntoParams, ToSchema)]
#[into_params(parameter_in = Path)]
pub struct ContactGroupPath {
    pub group_id: GroupId,
    pub contact_id: ContactId,
}

impl ContactGroupPath {
    pub fn validated(&self) -> Result<(GroupId, ContactId), ApiError> {
        Ok((
            GroupId::try_new(self.group_id.as_str()).map_err(id_validation_error)?,
            ContactId::try_new(self.contact_id.as_str()).map_err(id_validation_error)?,
        ))
    }
}

#[derive(Debug, Clone, Deserialize, IntoParams, ToSchema)]
#[into_params(parameter_in = Query, style = Form)]
pub struct ContactListParams {
    #[param(minimum = 1, maximum = 200, default = 50, example = 50)]
    pub limit: Option<u32>,
    #[param(example = "v1.eyJyb3dfaWQiOjF9")]
    pub cursor: Option<String>,
    #[param(max_length = 256, example = "jane")]
    pub q: Option<String>,
    pub container_id: Option<String>,
    pub group_id: Option<String>,
}

impl ContactListParams {
    pub fn validated_limit(&self) -> Result<u32, ApiError> {
        let limit = self.limit.unwrap_or(DEFAULT_PAGE_LIMIT);
        if !(1..=MAX_PAGE_LIMIT).contains(&limit) {
            return Err(invalid_limit(
                format!("limit must be between 1 and {MAX_PAGE_LIMIT}"),
                serde_json::json!({
                    "field": "limit",
                    "minimum": 1,
                    "maximum": MAX_PAGE_LIMIT,
                    "default": DEFAULT_PAGE_LIMIT,
                }),
            ));
        }
        Ok(limit)
    }

    pub fn validated_cursor(&self) -> Result<Option<&str>, ApiError> {
        match &self.cursor {
            None => Ok(None),
            Some(cursor) if cursor.starts_with(&format!("{CURSOR_VERSION}.")) => Ok(Some(cursor)),
            Some(_) => Err(invalid_cursor(
                format!("cursor must start with `{CURSOR_VERSION}.`"),
                serde_json::json!({
                    "field": "cursor",
                    "expected_prefix": format!("{CURSOR_VERSION}."),
                }),
            )),
        }
    }

    pub fn validated_search_query(&self) -> Result<String, ApiError> {
        match self.q.as_deref().map(str::trim) {
            None | Some("") => Err(invalid_parameter(
                "q is required",
                serde_json::json!({ "field": "q" }),
            )),
            Some(query) if query.len() > MAX_SEARCH_QUERY_LEN => Err(invalid_parameter(
                format!("q must be at most {MAX_SEARCH_QUERY_LEN} characters"),
                serde_json::json!({
                    "field": "q",
                    "maximum": MAX_SEARCH_QUERY_LEN,
                }),
            )),
            Some(query) => Ok(query.to_owned()),
        }
    }

    pub fn validated_filters(&self) -> Result<crate::contacts::ContactFilters, ApiError> {
        let q = match self.q.as_deref().map(str::trim) {
            None | Some("") => None,
            Some(query) if query.len() > MAX_SEARCH_QUERY_LEN => {
                return Err(invalid_parameter(
                    format!("q must be at most {MAX_SEARCH_QUERY_LEN} characters"),
                    serde_json::json!({
                        "field": "q",
                        "maximum": MAX_SEARCH_QUERY_LEN,
                    }),
                ));
            }
            Some(query) => Some(query.to_owned()),
        };

        Ok(crate::contacts::ContactFilters {
            q,
            container_id: self.container_id.clone(),
            group_id: self.group_id.clone(),
        })
    }
}
