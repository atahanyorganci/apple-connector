//! HTTP integration tests for write routes when EventKit / Contacts stores are absent.

use std::collections::HashMap;

use apple_connector::{
    AppState, connect_pool,
    contacts::ContactsSources,
    fixtures::{
        CalendarFixtureDb, ContactsFixtureDb, RemindersFixtureDb, SEED_CALENDAR_ID,
        SEED_CONTAINER_ID, SEED_GROUP_ID,
    },
    router,
};
use axum::{Router, body::Body};
use http::{Request, StatusCode};
use tower::ServiceExt;

async fn post_json(
    app: Router,
    uri: &str,
    body: &str,
) -> Result<StatusCode, Box<dyn std::error::Error>> {
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(uri)
                .header("content-type", "application/json")
                .body(Body::from(body.to_owned()))?,
        )
        .await?;
    Ok(response.status())
}

async fn patch_json(
    app: Router,
    uri: &str,
    body: &str,
) -> Result<StatusCode, Box<dyn std::error::Error>> {
    let response = app
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(uri)
                .header("content-type", "application/json")
                .body(Body::from(body.to_owned()))?,
        )
        .await?;
    Ok(response.status())
}

async fn delete_route(app: Router, uri: &str) -> Result<StatusCode, Box<dyn std::error::Error>> {
    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(uri)
                .body(Body::empty())?,
        )
        .await?;
    Ok(response.status())
}

async fn post_json_body(
    app: Router,
    uri: &str,
    body: &str,
) -> Result<(StatusCode, serde_json::Value), Box<dyn std::error::Error>> {
    json_body(app, "POST", uri, body).await
}

async fn patch_json_body(
    app: Router,
    uri: &str,
    body: &str,
) -> Result<(StatusCode, serde_json::Value), Box<dyn std::error::Error>> {
    json_body(app, "PATCH", uri, body).await
}

async fn json_body(
    app: Router,
    method: &str,
    uri: &str,
    body: &str,
) -> Result<(StatusCode, serde_json::Value), Box<dyn std::error::Error>> {
    use http_body_util::BodyExt;

    let response = app
        .oneshot(
            Request::builder()
                .method(method)
                .uri(uri)
                .header("content-type", "application/json")
                .body(Body::from(body.to_owned()))?,
        )
        .await?;
    let status = response.status();
    let bytes = response.into_body().collect().await?.to_bytes();
    let payload = serde_json::from_slice(&bytes)
        .unwrap_or(serde_json::json!({ "raw": String::from_utf8_lossy(&bytes) }));
    Ok((status, payload))
}

/// EventKit exposes `EKEvent.status` as read-only, so the API rejects the field instead of
/// accepting it and dropping it. The check runs before any store is consulted, which is why this
/// answers 422 rather than the 503 an absent EventKit would otherwise produce.
#[tokio::test]
async fn event_status_is_rejected_as_immutable() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = CalendarFixtureDb::seeded().await?;
    let pool = connect_pool(fixture.path()).await?;
    let app = router(AppState::with_eventkit(None, None, None, Some(pool), None));

    let (status, body) = post_json_body(
        app.clone(),
        &format!("/v1/calendars/{SEED_CALENDAR_ID}/events"),
        r#"{"summary":"Test","start":1705320000,"end":1705323600,"status":"cancelled"}"#,
    )
    .await?;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["error"]["code"], "immutable_event_field");
    assert_eq!(body["error"]["details"]["field"], "status");

    let (status, body) = patch_json_body(
        app,
        "/v1/events/00000000-0000-0000-0000-000000000001",
        r#"{"status":"confirmed"}"#,
    )
    .await?;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["error"]["code"], "immutable_event_field");
    Ok(())
}

/// `EKSpan` has only `ThisEvent` and `FutureEvents`, so `all` is gone from the API. The value now
/// fails to deserialize — and that failure has to stay inside the typed error catalog rather than
/// falling back to axum's plain-text rejection.
#[tokio::test]
async fn event_span_all_is_rejected_with_a_typed_error() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = CalendarFixtureDb::seeded().await?;
    let pool = connect_pool(fixture.path()).await?;
    let app = router(AppState::with_eventkit(None, None, None, Some(pool), None));

    let (status, body) = patch_json_body(
        app.clone(),
        "/v1/events/00000000-0000-0000-0000-000000000001",
        r#"{"span":"all"}"#,
    )
    .await?;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["error"]["code"], "unprocessable_entity");
    assert!(body["error"]["details"]["reason"].is_string());

    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/v1/events/00000000-0000-0000-0000-000000000001?span=all")
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    Ok(())
}

/// The two spans that remain must keep reaching the handler; a 503 here means the request parsed
/// and only the absent EventKit store stopped it.
#[tokio::test]
async fn this_and_future_spans_are_both_accepted() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = CalendarFixtureDb::seeded().await?;
    let pool = connect_pool(fixture.path()).await?;
    let app = router(AppState::with_eventkit(None, None, None, Some(pool), None));

    for span in ["this", "future"] {
        assert_eq!(
            patch_json(
                app.clone(),
                "/v1/events/00000000-0000-0000-0000-000000000001",
                &format!(r#"{{"span":"{span}"}}"#)
            )
            .await?,
            StatusCode::SERVICE_UNAVAILABLE,
            "span `{span}` should parse"
        );
    }
    Ok(())
}

#[tokio::test]
async fn reminder_mutations_return_503_without_eventkit() -> Result<(), Box<dyn std::error::Error>>
{
    let fixture = RemindersFixtureDb::seeded().await?;
    let pool = connect_pool(fixture.path()).await?;
    let app = router(AppState::with_eventkit(None, Some(pool), None, None, None));

    assert_eq!(
        post_json(
            app.clone(),
            "/v1/reminder-lists/00000000-0000-0000-0000-000000000001/reminders",
            r#"{"title":"Test"}"#
        )
        .await?,
        StatusCode::SERVICE_UNAVAILABLE
    );
    assert_eq!(
        patch_json(
            app.clone(),
            "/v1/reminders/00000000-0000-0000-0000-000000000001",
            r#"{"title":"Updated"}"#
        )
        .await?,
        StatusCode::SERVICE_UNAVAILABLE
    );
    assert_eq!(
        delete_route(app, "/v1/reminders/00000000-0000-0000-0000-000000000001").await?,
        StatusCode::SERVICE_UNAVAILABLE
    );
    Ok(())
}

#[tokio::test]
async fn event_mutations_return_503_without_eventkit() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = CalendarFixtureDb::seeded().await?;
    let pool = connect_pool(fixture.path()).await?;
    let app = router(AppState::with_eventkit(None, None, None, Some(pool), None));

    assert_eq!(
        post_json(
            app.clone(),
            &format!("/v1/calendars/{SEED_CALENDAR_ID}/events"),
            r#"{"summary":"Test","start":1705320000,"end":1705323600}"#
        )
        .await?,
        StatusCode::SERVICE_UNAVAILABLE
    );
    assert_eq!(
        patch_json(
            app.clone(),
            "/v1/events/00000000-0000-0000-0000-000000000001",
            r#"{"summary":"Updated"}"#
        )
        .await?,
        StatusCode::SERVICE_UNAVAILABLE
    );
    assert_eq!(
        delete_route(app, "/v1/events/00000000-0000-0000-0000-000000000001").await?,
        StatusCode::SERVICE_UNAVAILABLE
    );
    Ok(())
}

fn contacts_app(pool: sqlx::SqlitePool) -> Router {
    let mut pools = HashMap::new();
    pools.insert(
        apple_connector::apple_types::SourceId::new("fixture-source"),
        pool,
    );
    router(AppState::with_contacts(
        None,
        None,
        None,
        None,
        ContactsSources::new(pools),
        None,
    ))
}

#[tokio::test]
async fn contact_mutations_return_503_without_contacts_store()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = ContactsFixtureDb::seeded().await?;
    let pool = connect_pool(fixture.path()).await?;
    let app = contacts_app(pool);

    assert_eq!(
        post_json(
            app.clone(),
            &format!("/v1/containers/{SEED_CONTAINER_ID}/contacts"),
            r#"{"given_name":"Test","family_name":"User"}"#
        )
        .await?,
        StatusCode::SERVICE_UNAVAILABLE
    );
    assert_eq!(
        patch_json(
            app.clone(),
            "/v1/contacts/aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa",
            r#"{"given_name":"Updated"}"#
        )
        .await?,
        StatusCode::SERVICE_UNAVAILABLE
    );
    assert_eq!(
        delete_route(
            app.clone(),
            "/v1/contacts/aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa"
        )
        .await?,
        StatusCode::SERVICE_UNAVAILABLE
    );
    assert_eq!(
        post_json(
            app.clone(),
            &format!("/v1/containers/{SEED_CONTAINER_ID}/groups"),
            r#"{"name":"Test Group"}"#
        )
        .await?,
        StatusCode::SERVICE_UNAVAILABLE
    );
    assert_eq!(
        patch_json(
            app.clone(),
            &format!("/v1/groups/{SEED_GROUP_ID}"),
            r#"{"name":"Renamed"}"#
        )
        .await?,
        StatusCode::SERVICE_UNAVAILABLE
    );
    assert_eq!(
        delete_route(app.clone(), &format!("/v1/groups/{SEED_GROUP_ID}")).await?,
        StatusCode::SERVICE_UNAVAILABLE
    );
    assert_eq!(
        post_json(
            app.clone(),
            &format!("/v1/groups/{SEED_GROUP_ID}/contacts/aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa"),
            "{}"
        )
        .await?,
        StatusCode::SERVICE_UNAVAILABLE
    );
    assert_eq!(
        delete_route(
            app,
            &format!("/v1/groups/{SEED_GROUP_ID}/contacts/aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa")
        )
        .await?,
        StatusCode::SERVICE_UNAVAILABLE
    );
    Ok(())
}
