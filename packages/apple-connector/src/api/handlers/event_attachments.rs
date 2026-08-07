use axum::{
    body::Body,
    extract::{Request, State},
    http::{HeaderMap, Method},
    response::Response,
};

use super::health::require_calendar_db;
use crate::{
    api::{
        error::{ApiError, ErrorCode, ErrorResponse},
        media::{ServeMedia, copy_conditional_headers, serve_media_bytes},
        params::{ConditionalRequestHeaders, EventAttachmentIdPath, RangeRequestHeader},
        router::AppState,
    },
    calendar::{CalendarRepository, attachment_path::resolve_attachment_path},
    db::run_timed_query,
    messages::attachment_path::{content_disposition, sanitize_download_filename},
};

/// Stream event attachment bytes
#[utoipa::path(
    get,
    path = "/v1/events/{event_id}/attachments/{attachment_id}",
    operation_id = "getEventAttachmentContent",
    tag = "event-attachments",
    params(EventAttachmentIdPath, ConditionalRequestHeaders, RangeRequestHeader),
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
        (status = 503, description = "Calendar database is unavailable", body = ErrorResponse),
        (status = 500, description = "Unexpected server error", body = ErrorResponse),
    )
)]
pub async fn get_event_attachment_content(
    State(state): State<AppState>,
    axum::extract::Path(path): axum::extract::Path<EventAttachmentIdPath>,
    request: Request,
) -> Result<Response, ApiError> {
    let (event_id, attachment_id) = path.validated()?;
    let (path, content_type, disposition) =
        resolve_event_attachment_media(&state, event_id.as_str(), attachment_id.as_str()).await?;
    serve_media_bytes(
        &state.blocking_io,
        ServeMedia {
            path,
            content_type,
            content_disposition: disposition,
            unavailable: ErrorCode::EventAttachmentUnavailable,
        },
        request,
    )
    .await
}

/// Check event attachment content metadata
#[utoipa::path(
    head,
    path = "/v1/events/{event_id}/attachments/{attachment_id}",
    operation_id = "headEventAttachmentContent",
    tag = "event-attachments",
    params(EventAttachmentIdPath, ConditionalRequestHeaders),
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
        (status = 503, description = "Calendar database is unavailable", body = ErrorResponse),
        (status = 500, description = "Unexpected server error", body = ErrorResponse),
    )
)]
pub async fn head_event_attachment_content(
    State(state): State<AppState>,
    axum::extract::Path(path): axum::extract::Path<EventAttachmentIdPath>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let (event_id, attachment_id) = path.validated()?;
    let (path, content_type, disposition) =
        resolve_event_attachment_media(&state, event_id.as_str(), attachment_id.as_str()).await?;
    let mut request = Request::builder()
        .method(Method::HEAD)
        .uri("/")
        .body(Body::empty())
        .map_err(|_| ApiError::internal("failed to build head request"))?;
    copy_conditional_headers(&headers, request.headers_mut());
    serve_media_bytes(
        &state.blocking_io,
        ServeMedia {
            path,
            content_type,
            content_disposition: disposition,
            unavailable: ErrorCode::EventAttachmentUnavailable,
        },
        request,
    )
    .await
}

async fn resolve_event_attachment_media(
    state: &AppState,
    event_id: &str,
    attachment_id: &str,
) -> Result<(std::path::PathBuf, String, String), ApiError> {
    let pool = require_calendar_db(&state.calendar_db)?;
    let attachment = run_timed_query(|| async {
        CalendarRepository::new(pool)
            .get_attachment(event_id, attachment_id)
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
    let filename = attachment
        .filename
        .as_deref()
        .or_else(|| resolved.file_name().and_then(|name| name.to_str()))
        .unwrap_or(attachment_id);
    let content_type = attachment
        .format
        .as_deref()
        .map(mime_from_format)
        .unwrap_or_else(|| "application/octet-stream".to_owned());
    let disposition = content_disposition(
        &crate::messages::AttachmentKind::File,
        &sanitize_download_filename(Some(filename), Some(&content_type), attachment_id),
    );
    Ok((resolved, content_type, disposition))
}

fn mime_from_format(format: &str) -> String {
    match format {
        "com.adobe.pdf" | "public.pdf" => "application/pdf".to_owned(),
        "public.png" => "image/png".to_owned(),
        "public.jpeg" => "image/jpeg".to_owned(),
        _ => "application/octet-stream".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, io::Write, path::PathBuf};

    use axum::body::Body;
    use http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    use crate::{
        api::router::{AppState, router},
        db::connect_pool,
        fixtures::{CalendarFixtureDb, SEED_EVENT_ATTACHMENT_ID, SEED_EVENT_ID},
        messages::attachment_path::canonicalize_attachment_root,
    };

    async fn response_bytes(
        app: axum::Router,
        request: Request<Body>,
    ) -> Result<(StatusCode, http::HeaderMap, Vec<u8>), Box<dyn std::error::Error>> {
        let response = app.oneshot(request).await?;
        let status = response.status();
        let headers = response.headers().clone();
        let body = response.into_body().collect().await?.to_bytes().to_vec();
        Ok((status, headers, body))
    }

    #[tokio::test]
    async fn get_head_range_and_conditional_requests_work() -> Result<(), Box<dyn std::error::Error>>
    {
        let fixture = CalendarFixtureDb::seeded().await?;
        let root_dir = tempfile::tempdir()?;
        let attachments = root_dir.path().join("Attachments");
        fs::create_dir_all(&attachments)?;
        let file = attachments.join("agenda.pdf");
        fs::File::create(&file)?.write_all(b"0123456789")?;
        let root = canonicalize_attachment_root(root_dir.path())?;

        let pool = connect_pool(fixture.path()).await?;
        let app = router(AppState::with_attachment_roots(
            None,
            None,
            None,
            Some(pool),
            crate::contacts::ContactsSources::new(std::collections::HashMap::new()),
            PathBuf::from("/var/empty/messages-attachments"),
            PathBuf::from("/var/empty/reminders-attachments"),
            PathBuf::from("/var/empty/notes-attachments"),
            root,
            None,
            None,
        ));

        let uri = format!("/v1/events/{SEED_EVENT_ID}/attachments/{SEED_EVENT_ATTACHMENT_ID}");

        let (status, headers, body) = response_bytes(
            app.clone(),
            Request::builder().uri(&uri).body(Body::empty())?,
        )
        .await?;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, b"0123456789");
        assert_eq!(
            headers
                .get("content-type")
                .ok_or("missing content-type header")?,
            "application/pdf"
        );
        assert_eq!(
            headers
                .get("accept-ranges")
                .ok_or("missing accept-ranges header")?,
            "bytes"
        );
        assert_eq!(
            headers
                .get("x-content-type-options")
                .ok_or("missing x-content-type-options header")?,
            "nosniff"
        );
        let etag = headers.get("etag").ok_or("missing etag header")?.clone();

        let (status, headers, body) = response_bytes(
            app.clone(),
            Request::builder()
                .uri(&uri)
                .header("Range", "bytes=0-4")
                .body(Body::empty())?,
        )
        .await?;
        assert_eq!(status, StatusCode::PARTIAL_CONTENT);
        assert_eq!(body, b"01234");
        assert!(
            headers
                .get("content-range")
                .ok_or("missing content-range header")?
                .to_str()?
                .contains("0-4/10")
        );

        let (status, _, body) = response_bytes(
            app.clone(),
            Request::builder()
                .method("HEAD")
                .uri(&uri)
                .body(Body::empty())?,
        )
        .await?;
        assert_eq!(status, StatusCode::OK);
        assert!(body.is_empty());

        let (status, _, body) = response_bytes(
            app.clone(),
            Request::builder()
                .uri(&uri)
                .header("If-None-Match", etag)
                .body(Body::empty())?,
        )
        .await?;
        assert_eq!(status, StatusCode::NOT_MODIFIED);
        assert!(body.is_empty());

        let (status, _, _) = response_bytes(
            app,
            Request::builder()
                .uri(&uri)
                .header("Range", "bytes=100-200")
                .body(Body::empty())?,
        )
        .await?;
        assert_eq!(status, StatusCode::RANGE_NOT_SATISFIABLE);
        Ok(())
    }
}
