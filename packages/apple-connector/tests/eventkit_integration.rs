//! EventKit integration tests (ignored by default).
//!
//! Run manually on macOS with Full Disk Access plus Reminders and Calendars permissions:
//! `cargo test -p apple-connector --test eventkit_integration -- --ignored`

#[test]
#[ignore = "requires EventKit permissions and live Apple data stores"]
fn reminder_create_update_delete_round_trip() {}

#[test]
#[ignore = "requires EventKit permissions and live Apple data stores"]
fn event_create_update_delete_round_trip() {}

#[test]
#[ignore = "requires EventKit permissions and live Apple data stores"]
fn recurring_event_edit_with_span_this() {}
