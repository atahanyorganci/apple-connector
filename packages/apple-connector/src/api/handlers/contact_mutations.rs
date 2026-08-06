use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};

use crate::{
    api::{
        contacts::{require_contacts_access, require_contacts_sources},
        contacts_convert::{
            container_hint, create_contact_input, create_group_input, map_contacts_error,
            update_contact_input, update_group_input,
        },
        dto::contacts::{
            CreateContactRequest, CreateGroupRequest, UpdateContactRequest, UpdateGroupRequest,
        },
        error::{ApiError, ErrorResponse},
        hydrate::SyncPendingContactDetailDto,
        params::{ContactGroupPath, ContactIdPath, ContainerIdPath, GroupIdPath},
        router::AppState,
    },
    db::run_timed_query,
};

/// Create a contact in a container
#[utoipa::path(
    post,
    path = "/v1/containers/{container_id}/contacts",
    operation_id = "createContact",
    tag = "contacts",
    params(ContainerIdPath),
    request_body = CreateContactRequest,
    responses(
        (status = 201, description = "Contact created", body = SyncPendingContactDetailDto),
        (status = 403, description = "Read-only container", body = ErrorResponse),
        (status = 503, description = "Contacts databases or Contacts framework unavailable", body = ErrorResponse),
    )
)]
pub async fn create_contact(
    State(state): State<AppState>,
    Path(path): Path<ContainerIdPath>,
    Json(request): Json<CreateContactRequest>,
) -> Result<(StatusCode, Json<SyncPendingContactDetailDto>), ApiError> {
    let sources = require_contacts_sources(&state.contacts_sources)?;
    let store = require_contacts_access(&state).await?;
    let container_id = path.container_id.as_str();

    let container = run_timed_query(|| async { sources.get_container(container_id).await })
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?
        .ok_or_else(|| ApiError::not_found("container not found"))?;

    if container.read_only {
        return Err(ApiError::forbidden("cannot write to read-only container"));
    }

    let metadata =
        run_timed_query(|| async { sources.get_container_resolve_metadata(container_id).await })
            .await
            .map_err(|error| ApiError::internal(error.to_string()))?
            .ok_or_else(|| ApiError::not_found("container not found"))?;

    let saved = store
        .create_contact(
            container_hint(&container, metadata),
            create_contact_input(request),
        )
        .await
        .map_err(map_contacts_error)?;

    let response =
        crate::api::hydrate::hydrate_contact(&state.contacts_sources, &saved.identifier).await?;
    Ok((StatusCode::CREATED, Json(response)))
}

/// Update a contact
#[utoipa::path(
    patch,
    path = "/v1/contacts/{contact_id}",
    operation_id = "updateContact",
    tag = "contacts",
    params(ContactIdPath),
    request_body = UpdateContactRequest,
    responses(
        (status = 200, description = "Contact updated", body = SyncPendingContactDetailDto),
        (status = 404, description = "Contact not found", body = ErrorResponse),
        (status = 503, description = "Contacts databases or Contacts framework unavailable", body = ErrorResponse),
    )
)]
pub async fn update_contact(
    State(state): State<AppState>,
    Path(path): Path<ContactIdPath>,
    Json(request): Json<UpdateContactRequest>,
) -> Result<Json<SyncPendingContactDetailDto>, ApiError> {
    let sources = require_contacts_sources(&state.contacts_sources)?;
    let store = require_contacts_access(&state).await?;
    let contact_id = path.contact_id.as_str();

    let framework_id =
        run_timed_query(|| async { sources.get_contact_framework_id(contact_id).await })
            .await
            .map_err(|error| ApiError::internal(error.to_string()))?
            .ok_or_else(|| ApiError::not_found("contact not found"))?;

    let saved = store
        .update_contact(&framework_id, update_contact_input(request))
        .await
        .map_err(map_contacts_error)?;

    let response =
        crate::api::hydrate::hydrate_contact(&state.contacts_sources, &saved.identifier).await?;
    Ok(Json(response))
}

/// Delete a contact
#[utoipa::path(
    delete,
    path = "/v1/contacts/{contact_id}",
    operation_id = "deleteContact",
    tag = "contacts",
    params(ContactIdPath),
    responses(
        (status = 204, description = "Contact deleted"),
        (status = 404, description = "Contact not found", body = ErrorResponse),
        (status = 503, description = "Contacts databases or Contacts framework unavailable", body = ErrorResponse),
    )
)]
pub async fn delete_contact(
    State(state): State<AppState>,
    Path(path): Path<ContactIdPath>,
) -> Result<StatusCode, ApiError> {
    let sources = require_contacts_sources(&state.contacts_sources)?;
    let store = require_contacts_access(&state).await?;
    let contact_id = path.contact_id.as_str();

    let framework_id =
        run_timed_query(|| async { sources.get_contact_framework_id(contact_id).await })
            .await
            .map_err(|error| ApiError::internal(error.to_string()))?
            .ok_or_else(|| ApiError::not_found("contact not found"))?;

    store
        .delete_contact(&framework_id)
        .await
        .map_err(map_contacts_error)?;

    Ok(StatusCode::NO_CONTENT)
}

/// Create a group in a container
#[utoipa::path(
    post,
    path = "/v1/containers/{container_id}/groups",
    operation_id = "createGroup",
    tag = "groups",
    params(ContainerIdPath),
    request_body = CreateGroupRequest,
    responses(
        (status = 201, description = "Group created", body = crate::api::dto::contacts::GroupDetailDto),
        (status = 403, description = "Read-only container", body = ErrorResponse),
        (status = 503, description = "Contacts databases or Contacts framework unavailable", body = ErrorResponse),
    )
)]
pub async fn create_group(
    State(state): State<AppState>,
    Path(path): Path<ContainerIdPath>,
    Json(request): Json<CreateGroupRequest>,
) -> Result<(StatusCode, Json<crate::api::dto::contacts::GroupDetailDto>), ApiError> {
    use crate::api::dto::contacts_convert::group_detail_to_dto;

    let sources = require_contacts_sources(&state.contacts_sources)?;
    let store = require_contacts_access(&state).await?;
    let container_id = path.container_id.as_str();

    let container = run_timed_query(|| async { sources.get_container(container_id).await })
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?
        .ok_or_else(|| ApiError::not_found("container not found"))?;

    if container.read_only {
        return Err(ApiError::forbidden("cannot write to read-only container"));
    }

    let metadata =
        run_timed_query(|| async { sources.get_container_resolve_metadata(container_id).await })
            .await
            .map_err(|error| ApiError::internal(error.to_string()))?
            .ok_or_else(|| ApiError::not_found("container not found"))?;

    let saved = store
        .create_group(
            container_hint(&container, metadata),
            create_group_input(request),
        )
        .await
        .map_err(map_contacts_error)?;

    let group_api_id = crate::contacts::api_id_from_unique_id(&saved.identifier);
    for _ in 0..5 {
        if let Some(group) = run_timed_query(|| async { sources.get_group(&group_api_id).await })
            .await
            .map_err(|error| ApiError::internal(error.to_string()))?
        {
            return Ok((StatusCode::CREATED, Json(group_detail_to_dto(&group))));
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    Err(ApiError::service_unavailable(
        "group created but SQLite read path has not caught up yet",
    ))
}

/// Update a group
#[utoipa::path(
    patch,
    path = "/v1/groups/{group_id}",
    operation_id = "updateGroup",
    tag = "groups",
    params(GroupIdPath),
    request_body = UpdateGroupRequest,
    responses(
        (status = 200, description = "Group updated", body = crate::api::dto::contacts::GroupDetailDto),
        (status = 404, description = "Group not found", body = ErrorResponse),
        (status = 503, description = "Contacts databases or Contacts framework unavailable", body = ErrorResponse),
    )
)]
pub async fn update_group(
    State(state): State<AppState>,
    Path(path): Path<GroupIdPath>,
    Json(request): Json<UpdateGroupRequest>,
) -> Result<Json<crate::api::dto::contacts::GroupDetailDto>, ApiError> {
    use crate::api::dto::contacts_convert::group_detail_to_dto;

    let sources = require_contacts_sources(&state.contacts_sources)?;
    let store = require_contacts_access(&state).await?;
    let group_id = path.group_id.as_str();

    let framework_id = run_timed_query(|| async { sources.get_group_framework_id(group_id).await })
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?
        .ok_or_else(|| ApiError::not_found("group not found"))?;

    let saved = store
        .update_group(&framework_id, update_group_input(request))
        .await
        .map_err(map_contacts_error)?;

    let group_api_id = crate::contacts::api_id_from_unique_id(&saved.identifier);
    let group = run_timed_query(|| async { sources.get_group(&group_api_id).await })
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?
        .ok_or_else(|| ApiError::not_found("group not found"))?;
    Ok(Json(group_detail_to_dto(&group)))
}

/// Delete a group
#[utoipa::path(
    delete,
    path = "/v1/groups/{group_id}",
    operation_id = "deleteGroup",
    tag = "groups",
    params(GroupIdPath),
    responses(
        (status = 204, description = "Group deleted"),
        (status = 404, description = "Group not found", body = ErrorResponse),
        (status = 503, description = "Contacts databases or Contacts framework unavailable", body = ErrorResponse),
    )
)]
pub async fn delete_group(
    State(state): State<AppState>,
    Path(path): Path<GroupIdPath>,
) -> Result<StatusCode, ApiError> {
    let sources = require_contacts_sources(&state.contacts_sources)?;
    let store = require_contacts_access(&state).await?;
    let group_id = path.group_id.as_str();

    let framework_id = run_timed_query(|| async { sources.get_group_framework_id(group_id).await })
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?
        .ok_or_else(|| ApiError::not_found("group not found"))?;

    store
        .delete_group(&framework_id)
        .await
        .map_err(map_contacts_error)?;

    Ok(StatusCode::NO_CONTENT)
}

/// Add a contact to a group
#[utoipa::path(
    post,
    path = "/v1/groups/{group_id}/contacts/{contact_id}",
    operation_id = "addContactToGroup",
    tag = "groups",
    params(ContactGroupPath),
    responses(
        (status = 204, description = "Contact added to group"),
        (status = 404, description = "Contact or group not found", body = ErrorResponse),
        (status = 503, description = "Contacts databases or Contacts framework unavailable", body = ErrorResponse),
    )
)]
pub async fn add_contact_to_group(
    State(state): State<AppState>,
    Path(path): Path<ContactGroupPath>,
) -> Result<StatusCode, ApiError> {
    let sources = require_contacts_sources(&state.contacts_sources)?;
    let store = require_contacts_access(&state).await?;

    let group_framework_id =
        run_timed_query(|| async { sources.get_group_framework_id(path.group_id.as_str()).await })
            .await
            .map_err(|error| ApiError::internal(error.to_string()))?
            .ok_or_else(|| ApiError::not_found("group not found"))?;

    let contact_framework_id = run_timed_query(|| async {
        sources
            .get_contact_framework_id(path.contact_id.as_str())
            .await
    })
    .await
    .map_err(|error| ApiError::internal(error.to_string()))?
    .ok_or_else(|| ApiError::not_found("contact not found"))?;

    store
        .add_contact_to_group(&contact_framework_id, &group_framework_id)
        .await
        .map_err(map_contacts_error)?;

    Ok(StatusCode::NO_CONTENT)
}

/// Remove a contact from a group
#[utoipa::path(
    delete,
    path = "/v1/groups/{group_id}/contacts/{contact_id}",
    operation_id = "removeContactFromGroup",
    tag = "groups",
    params(ContactGroupPath),
    responses(
        (status = 204, description = "Contact removed from group"),
        (status = 404, description = "Contact or group not found", body = ErrorResponse),
        (status = 503, description = "Contacts databases or Contacts framework unavailable", body = ErrorResponse),
    )
)]
pub async fn remove_contact_from_group(
    State(state): State<AppState>,
    Path(path): Path<ContactGroupPath>,
) -> Result<StatusCode, ApiError> {
    let sources = require_contacts_sources(&state.contacts_sources)?;
    let store = require_contacts_access(&state).await?;

    let group_framework_id =
        run_timed_query(|| async { sources.get_group_framework_id(path.group_id.as_str()).await })
            .await
            .map_err(|error| ApiError::internal(error.to_string()))?
            .ok_or_else(|| ApiError::not_found("group not found"))?;

    let contact_framework_id = run_timed_query(|| async {
        sources
            .get_contact_framework_id(path.contact_id.as_str())
            .await
    })
    .await
    .map_err(|error| ApiError::internal(error.to_string()))?
    .ok_or_else(|| ApiError::not_found("contact not found"))?;

    store
        .remove_contact_from_group(&contact_framework_id, &group_framework_id)
        .await
        .map_err(map_contacts_error)?;

    Ok(StatusCode::NO_CONTENT)
}
