use axum::{
    Json,
    body::Body,
    extract::{Request, State},
    http::{HeaderValue, Method, header},
    response::Response,
};
use tower::ServiceExt;
use tower_http::services::ServeFile;

use super::health::require_reminders_db;
use crate::{
    api::{
        dto::{
            ReminderAttachmentDetailDto,
            reminder_convert::reminder_attachment_detail_to_dto,
        },
        error::{ApiError, ErrorResponse},
        params::{ConditionalRequestHeaders, ReminderAttachmentIdPath, RangeRequestHeader},
        router::AppState,
    },
    db::run_timed_query,
    messages::attachment_path::{
        content_disposition, file_validators, resolve_content_type, sanitize_download_filename,
    },
    reminders::{ReminderAttachment, ReminderRepository, attachment_path::validate_attachment_path},
};

/// Get reminder attachment metadata
#[utoipa::path(
    get,
    path = "/v1/reminder-attachments/{id}",
    operation_id = "getReminderAttachment",
    tag = "reminder-attachments",
    params(ReminderAttachmentIdPath),
    responses(
        (status = 200, description = "Reminder attachment metadata", body = ReminderAttachmentDetailDto),
        (status = 404, description = "Attachment not found", body = ErrorResponse),
        (status = 503, description = "Reminders database is unavailable", body = ErrorResponse),
    )
)]
pub async fn get_reminder_attachment(
    State(state): State<AppState>,
    axum::extract::Path(ReminderAttachmentIdPath { id }): axum::extract::Path<ReminderAttachmentIdPath>,
) -> Result<Json<ReminderAttachmentDetailDto>, ApiError> {
    let (attachment, reminder_id) = resolve_attachment(&state, &id).await?;
    Ok(Json(reminder_attachment_detail_to_dto(
        &attachment,
        reminder_id,
    )))
}

/// Download reminder attachment content
#[utoipa::path(
    get,
    path = "/v1/reminder-attachments/{id}/content",
    operation_id = "getReminderAttachmentContent",
    tag = "reminder-attachments",
    params(ReminderAttachmentIdPath, RangeRequestHeader, ConditionalRequestHeaders),
    responses(
        (status = 200, description = "Attachment bytes", content_type = "application/octet-stream"),
        (status = 404, description = "Attachment not found", body = ErrorResponse),
        (status = 503, description = "Reminders database is unavailable", body = ErrorResponse),
    )
)]
pub async fn get_reminder_attachment_content(
    State(state): State<AppState>,
    axum::extract::Path(ReminderAttachmentIdPath { id }): axum::extract::Path<ReminderAttachmentIdPath>,
    headers: axum::http::HeaderMap,
) -> Result<Response, ApiError> {
    let (_, path) = resolve_attachment_path(&state, &id).await?;
    serve_bytes(path, headers, Method::GET).await
}

/// Head reminder attachment content
#[utoipa::path(
    head,
    path = "/v1/reminder-attachments/{id}/content",
    operation_id = "headReminderAttachmentContent",
    tag = "reminder-attachments",
    params(ReminderAttachmentIdPath, RangeRequestHeader, ConditionalRequestHeaders),
    responses(
        (status = 200, description = "Attachment metadata headers"),
        (status = 404, description = "Attachment not found", body = ErrorResponse),
    )
)]
pub async fn head_reminder_attachment_content(
    State(state): State<AppState>,
    axum::extract::Path(ReminderAttachmentIdPath { id }): axum::extract::Path<ReminderAttachmentIdPath>,
    headers: axum::http::HeaderMap,
) -> Result<Response, ApiError> {
    let (_, path) = resolve_attachment_path(&state, &id).await?;
    serve_bytes(path, headers, Method::HEAD).await
}

async fn resolve_attachment(
    state: &AppState,
    id: &str,
) -> Result<(ReminderAttachment, String), ApiError> {
    let pool = require_reminders_db(&state.reminders_db)?;
    let attachment = run_timed_query(|| async {
        ReminderRepository::new(pool).get_attachment_by_id(id).await
    })
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?
        .ok_or_else(|| ApiError::not_found(format!("reminder attachment {id} not found")))?;

    let reminder_id = run_timed_query(|| async {
        ReminderRepository::new(pool)
            .get_reminder_id_for_row(attachment.reminder_row_id)
            .await
    })
    .await
    .map_err(|error| ApiError::internal(error.to_string()))?
    .unwrap_or_else(|| "unknown".to_owned());

    Ok((attachment, reminder_id))
}

async fn resolve_attachment_path(
    state: &AppState,
    id: &str,
) -> Result<(ReminderAttachment, std::path::PathBuf), ApiError> {
    let pool = require_reminders_db(&state.reminders_db)?;
    let attachment = run_timed_query(|| async {
        ReminderRepository::new(pool).get_attachment_by_id(id).await
    })
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?
        .ok_or_else(|| ApiError::not_found(format!("reminder attachment {id} not found")))?;

    let filename = attachment
        .filename
        .as_deref()
        .ok_or_else(|| ApiError::not_found(format!("reminder attachment {id} not found")))?;

    let validated = validate_attachment_path(state.reminders_attachment_root.as_ref(), filename)
        .map_err(|_| ApiError::not_found(format!("reminder attachment {id} not found")))?;

    Ok((attachment, validated.canonical_path))
}

async fn serve_bytes(
    path: std::path::PathBuf,
    headers: axum::http::HeaderMap,
    method: Method,
) -> Result<Response, ApiError> {
    let mut request = Request::builder().method(method).uri("/").body(Body::empty()).expect("request");
    if let Some(range) = headers.get(header::RANGE) {
        request.headers_mut().insert(header::RANGE, range.clone());
    }

    let validators = file_validators(&path)
        .map_err(|_| ApiError::not_found("reminder attachment is not available"))?;
    let filename = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("attachment");
    let content_type = resolve_content_type(None);
    let disposition = content_disposition(
        &crate::messages::AttachmentKind::File,
        &sanitize_download_filename(Some(filename), None, filename),
    );

    let response = ServeFile::new(path)
        .oneshot(request)
        .await
        .map_err(|_| ApiError::internal("attachment delivery failed"))?
        .map(Body::new);

    let mut response = response;
    let headers = response.headers_mut();
    if let Ok(value) = HeaderValue::from_str(&content_type) {
        headers.insert(header::CONTENT_TYPE, value);
    }
    if let Ok(value) = HeaderValue::from_str(&disposition) {
        headers.insert(header::CONTENT_DISPOSITION, value);
    }
    if let Ok(value) = HeaderValue::from_str(&validators.etag) {
        headers.insert(header::ETAG, value);
    }
    if let Ok(value) = HeaderValue::from_str(&validators.last_modified) {
        headers.insert(header::LAST_MODIFIED, value);
    }
    headers.insert(header::ACCEPT_RANGES, HeaderValue::from_static("bytes"));
    Ok(response)
}
