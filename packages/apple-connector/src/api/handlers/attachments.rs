use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::Response,
};

use super::health::require_db;
use crate::api::{
    dto::AttachmentDetailDto,
    error::{ApiError, ErrorResponse},
    params::{AttachmentGuidPath, ConditionalRequestHeaders, RangeRequestHeader},
    router::AppState,
};

/// Get attachment metadata
///
/// Returns safe attachment metadata without local filesystem paths.
#[utoipa::path(
    get,
    path = "/v1/attachments/{guid}",
    operation_id = "getAttachment",
    tag = "attachments",
    params(AttachmentGuidPath),
    responses(
        (status = 200, description = "Attachment metadata", body = AttachmentDetailDto),
        (status = 404, description = "Attachment not found", body = ErrorResponse),
        (status = 503, description = "Messages database is unavailable", body = ErrorResponse),
        (status = 500, description = "Unexpected server error", body = ErrorResponse),
    )
)]
pub async fn get_attachment(
    State(state): State<AppState>,
    axum::extract::Path(AttachmentGuidPath { guid }): axum::extract::Path<AttachmentGuidPath>,
) -> Result<Json<AttachmentDetailDto>, ApiError> {
    require_db(&state.db)?;
    Err(ApiError::not_found(format!("attachment {guid} not found")))
}

/// Download attachment content
///
/// Streams attachment bytes with range and conditional request support.
#[utoipa::path(
    get,
    path = "/v1/attachments/{guid}/content",
    operation_id = "getAttachmentContent",
    tag = "attachments",
    params(
        AttachmentGuidPath,
        RangeRequestHeader,
        ConditionalRequestHeaders,
    ),
    responses(
        (status = 200, description = "Full attachment bytes", content_type = "application/octet-stream",
            headers(
                ("Content-Type" = String, description = "Resolved attachment MIME type"),
                ("Content-Length" = i64, description = "Full object length in bytes"),
                ("Content-Disposition" = String, description = "Inline or attachment disposition with a safe filename"),
                ("Accept-Ranges" = String, description = "Always bytes"),
                ("ETag" = String, description = "Strong validator for conditional requests"),
                ("Last-Modified" = String, description = "Last modification time of the attachment bytes"),
                ("X-Content-Type-Options" = String, description = "Always nosniff")
            )
        ),
        (status = 206, description = "Partial attachment bytes", content_type = "application/octet-stream",
            headers(
                ("Content-Type" = String, description = "Resolved attachment MIME type"),
                ("Content-Length" = i64, description = "Length of the returned byte range"),
                ("Content-Range" = String, description = "Byte range delivered and total size"),
                ("Content-Disposition" = String, description = "Inline or attachment disposition with a safe filename"),
                ("Accept-Ranges" = String, description = "Always bytes"),
                ("ETag" = String, description = "Strong validator for conditional requests"),
                ("Last-Modified" = String, description = "Last modification time of the attachment bytes"),
                ("X-Content-Type-Options" = String, description = "Always nosniff")
            )
        ),
        (status = 304, description = "Attachment bytes not modified",
            headers(
                ("ETag" = String, description = "Strong validator for conditional requests"),
                ("Last-Modified" = String, description = "Last modification time of the attachment bytes")
            )
        ),
        (status = 404, description = "Attachment not found", body = ErrorResponse),
        (status = 416, description = "Requested byte range is not satisfiable", body = ErrorResponse),
        (status = 503, description = "Messages database is unavailable", body = ErrorResponse),
        (status = 500, description = "Unexpected server error", body = ErrorResponse),
    )
)]
pub async fn get_attachment_content(
    State(state): State<AppState>,
    axum::extract::Path(AttachmentGuidPath { guid }): axum::extract::Path<AttachmentGuidPath>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let _ = (
        headers.get("range"),
        headers.get("if-none-match"),
        headers.get("if-modified-since"),
    );
    require_db(&state.db)?;
    Err(ApiError::not_found(format!("attachment {guid} not found")))
}

/// Check attachment content metadata
///
/// Returns the same validators and length headers as GET without a response body.
#[utoipa::path(
    head,
    path = "/v1/attachments/{guid}/content",
    operation_id = "headAttachmentContent",
    tag = "attachments",
    params(
        AttachmentGuidPath,
        ConditionalRequestHeaders,
    ),
    responses(
        (status = 200, description = "Attachment exists",
            headers(
                ("Content-Type" = String, description = "Resolved attachment MIME type"),
                ("Content-Length" = i64, description = "Full object length in bytes"),
                ("Accept-Ranges" = String, description = "Always bytes"),
                ("ETag" = String, description = "Strong validator for conditional requests"),
                ("Last-Modified" = String, description = "Last modification time of the attachment bytes"),
                ("X-Content-Type-Options" = String, description = "Always nosniff")
            )
        ),
        (status = 304, description = "Attachment bytes not modified",
            headers(
                ("ETag" = String, description = "Strong validator for conditional requests"),
                ("Last-Modified" = String, description = "Last modification time of the attachment bytes")
            )
        ),
        (status = 404, description = "Attachment not found", body = ErrorResponse),
        (status = 503, description = "Messages database is unavailable", body = ErrorResponse),
        (status = 500, description = "Unexpected server error", body = ErrorResponse),
    )
)]
pub async fn head_attachment_content(
    State(state): State<AppState>,
    axum::extract::Path(AttachmentGuidPath { guid }): axum::extract::Path<AttachmentGuidPath>,
    _headers: HeaderMap,
) -> Result<StatusCode, ApiError> {
    require_db(&state.db)?;
    Err(ApiError::not_found(format!("attachment {guid} not found")))
}
