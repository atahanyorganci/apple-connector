//! Claims made in `packages/apple-connector/docs/SPEC.md` that no other test covered.
//!
//! Tests marked `#[ignore = "bug: ..."]` assert the behaviour the spec records as correct and
//! currently fail; each one is listed under "Known bugs" in the spec. Run them with
//! `cargo test -p apple-connector --test spec -- --ignored` to see the bug, and drop the
//! attribute in the PR that fixes it.

use std::collections::HashMap;

use apple_connector::{
    AppState, Cli, connect_pool,
    contacts::ContactsSources,
    fixtures::{CalendarFixtureDb, ContactsFixtureDb},
    router,
};
use axum::{Router, body::Body};
use clap::Parser;
use http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

type TestResult = Result<(), Box<dyn std::error::Error>>;

async fn send(
    app: &Router,
    method: &str,
    uri: &str,
    body: Option<&str>,
) -> Result<(StatusCode, serde_json::Value), Box<dyn std::error::Error>> {
    let builder = Request::builder().method(method).uri(uri);
    let request = match body {
        Some(body) => builder
            .header("content-type", "application/json")
            .body(Body::from(body.to_owned()))?,
        None => builder.body(Body::empty())?,
    };
    let response = app.clone().oneshot(request).await?;
    let status = response.status();
    let bytes = response.into_body().collect().await?.to_bytes();
    let payload = serde_json::from_slice(&bytes)
        .unwrap_or(serde_json::json!({ "raw": String::from_utf8_lossy(&bytes) }));
    Ok((status, payload))
}

/// Messages is the one store the server cannot start without: a missing `chat.db` aborts startup
/// before any listener is bound, while every other store degrades to "unavailable".
#[tokio::test]
async fn a_missing_messages_database_aborts_startup() -> TestResult {
    let cli = Cli::try_parse_from([
        "apple-connector",
        "--messages-database",
        "/nonexistent/apple-connector-spec/chat.db",
    ])?;
    let outcome = apple_connector::run(cli).await;
    assert!(outcome.is_err(), "startup succeeded without chat.db");
    Ok(())
}

/// EventKit cannot store a flag on a reminder, so `flagged` is rejected whatever its value —
/// including `false`. Validation runs before any store is consulted, so no EventKit is needed.
///
/// The Raycast "Create Reminder" form once sent `flagged` on every submission and was refused
/// every time (#155); it now sends only fields EventKit can store.
#[tokio::test]
async fn flagged_false_is_still_an_unsupported_reminder_field() -> TestResult {
    let app = router(AppState::new(None, None, None, None));
    let (status, body) = send(
        &app,
        "POST",
        "/v1/reminder-lists/00000000-0000-0000-0000-000000000001/reminders",
        Some(r#"{"title":"spec","flagged":false}"#),
    )
    .await?;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert_eq!(body["error"]["code"], "unsupported_reminder_field");
    assert_eq!(body["error"]["details"]["field"], "flagged");

    for (field, payload) in [
        ("tags", r#"{"title":"spec","tags":["errand"]}"#),
        (
            "section_id",
            r#"{"title":"spec","section_id":"00000000-0000-0000-0000-000000000002"}"#,
        ),
    ] {
        let (status, body) = send(
            &app,
            "POST",
            "/v1/reminder-lists/00000000-0000-0000-0000-000000000001/reminders",
            Some(payload),
        )
        .await?;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{field}: {body}");
        assert_eq!(body["error"]["details"]["field"], field);
    }
    Ok(())
}

/// `q` on Contacts listings is a SQL `LIKE` substring with no escape clause, so `%` matches
/// everything rather than a literal percent sign.
#[tokio::test]
async fn contact_search_treats_percent_as_a_wildcard() -> TestResult {
    let fixture = ContactsFixtureDb::seeded_with_batch_contacts(3).await?;
    let pool = connect_pool(fixture.path()).await?;
    let sources = ContactsSources::new(HashMap::from([(
        apple_connector::apple_types::SourceId::new("spec-source"),
        pool,
    )]));
    let app = router(AppState::with_contacts(
        None, None, None, None, sources, None,
    ));

    let (status, all) = send(&app, "GET", "/v1/contacts?limit=200", None).await?;
    assert_eq!(status, StatusCode::OK, "{all}");
    let (status, wildcard) = send(&app, "GET", "/v1/contacts?q=%25&limit=200", None).await?;
    assert_eq!(status, StatusCode::OK, "{wildcard}");
    let count = |page: &serde_json::Value| page["items"].as_array().map(Vec::len);
    assert_eq!(count(&wildcard), count(&all));
    Ok(())
}

/// A cursor is only valid for the filters that produced it. Messages, Reminders, Notes, and Events
/// enforce this; Contacts does not, so a cursor taken without `q` silently continues a `q` search.
#[tokio::test]
#[ignore = "bug: Contacts cursors are not bound to their filters (SPEC.md, Known bugs)"]
async fn contact_cursors_are_bound_to_their_filters() -> TestResult {
    let fixture = ContactsFixtureDb::seeded_with_batch_contacts(3).await?;
    let pool = connect_pool(fixture.path()).await?;
    let sources = ContactsSources::new(HashMap::from([(
        apple_connector::apple_types::SourceId::new("spec-source"),
        pool,
    )]));
    let app = router(AppState::with_contacts(
        None, None, None, None, sources, None,
    ));

    let (status, first) = send(&app, "GET", "/v1/contacts?limit=1", None).await?;
    assert_eq!(status, StatusCode::OK, "{first}");
    let cursor = first["page"]["next_cursor"]
        .as_str()
        .ok_or_else(|| format!("first page has no cursor: {first}"))?;

    let (status, body) = send(
        &app,
        "GET",
        &format!("/v1/contacts?limit=1&q=Person&cursor={cursor}"),
        None,
    )
    .await?;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert_eq!(body["error"]["code"], "invalid_cursor");
    Ok(())
}

/// A contact id names one contact. The photo route matches `ZUNIQUEID` with
/// `LIKE '<id>:' || '%'` and does not escape `LIKE` metacharacters, so an id of `%` returns some
/// contact's photo; every other contact route compares the id exactly.
#[tokio::test]
#[ignore = "bug: the photo route treats % and _ in the id as wildcards (SPEC.md, Known bugs)"]
async fn a_photo_is_only_served_for_the_exact_contact_id() -> TestResult {
    use sqlx::{Connection, sqlite::SqliteConnectOptions};

    let fixture = ContactsFixtureDb::seeded().await?;
    {
        let options = SqliteConnectOptions::new().filename(fixture.path());
        let mut connection = sqlx::SqliteConnection::connect_with(&options).await?;
        sqlx::query(
            "UPDATE ZABCDRECORD SET ZIMAGEDATA = x'FFD8FFE0', ZIMAGETYPE = 'JPEG' \
             WHERE lower(ZUNIQUEID) LIKE lower(?1) || ':%'",
        )
        .bind(apple_connector::fixtures::SEED_CONTACT_ID)
        .execute(&mut connection)
        .await?;
        connection.close().await?;
    }
    let pool = connect_pool(fixture.path()).await?;
    let sources = ContactsSources::new(HashMap::from([(
        apple_connector::apple_types::SourceId::new("spec-source"),
        pool,
    )]));
    let app = router(AppState::with_contacts(
        None, None, None, None, sources, None,
    ));

    let exact = format!(
        "/v1/contacts/{}/photo",
        apple_connector::fixtures::SEED_CONTACT_ID
    );
    let (status, body) = send(&app, "GET", &exact, None).await?;
    assert_eq!(status, StatusCode::OK, "exact id: {body}");

    let prefix = apple_connector::fixtures::SEED_CONTACT_ID
        .get(..4)
        .ok_or("seed id is too short")?;
    let (status, body) = send(&app, "GET", &format!("/v1/contacts/{prefix}/photo"), None).await?;
    assert_eq!(status, StatusCode::NOT_FOUND, "prefix {prefix:?}: {body}");

    let (status, body) = send(&app, "GET", "/v1/contacts/%25/photo", None).await?;
    assert_eq!(status, StatusCode::NOT_FOUND, "wildcard id: {body}");
    Ok(())
}

/// #99: an unsupported Calendar schema must stop startup, not pass `/healthz` and then fail every
/// query. Startup's schema gate is `warm_entity_id_caches`, which never looks at Calendar.
#[tokio::test]
#[ignore = "bug: legacy Calendar schemas are not rejected at startup (SPEC.md, Known bugs)"]
async fn a_legacy_calendar_schema_fails_the_startup_gate() -> TestResult {
    let fixture = CalendarFixtureDb::legacy_unsupported().await?;
    let pool = connect_pool(fixture.path()).await?;
    let state = AppState::new(None, None, None, Some(pool));
    assert!(
        state.warm_entity_id_caches().await.is_err(),
        "a ZCALENDARITEM database passed the startup gate"
    );
    Ok(())
}

/// A contact born before 2001 has a negative Core Data `ZBIRTHDAY`; the API returns it (#157).
#[tokio::test]
async fn a_birthday_before_2001_is_returned() -> TestResult {
    use sqlx::{Connection, sqlite::SqliteConnectOptions};

    // 1990-01-01T00:00:00Z: Unix 631152000, Core Data -347155200.
    let fixture = ContactsFixtureDb::seeded().await?;
    {
        let options = SqliteConnectOptions::new().filename(fixture.path());
        let mut connection = sqlx::SqliteConnection::connect_with(&options).await?;
        sqlx::query(
            "UPDATE ZABCDRECORD SET ZBIRTHDAY = -347155200 \
             WHERE lower(ZUNIQUEID) LIKE lower(?1) || ':%'",
        )
        .bind(apple_connector::fixtures::SEED_CONTACT_ID)
        .execute(&mut connection)
        .await?;
        connection.close().await?;
    }
    let pool = connect_pool(fixture.path()).await?;
    let sources = ContactsSources::new(HashMap::from([(
        apple_connector::apple_types::SourceId::new("spec-source"),
        pool,
    )]));
    let app = router(AppState::with_contacts(
        None, None, None, None, sources, None,
    ));

    let uri = format!(
        "/v1/contacts/{}",
        apple_connector::fixtures::SEED_CONTACT_ID
    );
    let (status, body) = send(&app, "GET", &uri, None).await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["birthday"], 631_152_000, "{body}");
    Ok(())
}

/// Timestamp query bounds are Unix seconds; an RFC 3339 value must still be answered inside the
/// error envelope. Read handlers use axum's `Query`, whose rejection is plain text.
#[tokio::test]
#[ignore = "bug: read endpoints answer malformed query parameters outside the error envelope (SPEC.md, Known bugs)"]
async fn an_rfc3339_query_bound_is_a_typed_error() -> TestResult {
    let app = router(AppState::new(None, None, None, None));
    let (status, body) = send(
        &app,
        "GET",
        "/v1/messages?before=2024-01-01T00:00:00Z",
        None,
    )
    .await?;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert!(
        body["error"]["code"].is_string(),
        "not an error envelope: {body}"
    );
    Ok(())
}

/// Core Data stores instants as seconds relative to 2001-01-01, so anything earlier is negative.
/// Only `NULL` and zero mean "unset"; a contact born in 1990 has a perfectly valid negative
/// `ZBIRTHDAY` (#157).
#[test]
fn core_data_dates_before_2001_are_kept() -> TestResult {
    let one_day_before = apple_connector::apple_types::parse_core_data_timestamp(Some(-86_400.0))
        .ok_or("a date one day before the Core Data epoch was read as unset")?;
    assert_eq!(one_day_before.timestamp(), 978_307_200 - 86_400);
    Ok(())
}
