//! Centralized value types shared across the OpenAPI contract.
//!
//! These types give the HTTP surface a single, consistent representation for
//! timestamps and resource identifiers. Timestamps are whole Unix seconds and
//! identifiers are transparent newtypes so they serialize exactly like their
//! underlying primitive while remaining distinct at the type level.

mod ids;
mod timestamp;

pub use ids::{
    AttachmentId, CalendarAccountId, CalendarAttachmentId, CalendarId, ChatId, EventId, MessageId,
    NoteAttachmentId, NoteFolderId, NoteId, ReminderAttachmentId, ReminderId, ReminderListId,
    SectionId,
};
pub use timestamp::UnixTimestamp;
