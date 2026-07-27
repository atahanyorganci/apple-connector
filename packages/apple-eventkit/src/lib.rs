//! EventKit integration for Reminders and Calendar writes on macOS.
//!
//! All Objective-C / `unsafe` code is confined to this crate.

mod error;

#[cfg(target_os = "macos")]
mod alarm;
#[cfg(target_os = "macos")]
mod auth;
#[cfg(target_os = "macos")]
mod calendar_resolve;
#[cfg(target_os = "macos")]
mod datetime;
#[cfg(target_os = "macos")]
mod event;
#[cfg(target_os = "macos")]
mod item_lookup;
#[cfg(target_os = "macos")]
mod recurrence;
#[cfg(target_os = "macos")]
mod reminder;
#[cfg(target_os = "macos")]
mod store;

#[cfg(target_os = "macos")]
pub use alarm::{AlarmInput, AlarmKind};
#[cfg(target_os = "macos")]
pub use auth::{AuthStatus, EntityAuthStatus};
#[cfg(target_os = "macos")]
pub use calendar_resolve::{CalendarResolveHint, CalendarStoreType, ReminderListResolveHint};
pub use error::{EventKitError, EventKitResult};
#[cfg(target_os = "macos")]
pub use event::{
    CreateEventInput, DeleteEventInput, EventSpan, EventStatusInput, UpdateEventInput,
};
#[cfg(target_os = "macos")]
pub use recurrence::{RecurrenceFrequency, RecurrenceInput};
#[cfg(target_os = "macos")]
pub use reminder::{CreateReminderInput, DueInput, LocationInput, UpdateReminderInput};
#[cfg(target_os = "macos")]
pub use store::EventKitStore;

#[cfg(not(target_os = "macos"))]
mod stub;

#[cfg(not(target_os = "macos"))]
pub use stub::*;
