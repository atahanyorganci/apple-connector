//! EventKit integration tests (ignored by default).
//!
//! Run manually on macOS with Reminders and Calendars permissions:
//! `cargo test -p apple-connector --test eventkit_integration -- --ignored`
//!
//! Optional env vars:
//! - `APPLE_CONNECTOR_TEST_REMINDER_LIST_TITLE` (default: `Reminders`)
//! - `APPLE_CONNECTOR_TEST_CALENDAR_TITLE` (default: `Calendar`)
//! - `APPLE_CONNECTOR_TEST_CALENDAR_ID` (HTTP tests; overrides the title lookup)
//! - `APPLE_CONNECTOR_CALENDAR_DATABASE` (HTTP tests; default: the live `Calendar.sqlitedb`)

use std::{path::PathBuf, sync::Arc, time::Duration};

use apple_connector::{AppState, connect_pool, router};
use apple_eventkit::{
    CalendarResolveHint, CalendarStoreType, CreateEventInput, CreateReminderInput,
    DeleteEventInput, EventKitError, EventKitStore, EventSpan, ReminderListResolveHint,
    UpdateEventInput, UpdateReminderInput,
};
use axum::{Router, body::Body};
use http::{Request, StatusCode};
use http_body_util::BodyExt;
use sqlx::Row;
use tower::ServiceExt;

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

async fn store() -> Result<EventKitStore, Box<dyn std::error::Error>> {
    let store = EventKitStore::new()?;
    store.request_access().await?;
    Ok(store)
}

#[tokio::test]
#[ignore = "requires EventKit permissions and live Apple data stores"]
async fn reminder_create_update_delete_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let store = store().await?;
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
        .await?;

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
        .await?;

    store
        .delete_reminder(&saved.calendar_item_id, Some(saved.external_id.as_str()))
        .await?;
    Ok(())
}

#[tokio::test]
#[ignore = "requires EventKit permissions and live Apple data stores"]
async fn event_create_update_delete_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let store = store().await?;
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
                location: None,
                alarms: Vec::new(),
                recurrence: None,
            },
        )
        .await?;

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
                calendar_hint: Some(calendar),
                location: None,
                alarms: None,
                recurrence: None,
                span: EventSpan::This,
            },
        )
        .await?;

    store
        .delete_event(
            &saved.calendar_item_id,
            Some(saved.external_id.as_str()),
            DeleteEventInput {
                span: EventSpan::This,
                occurrence_start: None,
            },
        )
        .await?;
    Ok(())
}

#[tokio::test]
#[ignore = "requires EventKit permissions and live Apple data stores"]
async fn recurring_event_edit_with_span_this() -> Result<(), Box<dyn std::error::Error>> {
    let store = store().await?;
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
        .await?;

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
                calendar_hint: Some(calendar),
                location: None,
                alarms: None,
                recurrence: None,
                span: EventSpan::This,
            },
        )
        .await?;

    store
        .delete_event(
            &saved.calendar_item_id,
            Some(saved.external_id.as_str()),
            DeleteEventInput {
                // Deleting the series root with `Future` removes every occurrence.
                span: EventSpan::Future,
                occurrence_start: None,
            },
        )
        .await?;
    Ok(())
}

/// A start-only update cannot be checked against the request alone: the stored end is what it
/// inverts. The rejection has to come from EventKit's side of the boundary.
#[tokio::test]
#[ignore = "requires EventKit permissions and live Apple data stores"]
async fn partial_update_cannot_invert_the_stored_range() -> Result<(), Box<dyn std::error::Error>> {
    let store = store().await?;
    let calendar = calendar_hint();
    let start = chrono::Utc::now().timestamp() + 259_200;
    let end = start + 3_600;

    let saved = store
        .create_event(
            calendar.clone(),
            CreateEventInput {
                summary: "apple-connector partial range".into(),
                description: None,
                start,
                end,
                all_day: false,
                url: None,
                location: None,
                alarms: Vec::new(),
                recurrence: None,
            },
        )
        .await?;

    let start_only = store
        .update_event(
            &saved.calendar_item_id,
            Some(saved.external_id.as_str()),
            None,
            UpdateEventInput {
                start: Some(end + 3_600),
                span: EventSpan::This,
                ..empty_event_update()
            },
        )
        .await;
    assert_eq!(start_only.err(), Some(EventKitError::EndBeforeStart));

    let end_only = store
        .update_event(
            &saved.calendar_item_id,
            Some(saved.external_id.as_str()),
            None,
            UpdateEventInput {
                end: Some(start - 3_600),
                span: EventSpan::This,
                ..empty_event_update()
            },
        )
        .await;
    assert_eq!(end_only.err(), Some(EventKitError::EndBeforeStart));

    store
        .delete_event(
            &saved.calendar_item_id,
            Some(saved.external_id.as_str()),
            DeleteEventInput {
                span: EventSpan::This,
                occurrence_start: None,
            },
        )
        .await?;
    Ok(())
}

/// `assert!` for the live HTTP tests: returns an error instead of panicking, so a failure still
/// reaches [`LiveCalendar::finish`] and the test's events are cleaned up.
macro_rules! ensure {
    ($cond:expr, $($arg:tt)+) => {
        if !$cond {
            return Err(format!($($arg)+).into());
        }
    };
}

/// `assert_eq!` counterpart of [`ensure!`].
macro_rules! ensure_eq {
    ($left:expr, $right:expr $(,)?) => {
        ensure_eq!($left, $right, "values differ")
    };
    ($left:expr, $right:expr, $($arg:tt)+) => {{
        let (left, right) = (&$left, &$right);
        if left != right {
            return Err(format!(
                "{}\n  left: {left:?}\n right: {right:?}",
                format!($($arg)+)
            )
            .into());
        }
    }};
}

fn calendar_database_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
    if let Ok(path) = std::env::var("APPLE_CONNECTOR_CALENDAR_DATABASE") {
        return Ok(PathBuf::from(path));
    }
    let home = std::env::var("HOME")?;
    Ok(PathBuf::from(home)
        .join("Library/Group Containers/group.com.apple.calendar/Calendar.sqlitedb"))
}

async fn send(
    app: &Router,
    method: &str,
    uri: &str,
    body: Option<serde_json::Value>,
) -> Result<(StatusCode, serde_json::Value), Box<dyn std::error::Error>> {
    let builder = Request::builder().method(method).uri(uri);
    let request = match body {
        Some(body) => builder
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&body)?))?,
        None => builder.body(Body::empty())?,
    };
    let response = app.clone().oneshot(request).await?;
    let status = response.status();
    let bytes = response.into_body().collect().await?.to_bytes();
    let payload = serde_json::from_slice(&bytes)
        .unwrap_or(serde_json::json!({ "raw": String::from_utf8_lossy(&bytes) }));
    Ok((status, payload))
}

/// Polls `GET uri` until `done` accepts the response or the deadline passes, and returns the last
/// response either way. The SQLite read path trails EventKit writes by an unspecified delay.
async fn poll_get<F>(
    app: &Router,
    uri: &str,
    done: F,
) -> Result<(StatusCode, serde_json::Value), Box<dyn std::error::Error>>
where
    F: Fn(StatusCode, &serde_json::Value) -> bool,
{
    let deadline = tokio::time::Instant::now() + Duration::from_secs(15);
    loop {
        let (status, body) = send(app, "GET", uri, None).await?;
        if done(status, &body) || tokio::time::Instant::now() >= deadline {
            return Ok((status, body));
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
}

async fn test_calendar_id(app: &Router) -> Result<String, Box<dyn std::error::Error>> {
    if let Ok(id) = std::env::var("APPLE_CONNECTOR_TEST_CALENDAR_ID") {
        return Ok(id);
    }
    let title =
        std::env::var("APPLE_CONNECTOR_TEST_CALENDAR_TITLE").unwrap_or_else(|_| "Calendar".into());
    let (status, page) = send(app, "GET", "/v1/calendars?limit=200", None).await?;
    ensure_eq!(status, StatusCode::OK, "list calendars: {page}");
    page["items"]
        .as_array()
        .and_then(|items| items.iter().find(|item| item["title"] == title.as_str()))
        .and_then(|item| item["id"].as_str())
        .map(str::to_owned)
        .ok_or_else(|| format!("no calendar titled {title:?}").into())
}

/// Live HTTP harness: the real Calendar database for reads and a real EventKit store for writes,
/// wired into the router exactly like the server does. Every event a test creates carries
/// `marker` in its summary so [`LiveCalendar::finish`] can remove it whatever the outcome.
struct LiveCalendar {
    app: Router,
    pool: sqlx::SqlitePool,
    store: Arc<EventKitStore>,
    marker: String,
}

impl LiveCalendar {
    async fn new(label: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let pool = connect_pool(&calendar_database_path()?).await?;
        let store = Arc::new(store().await?);
        let app = router(AppState::with_eventkit(
            None,
            None,
            None,
            Some(pool.clone()),
            Some(Arc::clone(&store)),
        ));
        let marker = format!(
            "apple-connector {label} {}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
        );
        Ok(Self {
            app,
            pool,
            store,
            marker,
        })
    }

    async fn create_event(
        &self,
    ) -> Result<(StatusCode, serde_json::Value), Box<dyn std::error::Error>> {
        let calendar_id = test_calendar_id(&self.app).await?;
        let start = chrono::Utc::now().timestamp() + 7 * 86_400;
        let (status, created) = send(
            &self.app,
            "POST",
            &format!("/v1/calendars/{calendar_id}/events"),
            Some(
                serde_json::json!({ "summary": self.marker, "start": start, "end": start + 3_600 }),
            ),
        )
        .await?;
        ensure!(
            status == StatusCode::CREATED || status == StatusCode::ACCEPTED,
            "create: {status} {created}"
        );
        Ok((status, created))
    }

    /// Returns the test's outcome after deleting every marked event straight through EventKit
    /// with the stored identifiers, so a failing HTTP round trip never leaves events behind.
    async fn finish(
        self,
        outcome: Result<(), Box<dyn std::error::Error>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let cleanup = self.cleanup().await;
        if let (Err(_), Err(error)) = (&outcome, &cleanup) {
            eprintln!("cleanup also failed: {error}");
        }
        outcome?;
        cleanup
    }

    async fn cleanup(&self) -> Result<(), Box<dyn std::error::Error>> {
        // The row can still be mid-sync with the server when the test ends, so retry until the
        // SQLite read path no longer shows it.
        let mut last_error = None;
        for _ in 0..20 {
            let rows = sqlx::query(
                "SELECT UUID, unique_identifier FROM CalendarItem WHERE summary LIKE ?1",
            )
            .bind(format!("{}%", self.marker))
            .fetch_all(&self.pool)
            .await?;
            if rows.is_empty() {
                return Ok(());
            }
            for row in rows {
                let uuid: String = row.try_get("UUID")?;
                let unique_identifier: Option<String> = row.try_get("unique_identifier")?;
                // A miss here is retried on the next pass.
                if let Err(error) = self
                    .store
                    .delete_event(
                        &uuid,
                        unique_identifier.as_deref(),
                        DeleteEventInput {
                            span: EventSpan::This,
                            occurrence_start: None,
                        },
                    )
                    .await
                {
                    last_error = Some(format!("{uuid}: {error:?}"));
                }
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
        Err(format!(
            "events marked {:?} were not cleaned up (last error: {last_error:?})",
            self.marker
        )
        .into())
    }
}

/// The id returned by `POST` is a read-API id: it resolves through `GET` and matches what the list
/// endpoint reports. Regression test for #150.
#[tokio::test]
#[ignore = "requires EventKit permissions, Full Disk Access, and live Apple data stores"]
async fn http_created_event_id_resolves_through_get() -> Result<(), Box<dyn std::error::Error>> {
    let live = LiveCalendar::new("created id").await?;
    let outcome = created_event_id_resolves(&live).await;
    live.finish(outcome).await
}

async fn created_event_id_resolves(live: &LiveCalendar) -> Result<(), Box<dyn std::error::Error>> {
    let (_, created) = live.create_event().await?;
    let event_id = created["id"]
        .as_str()
        .ok_or("create returned no id")?
        .to_owned();

    let (status, fetched) = poll_get(&live.app, &format!("/v1/events/{event_id}"), |status, _| {
        status == StatusCode::OK
    })
    .await?;
    ensure_eq!(
        status,
        StatusCode::OK,
        "GET created id {event_id}: {fetched}"
    );
    ensure_eq!(fetched["id"], event_id.as_str());
    ensure_eq!(fetched["summary"], live.marker.as_str());

    let (status, listed) = send(
        &live.app,
        "GET",
        &format!("/v1/events?q={}&limit=5", live.marker.replace(' ', "%20")),
        None,
    )
    .await?;
    ensure_eq!(status, StatusCode::OK, "list: {listed}");
    ensure_eq!(
        listed["items"][0]["id"],
        event_id.as_str(),
        "list: {listed}"
    );
    Ok(())
}

/// The id the read API reports reaches the EventKit event for `PATCH` and `DELETE`.
/// Regression test for #151.
#[tokio::test]
#[ignore = "requires EventKit permissions, Full Disk Access, and live Apple data stores"]
async fn http_listed_event_id_works_for_patch_and_delete() -> Result<(), Box<dyn std::error::Error>>
{
    let live = LiveCalendar::new("listed id").await?;
    let outcome = listed_event_id_mutates(&live).await;
    live.finish(outcome).await
}

async fn listed_event_id_mutates(live: &LiveCalendar) -> Result<(), Box<dyn std::error::Error>> {
    live.create_event().await?;

    // Take the id from the read API, as a client would, not from the create response.
    let search = format!("/v1/events?q={}&limit=5", live.marker.replace(' ', "%20"));
    let (status, listed) = poll_get(&live.app, &search, |status, body| {
        status == StatusCode::OK && body["items"][0]["id"].is_string()
    })
    .await?;
    ensure_eq!(status, StatusCode::OK, "list: {listed}");
    let event_id = listed["items"][0]["id"]
        .as_str()
        .ok_or(format!("event never listed: {listed}"))?
        .to_owned();

    let updated_summary = format!("{} updated", live.marker);
    let (status, updated) = send(
        &live.app,
        "PATCH",
        &format!("/v1/events/{event_id}"),
        Some(serde_json::json!({ "summary": updated_summary })),
    )
    .await?;
    ensure!(
        status == StatusCode::OK || status == StatusCode::ACCEPTED,
        "PATCH {event_id}: {status} {updated}"
    );
    ensure_eq!(
        updated["id"],
        event_id.as_str(),
        "PATCH response id: {updated}"
    );
    let (_, fetched) = poll_get(&live.app, &format!("/v1/events/{event_id}"), |_, body| {
        body["summary"] == updated_summary.as_str()
    })
    .await?;
    ensure_eq!(fetched["summary"], updated_summary.as_str());

    let (status, deleted) =
        send(&live.app, "DELETE", &format!("/v1/events/{event_id}"), None).await?;
    ensure_eq!(
        status,
        StatusCode::NO_CONTENT,
        "DELETE {event_id}: {deleted}"
    );
    let (status, _) = poll_get(&live.app, &format!("/v1/events/{event_id}"), |status, _| {
        status == StatusCode::NOT_FOUND
    })
    .await?;
    ensure_eq!(status, StatusCode::NOT_FOUND);
    Ok(())
}

fn empty_event_update() -> UpdateEventInput {
    UpdateEventInput {
        summary: None,
        description: None,
        start: None,
        end: None,
        all_day: None,
        url: None,
        calendar_hint: None,
        location: None,
        alarms: None,
        recurrence: None,
        span: EventSpan::This,
    }
}

/// `this` and `future` have to mean different things — an "all" span used to map to `future` and
/// silently claim a scope EventKit never applied.
#[tokio::test]
#[ignore = "requires EventKit permissions and live Apple data stores"]
async fn recurring_event_edit_with_span_future() -> Result<(), Box<dyn std::error::Error>> {
    let store = store().await?;
    let calendar = calendar_hint();
    let start = chrono::Utc::now().timestamp() + 345_600;
    let end = start + 3_600;

    let saved = store
        .create_event(
            calendar.clone(),
            CreateEventInput {
                summary: "apple-connector span future".into(),
                description: None,
                start,
                end,
                all_day: false,
                url: None,
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
        .await?;

    // The second occurrence forward, leaving the first as it was.
    store
        .update_event(
            &saved.calendar_item_id,
            Some(saved.external_id.as_str()),
            Some(start + 86_400),
            UpdateEventInput {
                summary: Some("apple-connector span future updated".into()),
                calendar_hint: Some(calendar),
                span: EventSpan::Future,
                ..empty_event_update()
            },
        )
        .await?;

    store
        .delete_event(
            &saved.calendar_item_id,
            Some(saved.external_id.as_str()),
            DeleteEventInput {
                span: EventSpan::Future,
                occurrence_start: None,
            },
        )
        .await?;
    Ok(())
}
