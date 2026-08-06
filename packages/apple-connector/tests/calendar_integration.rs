//! End-to-end HTTP integration coverage for the read-only Calendar API.

use apple_connector::{
    AppState, connect_pool,
    fixtures::{
        CalendarFixtureDb, SEED_CALENDAR_ACCOUNT_ID, SEED_CALENDAR_ID, SEED_EVENT_ID,
        SEED_RECURRING_EVENT_ID,
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

#[tokio::test]
async fn integration_calendar_unavailable_without_database()
-> Result<(), Box<dyn std::error::Error>> {
    let app = router(AppState::new(None, None, None, None));
    let (status, payload) = response_json(app, "/v1/events?limit=1").await?;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(payload["error"]["code"], "service_unavailable");
    Ok(())
}
