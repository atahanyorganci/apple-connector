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

async fn post_json(app: Router, uri: &str, body: &str) -> StatusCode {
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(uri)
                .header("content-type", "application/json")
                .body(Body::from(body.to_owned()))
                .expect("request"),
        )
        .await
        .expect("response");
    response.status()
}

async fn patch_json(app: Router, uri: &str, body: &str) -> StatusCode {
    let response = app
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(uri)
                .header("content-type", "application/json")
                .body(Body::from(body.to_owned()))
                .expect("request"),
        )
        .await
        .expect("response");
    response.status()
}

async fn delete_route(app: Router, uri: &str) -> StatusCode {
    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(uri)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    response.status()
}

#[tokio::test]
async fn reminder_mutations_return_503_without_eventkit() {
    let fixture = RemindersFixtureDb::seeded().await.expect("fixture");
    let pool = connect_pool(fixture.path()).await.expect("pool");
    let app = router(AppState::with_eventkit(None, Some(pool), None, None, None));

    assert_eq!(
        post_json(
            app.clone(),
            "/v1/reminder-lists/00000000-0000-0000-0000-000000000001/reminders",
            r#"{"title":"Test"}"#
        )
        .await,
        StatusCode::SERVICE_UNAVAILABLE
    );
    assert_eq!(
        patch_json(
            app.clone(),
            "/v1/reminders/00000000-0000-0000-0000-000000000001",
            r#"{"title":"Updated"}"#
        )
        .await,
        StatusCode::SERVICE_UNAVAILABLE
    );
    assert_eq!(
        delete_route(app, "/v1/reminders/00000000-0000-0000-0000-000000000001").await,
        StatusCode::SERVICE_UNAVAILABLE
    );
}

#[tokio::test]
async fn event_mutations_return_503_without_eventkit() {
    let fixture = CalendarFixtureDb::seeded().await.expect("fixture");
    let pool = connect_pool(fixture.path()).await.expect("pool");
    let app = router(AppState::with_eventkit(None, None, None, Some(pool), None));

    assert_eq!(
        post_json(
            app.clone(),
            &format!("/v1/calendars/{SEED_CALENDAR_ID}/events"),
            r#"{"summary":"Test","start":1705320000,"end":1705323600}"#
        )
        .await,
        StatusCode::SERVICE_UNAVAILABLE
    );
    assert_eq!(
        patch_json(
            app.clone(),
            "/v1/events/00000000-0000-0000-0000-000000000001",
            r#"{"summary":"Updated"}"#
        )
        .await,
        StatusCode::SERVICE_UNAVAILABLE
    );
    assert_eq!(
        delete_route(app, "/v1/events/00000000-0000-0000-0000-000000000001").await,
        StatusCode::SERVICE_UNAVAILABLE
    );
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
async fn contact_mutations_return_503_without_contacts_store() {
    let fixture = ContactsFixtureDb::seeded().await.expect("fixture");
    let pool = connect_pool(fixture.path()).await.expect("pool");
    let app = contacts_app(pool);

    assert_eq!(
        post_json(
            app.clone(),
            &format!("/v1/containers/{SEED_CONTAINER_ID}/contacts"),
            r#"{"given_name":"Test","family_name":"User"}"#
        )
        .await,
        StatusCode::SERVICE_UNAVAILABLE
    );
    assert_eq!(
        patch_json(
            app.clone(),
            "/v1/contacts/aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa",
            r#"{"given_name":"Updated"}"#
        )
        .await,
        StatusCode::SERVICE_UNAVAILABLE
    );
    assert_eq!(
        delete_route(
            app.clone(),
            "/v1/contacts/aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa"
        )
        .await,
        StatusCode::SERVICE_UNAVAILABLE
    );
    assert_eq!(
        post_json(
            app.clone(),
            &format!("/v1/containers/{SEED_CONTAINER_ID}/groups"),
            r#"{"name":"Test Group"}"#
        )
        .await,
        StatusCode::SERVICE_UNAVAILABLE
    );
    assert_eq!(
        patch_json(
            app.clone(),
            &format!("/v1/groups/{SEED_GROUP_ID}"),
            r#"{"name":"Renamed"}"#
        )
        .await,
        StatusCode::SERVICE_UNAVAILABLE
    );
    assert_eq!(
        delete_route(app.clone(), &format!("/v1/groups/{SEED_GROUP_ID}")).await,
        StatusCode::SERVICE_UNAVAILABLE
    );
    assert_eq!(
        post_json(
            app.clone(),
            &format!("/v1/groups/{SEED_GROUP_ID}/contacts/aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa"),
            "{}"
        )
        .await,
        StatusCode::SERVICE_UNAVAILABLE
    );
    assert_eq!(
        delete_route(
            app,
            &format!("/v1/groups/{SEED_GROUP_ID}/contacts/aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa")
        )
        .await,
        StatusCode::SERVICE_UNAVAILABLE
    );
}
