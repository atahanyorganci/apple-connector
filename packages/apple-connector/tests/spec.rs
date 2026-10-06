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

    // The cursor still continues the query it came from, filtered or not.
    let (status, second) = send(
        &app,
        "GET",
        &format!("/v1/contacts?limit=1&cursor={cursor}"),
        None,
    )
    .await?;
    assert_eq!(status, StatusCode::OK, "{second}");
    assert_ne!(second["items"][0]["id"], first["items"][0]["id"]);

    let (status, filtered) = send(&app, "GET", "/v1/contacts?limit=1&q=Person", None).await?;
    assert_eq!(status, StatusCode::OK, "{filtered}");
    let filtered_cursor = filtered["page"]["next_cursor"]
        .as_str()
        .ok_or_else(|| format!("filtered page has no cursor: {filtered}"))?;
    let (status, next) = send(
        &app,
        "GET",
        &format!("/v1/contacts?limit=1&q=Person&cursor={filtered_cursor}"),
        None,
    )
    .await?;
    assert_eq!(status, StatusCode::OK, "{next}");
    let (status, dropped) = send(
        &app,
        "GET",
        &format!("/v1/contacts?limit=1&cursor={filtered_cursor}"),
        None,
    )
    .await?;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "dropping the filter: {dropped}"
    );
    Ok(())
}

/// A contact id names one contact. The photo route used to match `ZUNIQUEID` with
/// `LIKE '<id>:' || '%'`, so an id of `%` returned some contact's photo (#163); it now compares the
/// id exactly, like every other contact route.
#[tokio::test]
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

/// Timestamp query bounds are Unix seconds; an RFC 3339 value is still answered inside the error
/// envelope, as is a path segment that does not parse (#160).
#[tokio::test]
async fn malformed_query_and_path_parameters_are_typed_errors() -> TestResult {
    let fixture = apple_connector::fixtures::FixtureDb::seeded().await?;
    let pool = connect_pool(fixture.path()).await?;
    let app = router(AppState::new(Some(pool), None, None, None));

    let (status, body) = send(
        &app,
        "GET",
        "/v1/messages?before=2024-01-01T00:00:00Z",
        None,
    )
    .await?;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert_eq!(body["error"]["code"], "invalid_parameter", "{body}");
    assert!(
        body["error"]["details"]["reason"]
            .as_str()
            .is_some_and(|reason| reason.contains("before")),
        "{body}"
    );

    let (status, body) = send(&app, "GET", "/v1/chats/abc", None).await?;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert_eq!(body["error"]["code"], "invalid_parameter", "{body}");
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

/// Protobuf helpers for building a note body that embeds a table, as Notes writes it.
mod note_body {
    fn varint(mut value: u64) -> Vec<u8> {
        let mut out = Vec::new();
        loop {
            let byte = (value & 0x7f) as u8;
            value >>= 7;
            if value == 0 {
                out.push(byte);
                return out;
            }
            out.push(byte | 0x80);
        }
    }

    fn bytes_field(number: u64, payload: &[u8]) -> Vec<u8> {
        let mut out = varint(number << 3 | 2);
        out.extend(varint(payload.len() as u64));
        out.extend_from_slice(payload);
        out
    }

    fn varint_field(number: u64, value: u64) -> Vec<u8> {
        let mut out = varint(number << 3);
        out.extend(varint(value));
        out
    }

    /// `text` with one attribute run per `(length, attachment)`, gzip-compressed.
    pub fn gzipped(
        text: &str,
        runs: &[(u64, Option<(&str, &str)>)],
    ) -> Result<Vec<u8>, std::io::Error> {
        use std::io::Write;

        let mut note = bytes_field(2, text.as_bytes());
        for (length, attachment) in runs {
            let mut run = varint_field(1, *length);
            if let Some((identifier, uti)) = attachment {
                let mut info = bytes_field(1, identifier.as_bytes());
                info.extend(bytes_field(2, uti.as_bytes()));
                run.extend(bytes_field(12, &info));
            }
            note.extend(bytes_field(5, &run));
        }
        let mut document = varint_field(2, 0);
        document.extend(bytes_field(3, &note));
        let root = bytes_field(2, &document);
        let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(&root)?;
        encoder.finish()
    }
}

/// A table's cells live in its attachment's mergeable data, not in the note body; the note detail
/// carries them and `/contents` renders them in place (#168).
#[tokio::test]
async fn embedded_tables_are_decoded_into_the_note() -> TestResult {
    use apple_connector::fixtures::{NotesFixtureDb, SEED_PLAIN_TEXT_NOTE_ID};
    use sqlx::{Connection, sqlite::SqliteConnectOptions};

    const TABLE_ID: &str = "7A5BE1E0-0000-4000-8000-0000000000AB";
    let table = std::fs::read(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/notes/bodies/acnp/table_gzipped.bin"),
    )?;
    let body = note_body::gzipped(
        "Before\n\u{FFFC}\nAfter",
        &[
            (7, None),
            (1, Some((TABLE_ID, "com.apple.notes.table"))),
            (6, None),
        ],
    )?;

    let fixture = NotesFixtureDb::seeded().await?;
    {
        let options = SqliteConnectOptions::new().filename(fixture.path());
        let mut connection = sqlx::SqliteConnection::connect_with(&options).await?;
        sqlx::query(
            "UPDATE ZICNOTEDATA SET ZDATA = ?1 \
             WHERE ZNOTE = (SELECT Z_PK FROM ZICCLOUDSYNCINGOBJECT WHERE ZIDENTIFIER = ?2)",
        )
        .bind(&body)
        .bind(SEED_PLAIN_TEXT_NOTE_ID)
        .execute(&mut connection)
        .await?;
        sqlx::query(
            "INSERT INTO ZICCLOUDSYNCINGOBJECT \
             (Z_PK, Z_ENT, Z_OPT, ZMARKEDFORDELETION, ZNOTE, ZIDENTIFIER, ZTYPEUTI, ZMERGEABLEDATA1) \
             VALUES (900, (SELECT Z_ENT FROM Z_PRIMARYKEY WHERE Z_NAME = 'ICAttachment'), 1, 0, \
                     (SELECT Z_PK FROM ZICCLOUDSYNCINGOBJECT WHERE ZIDENTIFIER = ?1), ?2, \
                     'com.apple.notes.table', ?3)",
        )
        .bind(SEED_PLAIN_TEXT_NOTE_ID)
        .bind(TABLE_ID)
        .bind(&table)
        .execute(&mut connection)
        .await?;
        connection.close().await?;
    }
    let pool = connect_pool(fixture.path()).await?;
    let app = router(AppState::new(None, None, Some(pool), None));

    let (status, detail) = send(
        &app,
        "GET",
        &format!("/v1/notes/{SEED_PLAIN_TEXT_NOTE_ID}"),
        None,
    )
    .await?;
    assert_eq!(status, StatusCode::OK, "{detail}");
    let embedded = &detail["body"]["embedded"][0];
    assert_eq!(embedded["attachment_identifier"], TABLE_ID, "{detail}");
    assert_eq!(
        embedded["table"]["rows"],
        serde_json::json!([
            ["Row 1 Column 1", "Row 1 Column 2"],
            ["Row 2 Column 1", "Row 2 Column 2"]
        ]),
        "{detail}"
    );
    assert_eq!(embedded["table"]["right_to_left"], false);

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/v1/notes/{SEED_PLAIN_TEXT_NOTE_ID}/contents"))
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let markdown = String::from_utf8(response.into_body().collect().await?.to_bytes().to_vec())?;
    assert!(
        markdown.contains(
            "Before\n| Row 1 Column 1 | Row 1 Column 2 |\n| --- | --- |\n| Row 2 Column 1 | Row 2 Column 2 |\n"
        ),
        "{markdown}"
    );
    assert!(markdown.trim_end().ends_with("After"), "{markdown}");
    Ok(())
}

/// Read-only probe: every table attachment in the local NoteStore decodes. Prints counts only.
///
/// ```bash
/// cargo test -p apple-connector --test spec live_note_tables_decode -- --ignored --nocapture
/// ```
#[tokio::test]
#[ignore = "reads the local NoteStore; needs Full Disk Access"]
async fn live_note_tables_decode() -> TestResult {
    use sqlx::Row;

    let path = std::path::PathBuf::from(std::env::var("HOME")?)
        .join("Library/Group Containers/group.com.apple.notes/NoteStore.sqlite");
    let pool = connect_pool(&path).await?;
    let rows = sqlx::query(
        "SELECT ZMERGEABLEDATA1 FROM ZICCLOUDSYNCINGOBJECT \
         WHERE ZTYPEUTI = 'com.apple.notes.table' AND ZMARKEDFORDELETION = 0",
    )
    .fetch_all(&pool)
    .await?;
    let (mut decoded, mut cells) = (0, 0);
    let mut failures = Vec::new();
    for row in &rows {
        let data: Option<Vec<u8>> = row.try_get(0)?;
        match data.as_deref().map(apple_notes_protobuf::decode_table) {
            Some(Ok(table)) => {
                decoded += 1;
                cells += table.rows.iter().map(Vec::len).sum::<usize>();
            }
            Some(Err(error)) => failures.push(error.to_string()),
            None => failures.push("no mergeable data".to_owned()),
        }
    }
    println!(
        "tables: {}, decoded: {decoded}, cells: {cells}, failures: {failures:?}",
        rows.len()
    );
    assert!(failures.is_empty(), "{failures:?}");
    Ok(())
}
