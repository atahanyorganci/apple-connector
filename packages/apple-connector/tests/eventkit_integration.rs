//! EventKit integration tests (ignored by default).
//!
//! Run manually on macOS with Reminders and Calendars permissions:
//! `cargo test -p apple-connector --test eventkit_integration -- --ignored`
//!
//! Optional env vars:
//! - `APPLE_CONNECTOR_TEST_REMINDER_LIST_TITLE` (default: `Reminders`)
//! - `APPLE_CONNECTOR_TEST_CALENDAR_TITLE` (default: `Calendar`)

use apple_eventkit::{
    CalendarResolveHint, CalendarStoreType, CreateEventInput, CreateReminderInput,
    DeleteEventInput, EventKitStore, EventSpan, ReminderListResolveHint, UpdateEventInput,
    UpdateReminderInput,
};

fn reminder_list_hint() -> ReminderListResolveHint {
    ReminderListResolveHint {
        api_id: "integration-test-list".into(),
        external_id: None,
        title: std::env::var("APPLE_CONNECTOR_TEST_REMINDER_LIST_TITLE")
            .unwrap_or_else(|_| "Reminders".into()),
        is_smart_list: false,
    }
}

fn calendar_hint() -> CalendarResolveHint {
    CalendarResolveHint {
        api_id: "integration-test-calendar".into(),
        external_id: None,
        title: Some(
            std::env::var("APPLE_CONNECTOR_TEST_CALENDAR_TITLE")
                .unwrap_or_else(|_| "Calendar".into()),
        ),
        store_type: CalendarStoreType::Local,
    }
}

async fn store() -> EventKitStore {
    let store = EventKitStore::new().expect("EventKit store");
    store.request_access().await.expect("EventKit access");
    store
}

#[tokio::test]
#[ignore = "requires EventKit permissions and live Apple data stores"]
async fn reminder_create_update_delete_round_trip() {
    let store = store().await;
    let list = reminder_list_hint();

    let saved = store
        .create_reminder(
            list,
            CreateReminderInput {
                title: "apple-connector integration".into(),
                notes: Some("created by ignored test".into()),
                due: None,
                completed: Some(false),
                priority: None,
                url: None,
                location: None,
                alarms: Vec::new(),
                recurrence: None,
            },
        )
        .await
        .expect("create reminder");

    store
        .update_reminder(
            &saved.calendar_item_id,
            Some(saved.external_id.as_str()),
            UpdateReminderInput {
                title: Some("apple-connector integration updated".into()),
                notes: None,
                due: None,
                completed: None,
                priority: None,
                url: None,
                list_hint: None,
                location: None,
                alarms: None,
                recurrence: None,
            },
        )
        .await
        .expect("update reminder");

    store
        .delete_reminder(&saved.calendar_item_id, Some(saved.external_id.as_str()))
        .await
        .expect("delete reminder");
}

#[tokio::test]
#[ignore = "requires EventKit permissions and live Apple data stores"]
async fn event_create_update_delete_round_trip() {
    let store = store().await;
    let calendar = calendar_hint();
    let start = chrono::Utc::now().timestamp() + 86_400;
    let end = start + 3_600;

    let saved = store
        .create_event(
            calendar.clone(),
            CreateEventInput {
                summary: "apple-connector integration".into(),
                description: Some("created by ignored test".into()),
                start,
                end,
                all_day: false,
                url: None,
                status: None,
                location: None,
                alarms: Vec::new(),
                recurrence: None,
            },
        )
        .await
        .expect("create event");

    store
        .update_event(
            &saved.calendar_item_id,
            Some(saved.external_id.as_str()),
            None,
            UpdateEventInput {
                summary: Some("apple-connector integration updated".into()),
                description: None,
                start: None,
                end: None,
                all_day: None,
                url: None,
                status: None,
                calendar_hint: Some(calendar),
                location: None,
                alarms: None,
                recurrence: None,
                span: EventSpan::This,
            },
        )
        .await
        .expect("update event");

    store
        .delete_event(
            &saved.calendar_item_id,
            Some(saved.external_id.as_str()),
            DeleteEventInput {
                span: EventSpan::This,
                occurrence_start: None,
            },
        )
        .await
        .expect("delete event");
}

#[tokio::test]
#[ignore = "requires EventKit permissions and live Apple data stores"]
async fn recurring_event_edit_with_span_this() {
    let store = store().await;
    let calendar = calendar_hint();
    let start = chrono::Utc::now().timestamp() + 172_800;
    let end = start + 3_600;

    let saved = store
        .create_event(
            calendar.clone(),
            CreateEventInput {
                summary: "apple-connector recurring".into(),
                description: None,
                start,
                end,
                all_day: false,
                url: None,
                status: None,
                location: None,
                alarms: Vec::new(),
                recurrence: Some(apple_eventkit::RecurrenceInput {
                    frequency: apple_eventkit::RecurrenceFrequency::Daily,
                    interval: 1,
                    count: Some(3),
                    end_date: None,
                }),
            },
        )
        .await
        .expect("create recurring event");

    store
        .update_event(
            &saved.calendar_item_id,
            Some(saved.external_id.as_str()),
            Some(start),
            UpdateEventInput {
                summary: Some("apple-connector recurring updated".into()),
                description: None,
                start: None,
                end: None,
                all_day: None,
                url: None,
                status: None,
                calendar_hint: Some(calendar),
                location: None,
                alarms: None,
                recurrence: None,
                span: EventSpan::This,
            },
        )
        .await
        .expect("update single occurrence");

    store
        .delete_event(
            &saved.calendar_item_id,
            Some(saved.external_id.as_str()),
            DeleteEventInput {
                span: EventSpan::All,
                occurrence_start: None,
            },
        )
        .await
        .expect("delete recurring series");
}
