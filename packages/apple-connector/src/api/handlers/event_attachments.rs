use axum::{
    body::Body,
    extract::State,
    http::{StatusCode, header},
    response::Response,
};
use tokio_util::io::ReaderStream;

use super::health::require_calendar_db;
use crate::{
    api::{
        error::{ApiError, ErrorCode, ErrorResponse},
        params::{ConditionalRequestHeaders, EventAttachmentIdPath, RangeRequestHeader},
        router::AppState,
    },
    calendar::{CalendarRepository, attachment_path::resolve_attachment_path},
    db::run_timed_query,
};

/// Stream event attachment bytes
#[utoipa::path(
    get,
    path = "/v1/events/{event_id}/attachments/{attachment_id}",
    operation_id = "getEventAttachmentContent",
    tag = "event-attachments",
    params(EventAttachmentIdPath, ConditionalRequestHeaders, RangeRequestHeader),
    responses(
        (status = 200, description = "Attachment bytes", content_type = "application/octet-stream"),
        (status = 404, description = "Attachment not found", body = ErrorResponse),
        (status = 503, description = "Calendar database is unavailable", body = ErrorResponse),
    )
)]
pub async fn get_event_attachment_content(
    State(state): State<AppState>,
    axum::extract::Path(path): axum::extract::Path<EventAttachmentIdPath>,
) -> Result<Response, ApiError> {
    let (event_id, attachment_id) = path.validated()?;
    let pool = require_calendar_db(&state.calendar_db)?;
    let attachment = run_timed_query(|| async {
        CalendarRepository::new(pool)
            .get_attachment(event_id.as_str(), attachment_id.as_str())
            .await
    })
    .await
    .map_err(ApiError::from_sqlx)?
    .ok_or_else(|| ApiError::new(ErrorCode::EventAttachmentNotFound))?;
    let local_path = attachment
        .local_path
        .as_deref()
        .ok_or_else(|| ApiError::new(ErrorCode::EventAttachmentUnavailable))?;
    let resolved = resolve_attachment_path(state.calendar_attachment_root.as_ref(), local_path)
        .map_err(|error| {
            ApiError::with_message(ErrorCode::EventAttachmentUnavailable, error.message())
        })?;
    let file = tokio::fs::File::open(resolved)
        .await
        .map_err(|_| ApiError::new(ErrorCode::EventAttachmentUnavailable))?;
    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);
    let content_type = attachment
        .format
        .as_deref()
        .map(mime_from_format)
        .unwrap_or_else(|| "application/octet-stream".to_owned());
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .header(header::ACCEPT_RANGES, "bytes")
        .body(body)
        .map_err(|_| ApiError::internal("failed to build attachment response"))
}

fn mime_from_format(format: &str) -> String {
    match format {
        "com.adobe.pdf" | "public.pdf" => "application/pdf".to_owned(),
        "public.png" => "image/png".to_owned(),
        "public.jpeg" => "image/jpeg".to_owned(),
        _ => "application/octet-stream".to_owned(),
    }
}
