use axum::{
    Json,
    extract::{Query, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
};

use crate::{
    api::{
        contacts::{
            contact_detail_carddav, contact_detail_vcard, contact_page_carddav, contact_page_vcard,
            require_contacts_sources,
        },
        dto::{
            ContactDetailDto, ContactPageDto,
            contacts_convert::{contact_detail_to_dto, contact_page_to_dto},
        },
        error::{ApiError, ErrorResponse},
        params::{ContactIdPath, ContactListParams},
        router::AppState,
    },
    contacts::{ContactSummary, Page},
    db::run_timed_query,
};

/// List contacts globally as JSON
#[utoipa::path(
    get,
    path = "/v1/contacts",
    operation_id = "listContacts",
    tag = "contacts",
    params(ContactListParams),
    responses(
        (status = 200, description = "Paginated contacts", body = ContactPageDto,
            content_type = "application/json"),
        (status = 400, description = "Invalid query parameters", body = ErrorResponse),
        (status = 503, description = "Contacts databases are unavailable", body = ErrorResponse),
    )
)]
pub async fn list_contacts(
    State(state): State<AppState>,
    Query(params): Query<ContactListParams>,
) -> Result<Json<ContactPageDto>, ApiError> {
    let sources = require_contacts_sources(&state.contacts_sources)?;
    let page = fetch_contact_page(sources, &params).await?;
    Ok(Json(contact_page_to_dto(page, params.validated_limit()?)))
}

/// List contacts globally as vCard
#[utoipa::path(
    get,
    path = "/v1/contacts/vcard",
    operation_id = "listContactsVcard",
    tag = "contacts",
    params(ContactListParams),
    responses(
        (status = 200, description = "vCard feed", content_type = "text/vcard"),
        (status = 400, description = "Invalid query parameters", body = ErrorResponse),
        (status = 503, description = "Contacts databases are unavailable", body = ErrorResponse),
    )
)]
pub async fn list_contacts_vcard(
    State(state): State<AppState>,
    Query(params): Query<ContactListParams>,
) -> Result<Response, ApiError> {
    let sources = require_contacts_sources(&state.contacts_sources)?;
    let page = fetch_contact_page(sources, &params).await?;
    let details = sources
        .hydrate_contact_summaries(page.items)
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;
    contact_page_vcard(&details)
}

/// List contacts globally as CardDAV XML
#[utoipa::path(
    get,
    path = "/v1/contacts/carddav",
    operation_id = "listContactsCarddav",
    tag = "contacts",
    params(ContactListParams),
    responses(
        (status = 200, description = "CardDAV multistatus", content_type = "application/carddav+xml"),
        (status = 400, description = "Invalid query parameters", body = ErrorResponse),
        (status = 503, description = "Contacts databases are unavailable", body = ErrorResponse),
    )
)]
pub async fn list_contacts_carddav(
    State(state): State<AppState>,
    Query(params): Query<ContactListParams>,
) -> Result<Response, ApiError> {
    let sources = require_contacts_sources(&state.contacts_sources)?;
    let page = fetch_contact_page(sources, &params).await?;
    let details = sources
        .hydrate_contact_summaries(page.items)
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;
    contact_page_carddav(&details)
}

/// Search contacts
#[utoipa::path(
    get,
    path = "/v1/contacts/search",
    operation_id = "searchContacts",
    tag = "contacts",
    params(ContactListParams),
    responses(
        (status = 200, description = "Matching contacts", body = ContactPageDto),
        (status = 400, description = "Invalid query parameters", body = ErrorResponse),
        (status = 503, description = "Contacts databases are unavailable", body = ErrorResponse),
    )
)]
pub async fn search_contacts(
    State(state): State<AppState>,
    Query(params): Query<ContactListParams>,
) -> Result<Json<ContactPageDto>, ApiError> {
    let sources = require_contacts_sources(&state.contacts_sources)?;
    let limit = params.validated_limit()?;
    let q = params.validated_search_query()?;
    let items = run_timed_query(|| async { sources.search_contacts(&q, limit).await })
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;
    Ok(Json(contact_page_to_dto(
        Page {
            items,
            has_more: false,
            next_cursor: None,
        },
        limit,
    )))
}

/// Get a contact as JSON
#[utoipa::path(
    get,
    path = "/v1/contacts/{contact_id}",
    operation_id = "getContact",
    tag = "contacts",
    params(ContactIdPath),
    responses(
        (status = 200, description = "Contact detail", body = ContactDetailDto,
            content_type = "application/json"),
        (status = 404, description = "Contact not found", body = ErrorResponse),
        (status = 503, description = "Contacts databases are unavailable", body = ErrorResponse),
    )
)]
pub async fn get_contact(
    State(state): State<AppState>,
    axum::extract::Path(path): axum::extract::Path<ContactIdPath>,
) -> Result<Json<ContactDetailDto>, ApiError> {
    let sources = require_contacts_sources(&state.contacts_sources)?;
    let contact_id = path.validated()?;
    let contact = fetch_contact_detail(sources, contact_id.as_str()).await?;
    Ok(Json(contact_detail_to_dto(&contact)))
}

/// Get a contact as vCard
#[utoipa::path(
    get,
    path = "/v1/contacts/{contact_id}/vcard",
    operation_id = "getContactVcard",
    tag = "contacts",
    params(ContactIdPath),
    responses(
        (status = 200, description = "vCard document", content_type = "text/vcard"),
        (status = 404, description = "Contact not found", body = ErrorResponse),
        (status = 503, description = "Contacts databases are unavailable", body = ErrorResponse),
    )
)]
pub async fn get_contact_vcard(
    State(state): State<AppState>,
    axum::extract::Path(path): axum::extract::Path<ContactIdPath>,
) -> Result<Response, ApiError> {
    let sources = require_contacts_sources(&state.contacts_sources)?;
    let contact_id = path.validated()?;
    let contact = fetch_contact_detail(sources, contact_id.as_str()).await?;
    contact_detail_vcard(&contact)
}

/// Get a contact as CardDAV XML
#[utoipa::path(
    get,
    path = "/v1/contacts/{contact_id}/carddav",
    operation_id = "getContactCarddav",
    tag = "contacts",
    params(ContactIdPath),
    responses(
        (status = 200, description = "CardDAV resource", content_type = "application/carddav+xml"),
        (status = 404, description = "Contact not found", body = ErrorResponse),
        (status = 503, description = "Contacts databases are unavailable", body = ErrorResponse),
    )
)]
pub async fn get_contact_carddav(
    State(state): State<AppState>,
    axum::extract::Path(path): axum::extract::Path<ContactIdPath>,
) -> Result<Response, ApiError> {
    let sources = require_contacts_sources(&state.contacts_sources)?;
    let contact_id = path.validated()?;
    let contact = fetch_contact_detail(sources, contact_id.as_str()).await?;
    contact_detail_carddav(&contact)
}

/// Get a contact photo
#[utoipa::path(
    get,
    path = "/v1/contacts/{contact_id}/photo",
    operation_id = "getContactPhoto",
    tag = "contacts",
    params(ContactIdPath),
    responses(
        (status = 200, description = "Contact photo bytes"),
        (status = 404, description = "Contact or photo not found", body = ErrorResponse),
        (status = 503, description = "Contacts databases are unavailable", body = ErrorResponse),
    )
)]
pub async fn get_contact_photo(
    State(state): State<AppState>,
    axum::extract::Path(path): axum::extract::Path<ContactIdPath>,
) -> Result<Response, ApiError> {
    let sources = require_contacts_sources(&state.contacts_sources)?;
    let contact_id = path.validated()?;
    let photo = run_timed_query(|| async { sources.get_contact_photo(contact_id.as_str()).await })
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?
        .ok_or_else(|| ApiError::not_found("Contact photo not found"))?;

    let (bytes, image_type) = photo;
    let content_type = image_type
        .as_deref()
        .map(mime_from_image_type)
        .unwrap_or("application/octet-stream");

    Ok((
        StatusCode::OK,
        [(header::CONTENT_TYPE, content_type)],
        bytes,
    )
        .into_response())
}

async fn fetch_contact_page(
    sources: &crate::contacts::ContactsSources,
    params: &ContactListParams,
) -> Result<Page<ContactSummary>, ApiError> {
    let limit = params.validated_limit()?;
    params.validated_cursor()?;
    let filters = params.validated_filters()?;
    let cursor = params
        .cursor
        .as_deref()
        .map(crate::api::cursor::decode::<crate::api::cursor::ContactListCursor>)
        .transpose()?;
    run_timed_query(|| async { sources.list_contacts(limit, cursor, &filters).await })
        .await
        .map_err(|error| ApiError::internal(error.to_string()))
}

async fn fetch_contact_detail(
    sources: &crate::contacts::ContactsSources,
    contact_id: &str,
) -> Result<crate::contacts::ContactDetail, ApiError> {
    run_timed_query(|| async { sources.get_contact(contact_id).await })
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?
        .ok_or_else(|| ApiError::not_found("Contact not found"))
}

fn mime_from_image_type(image_type: &str) -> &'static str {
    match image_type.to_ascii_lowercase().as_str() {
        "jpeg" | "jpg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "heic" => "image/heic",
        _ => "application/octet-stream",
    }
}

#[allow(dead_code)]
pub(crate) fn preferred_format(headers: &HeaderMap) -> ContactFormat {
    if accepts_carddav(headers) {
        ContactFormat::CardDav
    } else if accepts_vcard(headers) {
        ContactFormat::VCard
    } else {
        ContactFormat::Json
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ContactFormat {
    Json,
    VCard,
    CardDav,
}

fn accepts_vcard(headers: &HeaderMap) -> bool {
    header_accepts(headers, "text/vcard") || header_accepts(headers, "text/x-vcard")
}

fn accepts_carddav(headers: &HeaderMap) -> bool {
    header_accepts(headers, "application/carddav+xml")
}

fn header_accepts(headers: &HeaderMap, mime: &str) -> bool {
    headers
        .get(header::ACCEPT)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| {
            value.split(',').any(|part| {
                part.split(';')
                    .next()
                    .is_some_and(|candidate| candidate.trim().eq_ignore_ascii_case(mime))
            })
        })
}
