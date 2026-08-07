//! End-to-end HTTP integration coverage for the read-only Messages API.

use std::time::Duration;

use apple_connector::{
    AppState, connect_pool,
    fixtures::{
        FixtureDb, NotesFixtureDb, RemindersFixtureDb, SEED_CHECKLIST_NOTE_ID, SEED_LOCKED_NOTE_ID,
        SEED_NOTES_FOLDER_ID, SEED_PLAIN_TEXT_NOTE_ID, SEED_PROJECTS_FOLDER_ID,
    },
    router,
};
use axum::{Router, body::Body, routing::get};
use http::{Method, Request, StatusCode};
use http_body_util::BodyExt;
use sqlx::Connection;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::oneshot,
};
use tower::ServiceExt;

const HELLO_FIXTURE: &[u8] = include_bytes!("../fixtures/messages/attributed-body-hello.bin");

async fn seeded_search_fixture() -> Result<FixtureDb, Box<dyn std::error::Error>> {
    let fixture = FixtureDb::empty().await?;
    let mut connection =
        sqlx::SqliteConnection::connect(fixture.path().to_str().ok_or("invalid fixture path")?)
            .await?;

    for statement in [
        "DROP TRIGGER IF EXISTS verify_chat_insert",
        "DROP TRIGGER IF EXISTS verify_chat_update",
        "INSERT INTO handle (ROWID, id, service) VALUES (1, '+15550000001', 'iMessage')",
        "INSERT INTO chat (ROWID, guid, style, chat_identifier, service_name) VALUES (1, 'chat-a', 45, '+15550000001', 'iMessage')",
        "INSERT INTO message (ROWID, guid, text, service, is_from_me, date, handle_id, cache_has_attachments) VALUES (1, 'msg-plain', 'Hello World filter text', 'iMessage', 0, 300, 1, 0)",
        "INSERT INTO message (ROWID, guid, text, service, is_from_me, date, handle_id, cache_has_attachments) VALUES (2, 'msg-attributed', NULL, 'iMessage', 0, 200, 1, 0)",
        "INSERT INTO message (ROWID, guid, text, service, is_from_me, date, handle_id, cache_has_attachments) VALUES (3, 'msg-sent', 'Sent only body', 'iMessage', 1, 100, 0, 0)",
        "INSERT INTO message (ROWID, guid, text, service, is_from_me, date, handle_id, cache_has_attachments) VALUES (4, 'msg-noise-1', 'noise alpha', 'iMessage', 0, 40, 1, 0)",
        "INSERT INTO message (ROWID, guid, text, service, is_from_me, date, handle_id, cache_has_attachments) VALUES (5, 'msg-noise-2', 'noise beta', 'iMessage', 0, 30, 1, 0)",
        "INSERT INTO chat_message_join (chat_id, message_id, message_date) SELECT 1, message.ROWID, message.date FROM message",
    ] {
        sqlx::query(statement).execute(&mut connection).await?;
    }

    sqlx::query("UPDATE message SET attributedBody = ?1 WHERE guid = 'msg-attributed'")
        .bind(HELLO_FIXTURE)
        .execute(&mut connection)
        .await?;

    connection.close().await.ok();
    Ok(fixture)
}

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

#[tokio::test]
async fn integration_health_and_unavailable_errors() -> Result<(), Box<dyn std::error::Error>> {
    let messages_fixture = FixtureDb::empty().await?;
    let reminders_fixture = RemindersFixtureDb::empty().await?;
    let messages_pool = connect_pool(messages_fixture.path()).await?;
    let reminders_pool = connect_pool(reminders_fixture.path()).await?;
    let healthy_app = router(AppState::new(
        Some(messages_pool),
        Some(reminders_pool),
        None,
        None,
    ));

    let (status, payload) = response_json(healthy_app, "/healthz").await?;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(payload["messages"], "ok");
    assert_eq!(payload["reminders"], "ok");
    assert_eq!(payload["notes"], "unavailable");
    assert_eq!(payload["calendar"], "unavailable");

    let unavailable_app = router(AppState::new(None, None, None, None));
    let (status, payload) = response_json(unavailable_app.clone(), "/healthz").await?;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(payload["messages"], "unavailable");
    assert_eq!(payload["reminders"], "unavailable");
    assert_eq!(payload["notes"], "unavailable");
    assert_eq!(payload["calendar"], "unavailable");

    let (status, payload) = response_json(unavailable_app, "/v1/messages").await?;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(payload["error"]["code"], "messages_database_unavailable");

    let fixture = seeded_search_fixture().await?;
    let pool = connect_pool(fixture.path()).await?;
    let app = router(AppState::new(Some(pool), None, None, None));

    let (status, first_page) = response_json(app.clone(), "/v1/messages?limit=2").await?;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        first_page["items"]
            .as_array()
            .ok_or("expected array")?
            .len(),
        2
    );
    assert_eq!(first_page["page"]["limit"], 2);

    let cursor = first_page["page"]["next_cursor"]
        .as_str()
        .ok_or("expected string")?;
    let (_, second_page) = response_json(
        app.clone(),
        &format!("/v1/messages?limit=2&cursor={cursor}"),
    )
    .await?;
    assert_ne!(
        first_page["items"][0]["guid"], second_page["items"][0]["guid"],
        "cursor pagination must advance"
    );

    let (_, search) = response_json(app.clone(), "/v1/messages?q=noter").await?;
    assert_eq!(search["items"][0]["guid"].as_str(), Some("msg-attributed"));

    let (_, before) = response_json(app.clone(), "/v1/messages?q=live-term").await?;
    assert!(
        before["items"]
            .as_array()
            .ok_or("expected array")?
            .is_empty()
    );

    let mut connection =
        sqlx::SqliteConnection::connect(fixture.path().to_str().ok_or("invalid path")?).await?;
    sqlx::query(
        "INSERT INTO message (guid, text, service, is_from_me, date) VALUES ('msg-live', 'live-term appears', 'iMessage', 1, 400)",
    )
    .execute(&mut connection)
    .await?;
    connection.close().await.ok();

    let (_, after) = response_json(app, "/v1/messages?q=live-term").await?;
    assert_eq!(after["items"].as_array().ok_or("expected array")?.len(), 1);
    Ok(())
}

#[tokio::test]
async fn integration_media_metadata_is_structured_without_paths()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = FixtureDb::empty().await?;
    let pool = connect_pool(fixture.path()).await?;
    let app = router(AppState::new(Some(pool), None, None, None));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/attachments/missing-guid")
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let body = response.into_body().collect().await?.to_bytes();
    let payload = String::from_utf8(body.to_vec())?;
    assert!(payload.contains("\"code\":\"resource_not_found\""));
    assert!(!payload.contains("Library/Messages"));
    assert!(!payload.contains("Attachments/"));
    Ok(())
}

#[tokio::test]
async fn integration_wrong_method_returns_json_405() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = FixtureDb::empty().await?;
    let pool = connect_pool(fixture.path()).await?;
    let app = router(AppState::new(Some(pool), None, None, None));

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::DELETE)
                .uri("/v1/messages")
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
    let body = response.into_body().collect().await?.to_bytes();
    let payload = String::from_utf8(body.to_vec())?;
    assert!(payload.contains("\"code\":\"method_not_allowed\""));
    Ok(())
}

#[tokio::test]
async fn graceful_shutdown_drains_in_flight_requests() -> Result<(), Box<dyn std::error::Error>> {
    async fn slow() -> &'static str {
        tokio::time::sleep(Duration::from_millis(150)).await;
        "ok"
    }

    let app = Router::new().route("/slow", get(slow));
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

    let server = tokio::spawn(async move {
        axum::serve(listener, app)
            .with_graceful_shutdown(async {
                shutdown_rx.await.ok();
            })
            .await
            .ok();
    });

    let client = tokio::spawn(async move {
        let Ok(mut stream) = TcpStream::connect(addr).await else {
            return false;
        };
        if stream
            .write_all(b"GET /slow HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
            .await
            .is_err()
        {
            return false;
        }
        let mut buf = [0u8; 256];
        let Ok(read) = stream.read(&mut buf).await else {
            return false;
        };
        String::from_utf8_lossy(&buf[..read]).contains("200 OK")
    });

    tokio::time::sleep(Duration::from_millis(20)).await;
    shutdown_tx
        .send(())
        .map_err(|_| "shutdown signal send failed")?;

    assert!(
        client
            .await
            .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?
    );
    server
        .await
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
    Ok(())
}

#[tokio::test]
#[ignore = "requires APPLE_CONNECTOR_MESSAGES_DATABASE pointing at a read-only chat.db copy"]
async fn smoke_real_database_and_attachment_range() -> Result<(), Box<dyn std::error::Error>> {
    let database = std::env::var("APPLE_CONNECTOR_MESSAGES_DATABASE").or_else(|_| {
        eprintln!(
            "warning: APPLE_CONNECTOR_DATABASE is deprecated; use APPLE_CONNECTOR_MESSAGES_DATABASE"
        );
        std::env::var("APPLE_CONNECTOR_DATABASE")
    })?;
    let pool = connect_pool(std::path::Path::new(&database)).await?;
    let app = router(AppState::new(Some(pool), None, None, None));

    let (status, payload) = response_json(app.clone(), "/healthz").await?;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(payload["messages"], "ok");

    let (status, messages) = response_json(app.clone(), "/v1/messages?limit=1").await?;
    assert_eq!(status, StatusCode::OK);
    assert!(
        messages["items"]
            .as_array()
            .is_some_and(|items| !items.is_empty()),
        "expected at least one message in the real database"
    );

    let (_, message) = response_json(app.clone(), "/v1/messages?limit=1").await?;
    let content = message["items"][0]["content"].clone();
    let attachments = content
        .get("attachments")
        .or_else(|| content.get("body").and_then(|body| body.get("attachments")))
        .and_then(|value| value.as_array());

    let Some(attachments) = attachments.filter(|items| !items.is_empty()) else {
        eprintln!("smoke test skipped attachment range check: no attachment on latest message");
        return Ok(());
    };

    let guid = attachments[0]["guid"].as_str().ok_or("expected string")?;
    let content_path = format!("/v1/attachments/{guid}/content");

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(&content_path)
                .header("range", "bytes=0-0")
                .body(Body::empty())?,
        )
        .await?;

    match response.status() {
        StatusCode::PARTIAL_CONTENT | StatusCode::OK => {
            assert!(response.headers().contains_key("accept-ranges"));
        }
        StatusCode::NOT_FOUND => {
            eprintln!("smoke test: attachment bytes not on disk for {guid}");
        }
        other => panic!("unexpected attachment content status {other}"),
    }
    Ok(())
}

#[tokio::test]
async fn integration_reminders_fixture_endpoints() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = RemindersFixtureDb::seeded().await?;
    let pool = connect_pool(fixture.path()).await?;
    let app = router(AppState::new(None, Some(pool), None, None));

    let (status, lists) = response_json(app.clone(), "/v1/reminder-lists?limit=10").await?;
    assert_eq!(status, StatusCode::OK);
    assert!(
        lists["items"]
            .as_array()
            .is_some_and(|items| !items.is_empty()),
        "expected seeded reminder lists"
    );

    let (status, reminders) = response_json(
        app.clone(),
        "/v1/reminder-lists/aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa/reminders?limit=10",
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    assert!(
        reminders["items"]
            .as_array()
            .is_some_and(|items| !items.is_empty()),
        "expected seeded reminders in list"
    );

    let (status, _) =
        response_json(app, "/v1/reminders/bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb").await?;
    assert_eq!(status, StatusCode::OK);
    Ok(())
}

#[tokio::test]
#[ignore = "requires APPLE_CONNECTOR_REMINDERS_DATABASE pointing at a read-only Reminders store copy"]
async fn smoke_reminders_real_database() -> Result<(), Box<dyn std::error::Error>> {
    let database = std::env::var("APPLE_CONNECTOR_REMINDERS_DATABASE")?;
    let pool = connect_pool(std::path::Path::new(&database)).await?;
    let app = router(AppState::new(None, Some(pool), None, None));

    let (status, payload) = response_json(app.clone(), "/healthz").await?;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(payload["reminders"], "ok");

    let (status, lists) = response_json(app.clone(), "/v1/reminder-lists?limit=1").await?;
    assert_eq!(status, StatusCode::OK);
    assert!(
        lists["items"]
            .as_array()
            .is_some_and(|items| !items.is_empty()),
        "expected at least one reminder list in the real database"
    );
    Ok(())
}

#[tokio::test]
async fn integration_notes_fixture_endpoints() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = NotesFixtureDb::seeded().await?;
    let pool = connect_pool(fixture.path()).await?;
    let app = router(AppState::new(None, None, Some(pool), None));

    let (status, folders) = response_json(app.clone(), "/v1/note-folders?limit=10").await?;
    assert_eq!(status, StatusCode::OK);
    assert!(
        folders["items"]
            .as_array()
            .is_some_and(|items| items.len() >= 2),
        "expected seeded note folders"
    );

    let (status, notes) = response_json(
        app.clone(),
        &format!("/v1/note-folders/{SEED_NOTES_FOLDER_ID}/notes?limit=10"),
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    assert!(
        notes["items"]
            .as_array()
            .is_some_and(|items| !items.is_empty()),
        "expected seeded notes in folder"
    );

    let (status, detail) =
        response_json(app.clone(), &format!("/v1/notes/{SEED_PLAIN_TEXT_NOTE_ID}")).await?;
    assert_eq!(status, StatusCode::OK);
    assert!(
        detail["body"]["text"]
            .as_str()
            .is_some_and(|text| text.contains("IBAN")),
        "expected decoded plain-text body"
    );

    let (status, locked) =
        response_json(app.clone(), &format!("/v1/notes/{SEED_LOCKED_NOTE_ID}")).await?;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(locked["is_locked"], true);
    assert!(locked["body"]["text"].is_null());

    let (status, checklist) =
        response_json(app.clone(), &format!("/v1/notes/{SEED_CHECKLIST_NOTE_ID}")).await?;
    assert_eq!(status, StatusCode::OK);
    assert!(
        checklist["body"]["checklist_items"]
            .as_array()
            .is_some_and(|items| !items.is_empty()),
        "expected checklist items"
    );

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/v1/notes/{SEED_CHECKLIST_NOTE_ID}/contents"))
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get("content-type")
            .and_then(|value| value.to_str().ok()),
        Some("text/markdown; charset=utf-8")
    );
    let markdown = String::from_utf8(response.into_body().collect().await?.to_bytes().to_vec())?;
    assert!(markdown.starts_with("---\n"));
    assert!(markdown.contains("schema_version: 1"));
    assert!(markdown.contains("reading"));

    let (status, smart) =
        response_json(app, &format!("/v1/note-folders/{SEED_PROJECTS_FOLDER_ID}")).await?;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(smart["kind"], "smart");
    Ok(())
}

#[tokio::test]
async fn integration_notes_search_filters() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = NotesFixtureDb::seeded().await?;
    let pool = connect_pool(fixture.path()).await?;
    let app = router(AppState::new(None, None, Some(pool), None));

    let (status, results) = response_json(app.clone(), "/v1/notes?q=IBAN&limit=10").await?;
    assert_eq!(status, StatusCode::OK);
    assert!(
        results["items"]
            .as_array()
            .is_some_and(|items| !items.is_empty()),
        "expected text search hit"
    );

    let (status, pinned) = response_json(app, "/v1/notes?is_pinned=true&limit=10").await?;
    assert_eq!(status, StatusCode::OK);
    assert!(pinned["items"].as_array().is_some());
    Ok(())
}

#[tokio::test]
#[ignore = "requires APPLE_CONNECTOR_NOTES_DATABASE pointing at a read-only Notes store copy"]
async fn smoke_notes_real_database() -> Result<(), Box<dyn std::error::Error>> {
    let database = std::env::var("APPLE_CONNECTOR_NOTES_DATABASE")?;
    let pool = connect_pool(std::path::Path::new(&database)).await?;
    let app = router(AppState::new(None, None, Some(pool), None));

    let (status, payload) = response_json(app.clone(), "/healthz").await?;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(payload["notes"], "ok");

    let (status, folders) = response_json(app.clone(), "/v1/note-folders?limit=1").await?;
    assert_eq!(status, StatusCode::OK);
    assert!(
        folders["items"]
            .as_array()
            .is_some_and(|items| !items.is_empty()),
        "expected at least one note folder in the real database"
    );
    Ok(())
}
