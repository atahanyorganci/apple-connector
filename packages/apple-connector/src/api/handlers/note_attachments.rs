use std::sync::Arc;

use axum::{
    Json,
    body::Body,
    extract::{Request, State},
    http::{HeaderMap, Method},
    response::Response,
};

use super::health::require_notes_db;
use crate::{
    api::{
        dto::{NoteAttachmentDetailDto, note_convert::note_attachment_detail_to_dto},
        error::{ApiError, ErrorCode, ErrorResponse},
        media::{ServeMedia, copy_conditional_headers, serve_media_bytes},
        params::{ConditionalRequestHeaders, NoteAttachmentIdPath, RangeRequestHeader},
        router::AppState,
    },
    db::run_timed_query,
    messages::attachment_path::{
        content_disposition, resolve_content_type, sanitize_download_filename,
    },
    notes::{NoteAttachment, NoteRepository, attachment_path::validate_attachment_path_async},
};

/// Get note attachment metadata
#[utoipa::path(
    get,
    path = "/v1/note-attachments/{id}",
    operation_id = "getNoteAttachment",
    tag = "note-attachments",
    params(NoteAttachmentIdPath),
    responses(
        (status = 200, description = "Note attachment metadata", body = NoteAttachmentDetailDto),
        (status = 404, description = "Attachment not found", body = ErrorResponse),
        (status = 503, description = "Notes database is unavailable", body = ErrorResponse),
    )
)]
pub async fn get_note_attachment(
    State(state): State<AppState>,
    axum::extract::Path(path): axum::extract::Path<NoteAttachmentIdPath>,
) -> Result<Json<NoteAttachmentDetailDto>, ApiError> {
    let id = path.validated()?;
    let attachment = resolve_attachment(&state, id.as_str()).await?;
    Ok(Json(note_attachment_detail_to_dto(&attachment)))
}

/// Download note attachment content
#[utoipa::path(
    get,
    path = "/v1/note-attachments/{id}/content",
    operation_id = "getNoteAttachmentContent",
    tag = "note-attachments",
    params(NoteAttachmentIdPath, RangeRequestHeader, ConditionalRequestHeaders),
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
        (status = 503, description = "Notes database is unavailable", body = ErrorResponse),
        (status = 500, description = "Unexpected server error", body = ErrorResponse),
    )
)]
pub async fn get_note_attachment_content(
    State(state): State<AppState>,
    axum::extract::Path(path): axum::extract::Path<NoteAttachmentIdPath>,
    request: Request,
) -> Result<Response, ApiError> {
    let id = path.validated()?;
    let (_, file_path) = resolve_attachment_path(&state, id.as_str()).await?;
    serve_note_bytes(&state, file_path, request).await
}

/// Head note attachment content
#[utoipa::path(
    head,
    path = "/v1/note-attachments/{id}/content",
    operation_id = "headNoteAttachmentContent",
    tag = "note-attachments",
    params(NoteAttachmentIdPath, ConditionalRequestHeaders),
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
        (status = 503, description = "Notes database is unavailable", body = ErrorResponse),
        (status = 500, description = "Unexpected server error", body = ErrorResponse),
    )
)]
pub async fn head_note_attachment_content(
    State(state): State<AppState>,
    axum::extract::Path(path): axum::extract::Path<NoteAttachmentIdPath>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let id = path.validated()?;
    let (_, file_path) = resolve_attachment_path(&state, id.as_str()).await?;
    let mut request = Request::builder()
        .method(Method::HEAD)
        .uri("/")
        .body(Body::empty())
        .map_err(|_| ApiError::internal("failed to build head request"))?;
    copy_conditional_headers(&headers, request.headers_mut());
    serve_note_bytes(&state, file_path, request).await
}

async fn resolve_attachment(state: &AppState, id: &str) -> Result<NoteAttachment, ApiError> {
    let pool = require_notes_db(&state.notes_db)?;

    let note_entity_ids = state
        .cached_notes_entity_ids()
        .await
        .map_err(ApiError::from_sqlx)?;
    run_timed_query(|| async {
        NoteRepository::with_entity_ids(pool, Arc::clone(&note_entity_ids))
            .get_attachment_by_id(id)
            .await
    })
    .await
    .map_err(ApiError::from_sqlx)?
    .ok_or_else(|| {
        ApiError::with_details(
            ErrorCode::NoteAttachmentNotFound,
            format!("note attachment {id} not found"),
            serde_json::json!({ "id": id }),
        )
    })
}

async fn resolve_attachment_path(
    state: &AppState,
    id: &str,
) -> Result<(NoteAttachment, std::path::PathBuf), ApiError> {
    let attachment = resolve_attachment(state, id).await?;
    let account_id = attachment.account_id.clone().ok_or_else(|| {
        ApiError::with_details(
            ErrorCode::NoteAttachmentNotFound,
            format!("note attachment {id} not found"),
            serde_json::json!({ "id": id }),
        )
    })?;
    let filename = attachment.filename.clone().ok_or_else(|| {
        ApiError::with_details(
            ErrorCode::NoteAttachmentNotFound,
            format!("note attachment {id} not found"),
            serde_json::json!({ "id": id }),
        )
    })?;

    let validated = validate_attachment_path_async(
        &state.blocking_io,
        state.notes_attachment_root.as_ref().clone(),
        account_id,
        filename,
    )
    .await
    .map_err(|_| ApiError::internal("blocking attachment validation failed"))?
    .map_err(|_| {
        ApiError::with_details(
            ErrorCode::NoteAttachmentNotFound,
            format!("note attachment {id} not found"),
            serde_json::json!({ "id": id }),
        )
    })?;

    Ok((attachment, validated.canonical_path))
}

async fn serve_note_bytes(
    state: &AppState,
    path: std::path::PathBuf,
    request: Request,
) -> Result<Response, ApiError> {
    let filename = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("attachment");
    let content_type = resolve_content_type(None);
    let disposition = content_disposition(
        &crate::messages::AttachmentKind::File,
        &sanitize_download_filename(Some(filename), None, filename),
    );
    serve_media_bytes(
        &state.blocking_io,
        ServeMedia {
            path,
            content_type,
            content_disposition: disposition,
            unavailable: ErrorCode::NoteAttachmentUnavailable,
        },
        request,
    )
    .await
}
