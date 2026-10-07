//! Granular API error codes. HTTP status is derived from the code.

use axum::http::StatusCode;
use serde::Serialize;
use utoipa::ToSchema;

/// Stable, unique snake_case error identifiers returned in `error.code`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    // Common / infrastructure
    RouteNotFound,
    MethodNotAllowed,
    InvalidCursor,
    InvalidLimit,
    InvalidTimestamp,
    InvalidParameter,
    UnsupportedQueryParameter,
    MalformedRequestBody,
    InvalidRequestBody,
    UnsupportedMediaType,
    RequestTimeout,
    QueryTimeout,
    InternalError,
    ByteRangeNotSatisfiable,

    // Database availability
    MessagesDatabaseUnavailable,
    RemindersDatabaseUnavailable,
    NotesDatabaseUnavailable,
    CalendarDatabaseUnavailable,
    ContactsDatabaseUnavailable,

    // Messages
    MessageNotFound,
    ChatNotFound,
    MessageAttachmentNotFound,
    MessageAttachmentUnavailable,

    // Notes
    NoteNotFound,
    NoteFolderNotFound,
    NoteAttachmentNotFound,
    NoteAttachmentUnavailable,

    // Reminders
    ReminderNotFound,
    ReminderListNotFound,
    ReminderAttachmentNotFound,
    ReminderAttachmentUnavailable,
    SmartListReadOnly,
    UnsupportedReminderField,
    InvalidReminderPriority,

    // Calendar / events
    CalendarNotFound,
    CalendarAccountNotFound,
    EventNotFound,
    EventAttachmentNotFound,
    EventAttachmentUnavailable,
    EventEndBeforeStart,
    ImmutableEventField,
    UnsupportedAlarmKind,
    AmbiguousEventKitMatch,

    // Contacts
    ContactNotFound,
    GroupNotFound,
    ContainerNotFound,
    ContactPhotoNotFound,
    ReadOnlyContainer,
    AmbiguousContactsMatch,

    // Permissions / frameworks
    EventkitAccessDenied,
    ContactsAccessDenied,
    EventkitUnavailable,
    ContactsUnavailable,
    CalendarReadOnly,
    EventkitInvalidInput,
    ContactsInvalidInput,
    EventkitTimeout,
    ContactsTimeout,

    // Sync
    SqliteSyncPending,
}

impl ErrorCode {
    pub fn http_status(self) -> StatusCode {
        match self {
            Self::RouteNotFound
            | Self::MessageNotFound
            | Self::ChatNotFound
            | Self::MessageAttachmentNotFound
            | Self::NoteNotFound
            | Self::NoteFolderNotFound
            | Self::NoteAttachmentNotFound
            | Self::ReminderNotFound
            | Self::ReminderListNotFound
            | Self::ReminderAttachmentNotFound
            | Self::CalendarNotFound
            | Self::CalendarAccountNotFound
            | Self::EventNotFound
            | Self::EventAttachmentNotFound
            | Self::ContactNotFound
            | Self::GroupNotFound
            | Self::ContainerNotFound
            | Self::ContactPhotoNotFound => StatusCode::NOT_FOUND,

            Self::MethodNotAllowed => StatusCode::METHOD_NOT_ALLOWED,

            Self::UnsupportedMediaType => StatusCode::UNSUPPORTED_MEDIA_TYPE,

            Self::InvalidCursor
            | Self::InvalidLimit
            | Self::InvalidTimestamp
            | Self::InvalidParameter
            | Self::UnsupportedQueryParameter
            | Self::MalformedRequestBody => StatusCode::BAD_REQUEST,

            Self::UnsupportedReminderField
            | Self::EventEndBeforeStart
            | Self::ImmutableEventField
            | Self::UnsupportedAlarmKind
            | Self::InvalidRequestBody
            | Self::InvalidReminderPriority
            | Self::EventkitInvalidInput
            | Self::ContactsInvalidInput => StatusCode::UNPROCESSABLE_ENTITY,

            Self::ByteRangeNotSatisfiable => StatusCode::RANGE_NOT_SATISFIABLE,

            Self::MessageAttachmentUnavailable
            | Self::NoteAttachmentUnavailable
            | Self::ReminderAttachmentUnavailable
            | Self::EventAttachmentUnavailable => StatusCode::NOT_FOUND,

            Self::SmartListReadOnly
            | Self::ReadOnlyContainer
            | Self::CalendarReadOnly
            | Self::EventkitAccessDenied
            | Self::ContactsAccessDenied => StatusCode::FORBIDDEN,

            Self::AmbiguousEventKitMatch | Self::AmbiguousContactsMatch => StatusCode::CONFLICT,

            Self::MessagesDatabaseUnavailable
            | Self::RemindersDatabaseUnavailable
            | Self::NotesDatabaseUnavailable
            | Self::CalendarDatabaseUnavailable
            | Self::ContactsDatabaseUnavailable
            | Self::EventkitUnavailable
            | Self::ContactsUnavailable
            | Self::SqliteSyncPending => StatusCode::SERVICE_UNAVAILABLE,

            Self::RequestTimeout
            | Self::QueryTimeout
            | Self::EventkitTimeout
            | Self::ContactsTimeout => StatusCode::GATEWAY_TIMEOUT,

            Self::InternalError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub fn default_message(self) -> &'static str {
        match self {
            Self::RouteNotFound => "route not found",
            Self::MethodNotAllowed => "method not allowed",
            Self::InvalidCursor => "invalid cursor",
            Self::InvalidLimit => "invalid limit",
            Self::InvalidTimestamp => "invalid timestamp",
            Self::InvalidParameter => "invalid parameter",
            Self::UnsupportedQueryParameter => "unsupported query parameter",
            Self::RequestTimeout => "request timed out",
            Self::QueryTimeout => "database query timed out",
            Self::InternalError => "internal server error",
            Self::ByteRangeNotSatisfiable => "requested byte range is not satisfiable",
            Self::MessagesDatabaseUnavailable => "Messages database is unavailable",
            Self::RemindersDatabaseUnavailable => "Reminders database is unavailable",
            Self::NotesDatabaseUnavailable => "Notes database is unavailable",
            Self::CalendarDatabaseUnavailable => "Calendar database is unavailable",
            Self::ContactsDatabaseUnavailable => "Contacts databases are unavailable",
            Self::MessageNotFound => "message not found",
            Self::ChatNotFound => "chat not found",
            Self::MessageAttachmentNotFound => "message attachment not found",
            Self::MessageAttachmentUnavailable => "message attachment is not available",
            Self::NoteNotFound => "note not found",
            Self::NoteFolderNotFound => "note folder not found",
            Self::NoteAttachmentNotFound => "note attachment not found",
            Self::NoteAttachmentUnavailable => "note attachment is not available",
            Self::ReminderNotFound => "reminder not found",
            Self::ReminderListNotFound => "reminder list not found",
            Self::ReminderAttachmentNotFound => "reminder attachment not found",
            Self::ReminderAttachmentUnavailable => "reminder attachment is not available",
            Self::SmartListReadOnly => "cannot write to smart reminder lists",
            Self::UnsupportedReminderField => "unsupported reminder field",
            Self::CalendarNotFound => "calendar not found",
            Self::CalendarAccountNotFound => "calendar account not found",
            Self::EventNotFound => "event not found",
            Self::EventAttachmentNotFound => "event attachment not found",
            Self::EventAttachmentUnavailable => "event attachment is not available",
            Self::EventEndBeforeStart => "end must be greater than or equal to start",
            Self::ImmutableEventField => "event field cannot be written through EventKit",
            Self::UnsupportedAlarmKind => "unsupported alarm kind",
            Self::AmbiguousEventKitMatch => "ambiguous EventKit match",
            Self::AmbiguousContactsMatch => "ambiguous Contacts match",
            Self::ContactNotFound => "contact not found",
            Self::GroupNotFound => "group not found",
            Self::ContainerNotFound => "container not found",
            Self::ContactPhotoNotFound => "contact photo not found",
            Self::ReadOnlyContainer => "target container is read-only",
            Self::EventkitAccessDenied => "EventKit access denied",
            Self::ContactsAccessDenied => "Contacts access denied",
            Self::EventkitUnavailable => {
                "EventKit is unavailable on this platform or could not be initialized"
            }
            Self::ContactsUnavailable => {
                "Contacts framework is unavailable on this platform or could not be initialized"
            }
            Self::CalendarReadOnly => "target calendar or list is read-only",
            Self::SqliteSyncPending => "write succeeded but SQLite read path has not caught up yet",
            Self::MalformedRequestBody => "request body is not valid JSON",
            Self::InvalidRequestBody => "request body does not match the schema",
            Self::UnsupportedMediaType => "request body must be application/json",
            Self::InvalidReminderPriority => "reminder priority must be between 0 and 9",
            Self::EventkitInvalidInput => "EventKit rejected the input",
            Self::ContactsInvalidInput => "Contacts rejected the input",
            Self::EventkitTimeout => "EventKit operation timed out",
            Self::ContactsTimeout => "Contacts operation timed out",
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::RouteNotFound => "route_not_found",
            Self::MethodNotAllowed => "method_not_allowed",
            Self::InvalidCursor => "invalid_cursor",
            Self::InvalidLimit => "invalid_limit",
            Self::InvalidTimestamp => "invalid_timestamp",
            Self::InvalidParameter => "invalid_parameter",
            Self::UnsupportedQueryParameter => "unsupported_query_parameter",
            Self::RequestTimeout => "request_timeout",
            Self::QueryTimeout => "query_timeout",
            Self::InternalError => "internal_error",
            Self::ByteRangeNotSatisfiable => "byte_range_not_satisfiable",
            Self::MessagesDatabaseUnavailable => "messages_database_unavailable",
            Self::RemindersDatabaseUnavailable => "reminders_database_unavailable",
            Self::NotesDatabaseUnavailable => "notes_database_unavailable",
            Self::CalendarDatabaseUnavailable => "calendar_database_unavailable",
            Self::ContactsDatabaseUnavailable => "contacts_database_unavailable",
            Self::MessageNotFound => "message_not_found",
            Self::ChatNotFound => "chat_not_found",
            Self::MessageAttachmentNotFound => "message_attachment_not_found",
            Self::MessageAttachmentUnavailable => "message_attachment_unavailable",
            Self::NoteNotFound => "note_not_found",
            Self::NoteFolderNotFound => "note_folder_not_found",
            Self::NoteAttachmentNotFound => "note_attachment_not_found",
            Self::NoteAttachmentUnavailable => "note_attachment_unavailable",
            Self::ReminderNotFound => "reminder_not_found",
            Self::ReminderListNotFound => "reminder_list_not_found",
            Self::ReminderAttachmentNotFound => "reminder_attachment_not_found",
            Self::ReminderAttachmentUnavailable => "reminder_attachment_unavailable",
            Self::SmartListReadOnly => "smart_list_read_only",
            Self::UnsupportedReminderField => "unsupported_reminder_field",
            Self::CalendarNotFound => "calendar_not_found",
            Self::CalendarAccountNotFound => "calendar_account_not_found",
            Self::EventNotFound => "event_not_found",
            Self::EventAttachmentNotFound => "event_attachment_not_found",
            Self::EventAttachmentUnavailable => "event_attachment_unavailable",
            Self::EventEndBeforeStart => "event_end_before_start",
            Self::ImmutableEventField => "immutable_event_field",
            Self::UnsupportedAlarmKind => "unsupported_alarm_kind",
            Self::AmbiguousEventKitMatch => "ambiguous_event_kit_match",
            Self::AmbiguousContactsMatch => "ambiguous_contacts_match",
            Self::ContactNotFound => "contact_not_found",
            Self::GroupNotFound => "group_not_found",
            Self::ContainerNotFound => "container_not_found",
            Self::ContactPhotoNotFound => "contact_photo_not_found",
            Self::ReadOnlyContainer => "read_only_container",
            Self::EventkitAccessDenied => "eventkit_access_denied",
            Self::ContactsAccessDenied => "contacts_access_denied",
            Self::EventkitUnavailable => "eventkit_unavailable",
            Self::ContactsUnavailable => "contacts_unavailable",
            Self::CalendarReadOnly => "calendar_read_only",
            Self::SqliteSyncPending => "sqlite_sync_pending",
            Self::MalformedRequestBody => "malformed_request_body",
            Self::InvalidRequestBody => "invalid_request_body",
            Self::UnsupportedMediaType => "unsupported_media_type",
            Self::InvalidReminderPriority => "invalid_reminder_priority",
            Self::EventkitInvalidInput => "eventkit_invalid_input",
            Self::ContactsInvalidInput => "contacts_invalid_input",
            Self::EventkitTimeout => "eventkit_timeout",
            Self::ContactsTimeout => "contacts_timeout",
        }
    }

    /// Every registered error code (for OpenAPI contract tests and docs).
    pub fn all() -> &'static [Self] {
        &Self::ALL
    }

    const ALL: [Self; 59] = [
        Self::RouteNotFound,
        Self::MethodNotAllowed,
        Self::InvalidCursor,
        Self::InvalidLimit,
        Self::InvalidTimestamp,
        Self::InvalidParameter,
        Self::UnsupportedQueryParameter,
        Self::RequestTimeout,
        Self::QueryTimeout,
        Self::InternalError,
        Self::ByteRangeNotSatisfiable,
        Self::MessagesDatabaseUnavailable,
        Self::RemindersDatabaseUnavailable,
        Self::NotesDatabaseUnavailable,
        Self::CalendarDatabaseUnavailable,
        Self::ContactsDatabaseUnavailable,
        Self::MessageNotFound,
        Self::ChatNotFound,
        Self::MessageAttachmentNotFound,
        Self::MessageAttachmentUnavailable,
        Self::NoteNotFound,
        Self::NoteFolderNotFound,
        Self::NoteAttachmentNotFound,
        Self::NoteAttachmentUnavailable,
        Self::ReminderNotFound,
        Self::ReminderListNotFound,
        Self::ReminderAttachmentNotFound,
        Self::ReminderAttachmentUnavailable,
        Self::SmartListReadOnly,
        Self::UnsupportedReminderField,
        Self::CalendarNotFound,
        Self::CalendarAccountNotFound,
        Self::EventNotFound,
        Self::EventAttachmentNotFound,
        Self::EventAttachmentUnavailable,
        Self::EventEndBeforeStart,
        Self::ImmutableEventField,
        Self::UnsupportedAlarmKind,
        Self::AmbiguousEventKitMatch,
        Self::ContactNotFound,
        Self::GroupNotFound,
        Self::ContainerNotFound,
        Self::ContactPhotoNotFound,
        Self::ReadOnlyContainer,
        Self::AmbiguousContactsMatch,
        Self::EventkitAccessDenied,
        Self::ContactsAccessDenied,
        Self::EventkitUnavailable,
        Self::ContactsUnavailable,
        Self::CalendarReadOnly,
        Self::SqliteSyncPending,
        Self::MalformedRequestBody,
        Self::InvalidRequestBody,
        Self::UnsupportedMediaType,
        Self::InvalidReminderPriority,
        Self::EventkitInvalidInput,
        Self::ContactsInvalidInput,
        Self::EventkitTimeout,
        Self::ContactsTimeout,
    ];
}

#[cfg(test)]
mod tests {
    use axum::http::StatusCode;

    use super::ErrorCode;

    fn all_codes() -> Vec<ErrorCode> {
        ErrorCode::all().to_vec()
    }

    #[test]
    fn every_code_has_status_and_default_message() {
        for code in all_codes() {
            assert!(!code.default_message().is_empty(), "{code:?}");
            assert!(!code.as_str().is_empty(), "{code:?}");
            let _ = code.http_status();
        }
    }

    #[test]
    fn representative_status_mappings() {
        assert_eq!(
            ErrorCode::MethodNotAllowed.http_status(),
            StatusCode::METHOD_NOT_ALLOWED
        );
        assert_eq!(
            ErrorCode::InvalidRequestBody.http_status(),
            StatusCode::UNPROCESSABLE_ENTITY
        );
        assert_eq!(
            ErrorCode::QueryTimeout.http_status(),
            StatusCode::GATEWAY_TIMEOUT
        );
        assert_eq!(
            ErrorCode::ByteRangeNotSatisfiable.http_status(),
            StatusCode::RANGE_NOT_SATISFIABLE
        );
        assert_eq!(
            ErrorCode::NoteAttachmentNotFound.http_status(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            ErrorCode::UnsupportedAlarmKind.http_status(),
            StatusCode::UNPROCESSABLE_ENTITY
        );
    }

    /// `docs/errors.md` lists every code with its default message, row for row, so the catalog
    /// cannot drift from the enum.
    #[test]
    fn errors_md_lists_every_code() {
        const ERRORS_MD: &str = include_str!("../../../../docs/errors.md");
        let mut expected: Vec<String> = all_codes()
            .into_iter()
            .map(|code| format!("| `{}` | {} |", code.as_str(), code.default_message()))
            .collect();
        expected.sort();
        let actual: Vec<&str> = ERRORS_MD
            .lines()
            .filter(|line| line.starts_with("| `"))
            .collect();
        assert_eq!(actual, expected);
    }

    #[test]
    fn serialized_codes_are_snake_case() -> Result<(), Box<dyn std::error::Error>> {
        let value = serde_json::to_value(ErrorCode::NoteAttachmentNotFound)?;
        assert_eq!(value, "note_attachment_not_found");
        let value = serde_json::to_value(ErrorCode::MethodNotAllowed)?;
        assert_eq!(value, "method_not_allowed");
        Ok(())
    }
}
