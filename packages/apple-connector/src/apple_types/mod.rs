//! Centralized value types shared across the OpenAPI contract.
//!
//! These types give the HTTP surface a single, consistent representation for
//! timestamps and resource identifiers. Timestamps are whole Unix seconds and
//! identifiers are transparent newtypes so they serialize exactly like their
//! underlying primitive while remaining distinct at the type level.

mod codes;
mod ids;
mod timestamp;

pub use codes::{CodeValidationError, ReminderPriority, ReminderPriorityCategory};
pub use ids::{
    AttachmentId, CalendarAccountId, CalendarAttachmentId, CalendarId, ChatId, ContactAddressId,
    ContactEmailId, ContactId, ContactPhoneId, ContactSocialProfileId, ContactUrlId, ContainerId,
    EventId, GroupId, HandleId, IdValidationError, MessageId, NoteAttachmentId, NoteFolderId,
    NoteId, ReminderAttachmentId, ReminderId, ReminderListId, RowId, SectionId, SourceId,
};
pub use timestamp::{
    CORE_DATA_EPOCH_UNIX_SECS, UnixTimestamp, core_data_secs_from_timestamp,
    parse_core_data_timestamp,
};
