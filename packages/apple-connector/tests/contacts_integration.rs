//! End-to-end HTTP integration coverage for the read-only Contacts API.

use std::collections::HashMap;

use apple_connector::{
    AppState, connect_pool,
    contacts::ContactsSources,
    fixtures::{ContactsFixtureDb, SEED_CONTACT_ID, SEED_CONTAINER_ID, SEED_GROUP_ID},
    router,
};
use axum::{Router, body::Body};
use http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

async fn response_json(app: Router, uri: &str) -> (StatusCode, serde_json::Value) {
    let response = app
        .oneshot(
            Request::builder()
                .uri(uri)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    let status = response.status();
    let body = response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes();
    let payload = serde_json::from_slice(&body)
        .unwrap_or(serde_json::json!({ "raw": String::from_utf8_lossy(&body) }));
    (status, payload)
}

async fn response_text(app: Router, uri: &str) -> (StatusCode, String) {
    let response = app
        .oneshot(
            Request::builder()
                .uri(uri)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    let status = response.status();
    let body = response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes();
    (status, String::from_utf8_lossy(&body).into_owned())
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
async fn integration_containers_groups_and_contacts() {
    let fixture = ContactsFixtureDb::seeded().await.expect("fixture");
    let pool = connect_pool(fixture.path()).await.expect("pool");
    let app = contacts_app(pool);

    let (status, payload) = response_json(app.clone(), "/healthz").await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(payload["contacts"], "ok");

    let (status, containers) = response_json(app.clone(), "/v1/containers").await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        containers["items"]
            .as_array()
            .is_some_and(|items| { items.iter().any(|item| item["id"] == SEED_CONTAINER_ID) })
    );

    let (status, groups) = response_json(app.clone(), "/v1/groups?limit=10").await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        groups["items"]
            .as_array()
            .is_some_and(|items| items.iter().any(|item| item["id"] == SEED_GROUP_ID))
    );

    let (status, contacts) = response_json(app.clone(), "/v1/contacts?limit=10").await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        contacts["items"]
            .as_array()
            .is_some_and(|items| { items.iter().any(|item| item["id"] == SEED_CONTACT_ID) })
    );

    let (status, detail) =
        response_json(app.clone(), &format!("/v1/contacts/{SEED_CONTACT_ID}")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(detail["id"], SEED_CONTACT_ID);
    assert_eq!(detail["first_name"], "Jane");
    assert!(
        detail["phones"]
            .as_array()
            .is_some_and(|items| !items.is_empty())
    );
}

#[tokio::test]
async fn integration_contacts_search_vcard_and_carddav() {
    let fixture = ContactsFixtureDb::seeded().await.expect("fixture");
    let pool = connect_pool(fixture.path()).await.expect("pool");
    let app = contacts_app(pool);

    let (status, search) = response_json(app.clone(), "/v1/contacts/search?q=Jane&limit=10").await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        search["items"]
            .as_array()
            .is_some_and(|items| !items.is_empty())
    );

    let (status, vcard) = response_text(
        app.clone(),
        &format!("/v1/contacts/{SEED_CONTACT_ID}/vcard"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(vcard.contains("BEGIN:VCARD"));
    assert!(vcard.contains("Jane"));

    let (status, carddav) = response_text(
        app.clone(),
        &format!("/v1/contacts/{SEED_CONTACT_ID}/carddav"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(carddav.contains("address-data"));

    let (status, list_vcard) = response_text(app.clone(), "/v1/contacts/vcard?limit=10").await;
    assert_eq!(status, StatusCode::OK);
    assert!(list_vcard.contains("BEGIN:VCARD"));

    let (status, group_contacts) = response_json(
        app.clone(),
        &format!("/v1/groups/{SEED_GROUP_ID}/contacts?limit=10"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        group_contacts["items"]
            .as_array()
            .is_some_and(|items| !items.is_empty())
    );
}

#[tokio::test]
async fn integration_contacts_unavailable_without_sources() {
    let app = router(AppState::new(None, None, None, None));
    let (status, _) = response_json(app, "/v1/contacts").await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
#[ignore = "requires macOS Contacts permission and live AddressBook sources"]
async fn integration_contacts_mutations_on_macos() {
    use apple_contacts::{
        ContactsStore, ContainerResolveHint, CreateContactInput, UpdateContactInput,
    };

    let store = ContactsStore::new().expect("Contacts store");
    store.request_access().await.expect("Contacts access");

    let container = ContainerResolveHint {
        api_id: "integration-test-container".into(),
        external_id: None,
        name: Some(
            std::env::var("APPLE_CONNECTOR_TEST_CONTACTS_CONTAINER")
                .unwrap_or_else(|_| "Contacts".into()),
        ),
        read_only: false,
    };

    let saved = store
        .create_contact(
            container,
            CreateContactInput {
                given_name: Some("Connector".into()),
                family_name: Some("Integration".into()),
                middle_name: None,
                nickname: None,
                organization_name: None,
                job_title: None,
                department_name: None,
                note: Some("created by ignored test".into()),
                phone_numbers: Vec::new(),
                email_addresses: Vec::new(),
                postal_addresses: Vec::new(),
                url_addresses: Vec::new(),
            },
        )
        .await
        .expect("create contact");

    store
        .update_contact(
            &saved.identifier,
            UpdateContactInput {
                given_name: Some("Connector".into()),
                family_name: Some("Updated".into()),
                middle_name: None,
                nickname: None,
                organization_name: None,
                job_title: None,
                department_name: None,
                note: Some(Some("updated by ignored test".into())),
                phone_numbers: None,
                email_addresses: None,
                postal_addresses: None,
                url_addresses: None,
            },
        )
        .await
        .expect("update contact");

    store
        .delete_contact(&saved.identifier)
        .await
        .expect("delete contact");
}
