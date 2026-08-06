use axum::{
    Json,
    extract::{Query, State},
    response::Response,
};

use crate::{
    api::{
        contacts::{contact_page_carddav, contact_page_vcard, require_contacts_sources},
        dto::{
            ContactPageDto, GroupDetailDto, GroupPageDto,
            contacts_convert::{contact_page_to_dto, group_detail_to_dto, group_page_to_dto},
        },
        error::{ApiError, ErrorResponse},
        params::{GroupIdPath, PageParams},
        router::AppState,
    },
    db::run_timed_query,
};

/// List contact groups
#[utoipa::path(
    get,
    path = "/v1/groups",
    operation_id = "listGroups",
    tag = "groups",
    params(PageParams),
    responses(
        (status = 200, description = "Paginated groups", body = GroupPageDto),
        (status = 400, description = "Invalid query parameters", body = ErrorResponse),
        (status = 503, description = "Contacts databases are unavailable", body = ErrorResponse),
    )
)]
pub async fn list_groups(
    State(state): State<AppState>,
    Query(params): Query<PageParams>,
) -> Result<Json<GroupPageDto>, ApiError> {
    let sources = require_contacts_sources(&state.contacts_sources)?;
    let limit = params.validated_limit()?;
    params.validated_cursor()?;
    let cursor = params
        .cursor
        .as_deref()
        .map(crate::api::cursor::decode::<crate::api::cursor::ContactListCursor>)
        .transpose()?;
    let page = run_timed_query(|| async { sources.list_groups(limit, cursor).await })
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;
    Ok(Json(group_page_to_dto(
        page.items,
        page.has_more,
        page.next_cursor,
        limit,
    )))
}

/// Get a contact group
#[utoipa::path(
    get,
    path = "/v1/groups/{group_id}",
    operation_id = "getGroup",
    tag = "groups",
    params(GroupIdPath),
    responses(
        (status = 200, description = "Group detail", body = GroupDetailDto),
        (status = 404, description = "Group not found", body = ErrorResponse),
        (status = 503, description = "Contacts databases are unavailable", body = ErrorResponse),
    )
)]
pub async fn get_group(
    State(state): State<AppState>,
    axum::extract::Path(path): axum::extract::Path<GroupIdPath>,
) -> Result<Json<GroupDetailDto>, ApiError> {
    let sources = require_contacts_sources(&state.contacts_sources)?;
    let group = run_timed_query(|| async { sources.get_group(path.group_id.as_str()).await })
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?
        .ok_or_else(|| ApiError::not_found("Group not found"))?;
    Ok(Json(group_detail_to_dto(&group)))
}

/// List contacts in a group as JSON
#[utoipa::path(
    get,
    path = "/v1/groups/{group_id}/contacts",
    operation_id = "listGroupContacts",
    tag = "contacts",
    params(GroupIdPath, PageParams),
    responses(
        (status = 200, description = "Paginated contacts", body = ContactPageDto,
            content_type = "application/json"),
        (status = 400, description = "Invalid query parameters", body = ErrorResponse),
        (status = 404, description = "Group not found", body = ErrorResponse),
        (status = 503, description = "Contacts databases are unavailable", body = ErrorResponse),
    )
)]
pub async fn list_group_contacts(
    State(state): State<AppState>,
    axum::extract::Path(path): axum::extract::Path<GroupIdPath>,
    Query(params): Query<PageParams>,
) -> Result<Json<ContactPageDto>, ApiError> {
    let sources = require_contacts_sources(&state.contacts_sources)?;
    let page = fetch_group_contact_page(sources, path.group_id.as_str(), &params).await?;
    Ok(Json(contact_page_to_dto(page, params.validated_limit()?)))
}

/// List contacts in a group as vCard
#[utoipa::path(
    get,
    path = "/v1/groups/{group_id}/contacts/vcard",
    operation_id = "listGroupContactsVcard",
    tag = "contacts",
    params(GroupIdPath, PageParams),
    responses(
        (status = 200, description = "vCard feed", content_type = "text/vcard"),
        (status = 400, description = "Invalid query parameters", body = ErrorResponse),
        (status = 404, description = "Group not found", body = ErrorResponse),
        (status = 503, description = "Contacts databases are unavailable", body = ErrorResponse),
    )
)]
pub async fn list_group_contacts_vcard(
    State(state): State<AppState>,
    axum::extract::Path(path): axum::extract::Path<GroupIdPath>,
    Query(params): Query<PageParams>,
) -> Result<Response, ApiError> {
    let sources = require_contacts_sources(&state.contacts_sources)?;
    let page = fetch_group_contact_page(sources, path.group_id.as_str(), &params).await?;
    let details = hydrate_contact_summaries(sources, page.items).await?;
    contact_page_vcard(&details)
}

/// List contacts in a group as CardDAV XML
#[utoipa::path(
    get,
    path = "/v1/groups/{group_id}/contacts/carddav",
    operation_id = "listGroupContactsCarddav",
    tag = "contacts",
    params(GroupIdPath, PageParams),
    responses(
        (status = 200, description = "CardDAV multistatus", content_type = "application/carddav+xml"),
        (status = 400, description = "Invalid query parameters", body = ErrorResponse),
        (status = 404, description = "Group not found", body = ErrorResponse),
        (status = 503, description = "Contacts databases are unavailable", body = ErrorResponse),
    )
)]
pub async fn list_group_contacts_carddav(
    State(state): State<AppState>,
    axum::extract::Path(path): axum::extract::Path<GroupIdPath>,
    Query(params): Query<PageParams>,
) -> Result<Response, ApiError> {
    let sources = require_contacts_sources(&state.contacts_sources)?;
    let page = fetch_group_contact_page(sources, path.group_id.as_str(), &params).await?;
    let details = hydrate_contact_summaries(sources, page.items).await?;
    contact_page_carddav(&details)
}

async fn fetch_group_contact_page(
    sources: &crate::contacts::ContactsSources,
    group_id: &str,
    params: &PageParams,
) -> Result<crate::contacts::Page<crate::contacts::ContactSummary>, ApiError> {
    let limit = params.validated_limit()?;
    params.validated_cursor()?;
    let group = run_timed_query(|| async { sources.get_group(group_id).await })
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?
        .ok_or_else(|| ApiError::not_found("Group not found"))?;
    let _ = group;
    let cursor = params
        .cursor
        .as_deref()
        .map(crate::api::cursor::decode::<crate::api::cursor::GroupContactCursor>)
        .transpose()?;
    run_timed_query(|| async { sources.list_group_contacts(group_id, limit, cursor).await })
        .await
        .map_err(|error| ApiError::internal(error.to_string()))
}

async fn hydrate_contact_summaries(
    sources: &crate::contacts::ContactsSources,
    summaries: Vec<crate::contacts::ContactSummary>,
) -> Result<Vec<crate::contacts::ContactDetail>, ApiError> {
    let mut details = Vec::with_capacity(summaries.len());
    for summary in summaries {
        let contact = run_timed_query(|| async { sources.get_contact(summary.id.as_str()).await })
            .await
            .map_err(|error| ApiError::internal(error.to_string()))?
            .ok_or_else(|| ApiError::not_found("Contact not found"))?;
        details.push(contact);
    }
    Ok(details)
}
