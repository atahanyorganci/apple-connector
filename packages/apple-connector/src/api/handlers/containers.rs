use axum::{Json, extract::State};

use crate::{
    api::{
        contacts::require_contacts_sources,
        dto::{
            ContainerDetailDto, ContainerPageDto,
            contacts_convert::{container_detail_to_dto, container_page_to_dto},
        },
        error::{ApiError, ErrorResponse},
        params::ContainerIdPath,
        router::AppState,
    },
    db::run_timed_query,
};

/// List contact containers
#[utoipa::path(
    get,
    path = "/v1/containers",
    operation_id = "listContainers",
    tag = "containers",
    responses(
        (status = 200, description = "Contact containers", body = ContainerPageDto),
        (status = 503, description = "Contacts databases are unavailable", body = ErrorResponse),
    )
)]
pub async fn list_containers(
    State(state): State<AppState>,
) -> Result<Json<ContainerPageDto>, ApiError> {
    let sources = require_contacts_sources(&state.contacts_sources)?;
    let containers = run_timed_query(|| async { sources.list_containers().await })
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;
    Ok(Json(container_page_to_dto(containers)))
}

/// Get a contact container
#[utoipa::path(
    get,
    path = "/v1/containers/{container_id}",
    operation_id = "getContainer",
    tag = "containers",
    params(ContainerIdPath),
    responses(
        (status = 200, description = "Container detail", body = ContainerDetailDto),
        (status = 404, description = "Container not found", body = ErrorResponse),
        (status = 503, description = "Contacts databases are unavailable", body = ErrorResponse),
    )
)]
pub async fn get_container(
    State(state): State<AppState>,
    axum::extract::Path(path): axum::extract::Path<ContainerIdPath>,
) -> Result<Json<ContainerDetailDto>, ApiError> {
    let sources = require_contacts_sources(&state.contacts_sources)?;
    let container = run_timed_query(|| async {
        sources.get_container(path.container_id.as_str()).await
    })
    .await
    .map_err(|error| ApiError::internal(error.to_string()))?
    .ok_or_else(|| ApiError::not_found("Container not found"))?;
    Ok(Json(container_detail_to_dto(&container)))
}
