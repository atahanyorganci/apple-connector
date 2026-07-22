use axum::Json;
use utoipa::openapi::OpenApi;

use crate::api::router::AppState;

/// Get OpenAPI specification
///
/// Returns the OpenAPI 3.1 contract served by this API.
#[utoipa::path(
    get,
    path = "/openapi.json",
    operation_id = "getOpenApiSpec",
    tag = "meta",
    responses(
        (status = 200, description = "OpenAPI 3.1 specification", content_type = "application/json"),
        (status = 500, description = "Unexpected server error", body = crate::api::error::ErrorResponse),
    )
)]
pub async fn get_openapi_spec(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> Json<OpenApi> {
    Json(state.openapi.as_ref().clone())
}
