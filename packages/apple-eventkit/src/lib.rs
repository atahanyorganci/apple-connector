//! EventKit integration for Reminders and Calendar writes on macOS.
//!
//! All Objective-C / `unsafe` code is confined to this crate.

mod alarm;
mod auth;
mod calendar_resolve;
mod datetime;
mod error;
mod event;
mod item_lookup;
mod recurrence;
mod reminder;
mod store;

pub use alarm::{AlarmInput, AlarmKind};
pub use auth::{AuthStatus, EntityAuthStatus};
pub use calendar_resolve::{CalendarResolveHint, CalendarStoreType, ReminderListResolveHint};
pub use error::{EventKitError, EventKitResult};
pub use event::{
    CreateEventInput, DeleteEventInput, EventSpan, EventStatusInput, SavedEvent, UpdateEventInput,
};
pub use recurrence::{RecurrenceFrequency, RecurrenceInput};
pub use reminder::{
    CreateReminderInput, DueInput, LocationInput, SavedReminder, UpdateReminderInput,
};
pub use store::EventKitStore;
