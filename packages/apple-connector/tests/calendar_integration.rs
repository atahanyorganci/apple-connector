//! End-to-end HTTP integration coverage for the read-only Calendar API.

use apple_connector::{
    AppState, connect_pool,
    fixtures::{
        CalendarFixtureDb, SEED_CALENDAR_ACCOUNT_ID, SEED_CALENDAR_ID, SEED_EVENT_ID,
        SEED_OVERNIGHT_EVENT_ID, SEED_RECURRING_EVENT_ID,
    },
    router,
};
use axum::{Router, body::Body};
use http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

async fn response_json(
    app: Router,
    uri: &str,
) -> Result<(StatusCode, serde_json::Value), Box<dyn std::error::Error>> {
    let response = app
        .oneshot(Request::builder().uri(uri).body(Body::empty())?)
        .await?;
    let status = response.status();
    let body = response.into_body().collect().await?.to_bytes();
    let payload = serde_json::from_slice(&body)
        .unwrap_or(serde_json::json!({ "raw": String::from_utf8_lossy(&body) }));
    Ok((status, payload))
}

async fn response_text(
    app: Router,
    uri: &str,
) -> Result<(StatusCode, String), Box<dyn std::error::Error>> {
    let response = app
        .oneshot(Request::builder().uri(uri).body(Body::empty())?)
        .await?;
    let status = response.status();
    let body = response.into_body().collect().await?.to_bytes();
    Ok((status, String::from_utf8_lossy(&body).into_owned()))
}

#[tokio::test]
async fn integration_calendar_accounts_and_calendars() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = CalendarFixtureDb::seeded().await?;
    let pool = connect_pool(fixture.path()).await?;
    let app = router(AppState::new(None, None, None, Some(pool)));

    let (status, payload) = response_json(app.clone(), "/healthz").await?;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(payload["calendar"], "ok");

    let (status, accounts) = response_json(app.clone(), "/v1/calendar-accounts").await?;
    assert_eq!(status, StatusCode::OK);
    assert!(accounts["items"].as_array().is_some_and(|items| {
        items
            .iter()
            .any(|item| item["id"] == SEED_CALENDAR_ACCOUNT_ID)
    }));

    let (status, calendars) = response_json(app.clone(), "/v1/calendars?limit=10").await?;
    assert_eq!(status, StatusCode::OK);
    assert!(
        calendars["items"]
            .as_array()
            .is_some_and(|items| items.iter().any(|item| item["id"] == SEED_CALENDAR_ID))
    );

    let (status, calendar) =
        response_json(app, &format!("/v1/calendars/{SEED_CALENDAR_ID}")).await?;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(calendar["id"], SEED_CALENDAR_ID);
    Ok(())
}

#[tokio::test]
async fn integration_calendar_events_json_ics_and_caldav() -> Result<(), Box<dyn std::error::Error>>
{
    let fixture = CalendarFixtureDb::seeded().await?;
    let pool = connect_pool(fixture.path()).await?;
    let app = router(AppState::new(None, None, None, Some(pool)));

    let (status, events) = response_json(app.clone(), "/v1/events?limit=10").await?;
    assert_eq!(status, StatusCode::OK);
    assert!(
        events["items"]
            .as_array()
            .is_some_and(|items| items.iter().any(|item| item["id"] == SEED_EVENT_ID))
    );

    let (status, detail) =
        response_json(app.clone(), &format!("/v1/events/{SEED_EVENT_ID}")).await?;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(detail["id"], SEED_EVENT_ID);
    assert!(detail["location"].is_object());
    assert!(
        detail["attendees"]
            .as_array()
            .is_some_and(|a| !a.is_empty())
    );

    let (status, ics) =
        response_text(app.clone(), &format!("/v1/events/{SEED_EVENT_ID}/iCal")).await?;
    assert_eq!(status, StatusCode::OK);
    assert!(ics.contains("BEGIN:VCALENDAR"));
    assert!(ics.contains("SUMMARY:Team Standup"));

    let (status, caldav) =
        response_text(app.clone(), &format!("/v1/events/{SEED_EVENT_ID}/caldav")).await?;
    assert_eq!(status, StatusCode::OK);
    assert!(caldav.contains("calendar-data"));

    let (status, list_ics) = response_text(app.clone(), "/v1/events/iCal?limit=10").await?;
    assert_eq!(status, StatusCode::OK);
    assert!(list_ics.contains("BEGIN:VCALENDAR"));

    let (status, list_caldav) = response_text(app.clone(), "/v1/events/caldav?limit=10").await?;
    assert_eq!(status, StatusCode::OK);
    assert!(list_caldav.contains("calendar-data"));

    let (status, scoped) = response_json(
        app.clone(),
        &format!("/v1/calendars/{SEED_CALENDAR_ID}/events?limit=10"),
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    assert!(
        scoped["items"]
            .as_array()
            .is_some_and(|items| !items.is_empty())
    );

    let (status, recurring) =
        response_json(app, "/v1/events?start=1736942400&end=1739548800&limit=10").await?;
    assert_eq!(status, StatusCode::OK);
    assert!(recurring["items"].as_array().is_some_and(|items| {
        items
            .iter()
            .any(|item| item["id"] == SEED_RECURRING_EVENT_ID)
    }));
    Ok(())
}

fn ids_of(page: &serde_json::Value) -> Vec<String> {
    page["items"]
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item["id"].as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

fn count_of(ids: &[String], id: &str) -> usize {
    ids.iter()
        .filter(|candidate| candidate.as_str() == id)
        .count()
}

/// macOS leaves `OccurrenceCache.occurrence_start_date` NULL on the first row of every
/// occurrence. A range query that filters on that column alone drops every single-day event.
/// Regression test for #152.
#[tokio::test]
async fn integration_calendar_range_includes_single_day_events()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = CalendarFixtureDb::seeded().await?;
    let pool = connect_pool(fixture.path()).await?;
    let app = router(AppState::new(None, None, None, Some(pool)));

    // 2025-01-15 00:00 UTC → 2025-01-16 23:59:59 UTC
    let range = "start=1736899200&end=1737071999";

    let (status, page) =
        response_json(app.clone(), &format!("/v1/events?{range}&limit=50")).await?;
    assert_eq!(status, StatusCode::OK);
    let ids = ids_of(&page);
    assert_eq!(
        count_of(&ids, SEED_EVENT_ID),
        1,
        "single-day event: {ids:?}"
    );
    assert_eq!(
        count_of(&ids, SEED_RECURRING_EVENT_ID),
        1,
        "recurring event: {ids:?}"
    );
    assert_eq!(
        count_of(&ids, SEED_OVERNIGHT_EVENT_ID),
        1,
        "overnight event: {ids:?}"
    );

    let (status, scoped) = response_json(
        app,
        &format!("/v1/calendars/{SEED_CALENDAR_ID}/events?{range}&limit=50"),
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    let ids = ids_of(&scoped);
    assert_eq!(
        count_of(&ids, SEED_EVENT_ID),
        1,
        "scoped single-day event: {ids:?}"
    );
    Ok(())
}

/// An occurrence that crosses midnight is listed once, with its real start, even when the range
/// only overlaps the continuation day. Regression test for #152.
#[tokio::test]
async fn integration_calendar_range_lists_overnight_occurrence_once()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = CalendarFixtureDb::seeded().await?;
    let pool = connect_pool(fixture.path()).await?;
    let app = router(AppState::new(None, None, None, Some(pool)));

    // 2025-01-16 00:30 UTC → 2025-01-16 23:59:59 UTC: only the overnight event overlaps.
    let (status, page) =
        response_json(app, "/v1/events?start=1736987400&end=1737071999&limit=50").await?;
    assert_eq!(status, StatusCode::OK);
    let items = page["items"].as_array().ok_or("items must be an array")?;
    assert_eq!(items.len(), 1, "unexpected items: {items:?}");
    assert_eq!(items[0]["id"], SEED_OVERNIGHT_EVENT_ID);
    // 2025-01-15 23:00 UTC → 2025-01-16 01:00 UTC
    assert_eq!(items[0]["occurrence_start"], 1736982000);
    assert_eq!(items[0]["occurrence_end"], 1736989200);
    Ok(())
}

/// Follows `next_cursor` one item at a time from `base` and returns every id visited, in order.
async fn page_through(app: &Router, base: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut seen = Vec::new();
    let mut cursor: Option<String> = None;
    for _ in 0..10 {
        let uri = match &cursor {
            Some(cursor) => format!("{base}&limit=1&cursor={cursor}"),
            None => format!("{base}&limit=1"),
        };
        let (status, page) = response_json(app.clone(), &uri).await?;
        assert_eq!(status, StatusCode::OK, "{uri}: {page}");
        seen.extend(ids_of(&page));
        match page["page"]["next_cursor"].as_str() {
            Some(next) => cursor = Some(next.to_owned()),
            None => return Ok(seen),
        }
    }
    Err(format!("pagination did not terminate: {seen:?}").into())
}

/// Paging through a filtered listing one item at a time visits every event exactly once. The
/// range cursor keys on the resolved occurrence start (#152), and filtered listings hand out the
/// filter-bound cursor shape their next request decodes.
#[tokio::test]
async fn integration_calendar_filtered_listings_page_each_event_once()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = CalendarFixtureDb::seeded().await?;
    let pool = connect_pool(fixture.path()).await?;
    let app = router(AppState::new(None, None, None, Some(pool)));

    let mut expected = vec![
        SEED_EVENT_ID.to_owned(),
        SEED_RECURRING_EVENT_ID.to_owned(),
        SEED_OVERNIGHT_EVENT_ID.to_owned(),
    ];
    expected.sort();

    for base in [
        "/v1/events?start=1736899200&end=1737071999",
        "/v1/events?q=e",
    ] {
        let seen = page_through(&app, base).await?;
        let mut visited = seen.clone();
        visited.sort();
        assert_eq!(visited, expected, "{base} visited: {seen:?}");
    }
    Ok(())
}

#[tokio::test]
async fn integration_calendar_unavailable_without_database()
-> Result<(), Box<dyn std::error::Error>> {
    let app = router(AppState::new(None, None, None, None));
    let (status, payload) = response_json(app, "/v1/events?limit=1").await?;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(payload["error"]["code"], "calendar_database_unavailable");
    Ok(())
}
