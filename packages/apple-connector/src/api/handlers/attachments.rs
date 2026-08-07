use axum::{
    Json,
    body::Body,
    extract::{Request, State},
    http::{HeaderMap, Method},
    response::Response,
};

use super::health::require_messages_db;
use crate::{
    api::{
        dto::{AttachmentDetailDto, convert::attachment_detail_to_dto},
        error::{ApiError, ErrorCode},
        params::AttachmentGuidPath,
        router::AppState,
    },
    db::run_timed_query,
    messages::{
        Attachment,
        attachment_path::{
            content_disposition, is_present_on_disk_async, resolve_content_type,
            sanitize_download_filename, validate_attachment_path_async,
        },
        repository::MessageRepository,
    },
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
    axum::extract::Path(path): axum::extract::Path<AttachmentGuidPath>,
) -> Result<Json<AttachmentDetailDto>, ApiError> {
    let guid = path.validated()?;
    let pool = require_messages_db(&state.messages_db)?;
    let mut attachment = run_timed_query(|| async {
        MessageRepository::new(pool)
            .get_attachment_by_guid(guid.as_str())
            .await
    })
    .await
    .map_err(ApiError::from_sqlx)?
    .ok_or_else(|| {
        ApiError::with_details(
            ErrorCode::MessageAttachmentNotFound,
            format!("attachment {guid} not found"),
            serde_json::json!({ "guid": guid.as_str() }),
        )
    })?;

    attachment.present_on_disk = is_present_on_disk_async(
        &state.blocking_io,
        state.attachment_root.as_ref().clone(),
        attachment.filename.clone(),
    )
    .await
    .map_err(|_| ApiError::internal("blocking attachment check failed"))?;

    Ok(Json(attachment_detail_to_dto(&attachment)))
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
    axum::extract::Path(path): axum::extract::Path<AttachmentGuidPath>,
    request: Request,
) -> Result<Response, ApiError> {
    let guid = path.validated()?;
    let (attachment, validated_path) = resolve_content_attachment(&state, guid.as_str()).await?;
    serve_attachment_bytes(&state.blocking_io, attachment, validated_path, request).await
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
    axum::extract::Path(path): axum::extract::Path<AttachmentGuidPath>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let guid = path.validated()?;
    let (attachment, validated_path) = resolve_content_attachment(&state, guid.as_str()).await?;
    let mut request = Request::builder()
        .method(Method::HEAD)
        .uri("/")
        .body(Body::empty())
        .map_err(|_| ApiError::internal("failed to build head request"))?;
    copy_conditional_headers(&headers, request.headers_mut());
    serve_attachment_bytes(&state.blocking_io, attachment, validated_path, request).await
}

async fn resolve_content_attachment(
    state: &AppState,
    guid: &str,
) -> Result<(Attachment, std::path::PathBuf), ApiError> {
    let pool = require_messages_db(&state.messages_db)?;
    let attachment = run_timed_query(|| async {
        MessageRepository::new(pool)
            .get_attachment_by_guid(guid)
            .await
    })
    .await
    .map_err(ApiError::from_sqlx)?
    .ok_or_else(|| {
        ApiError::with_details(
            ErrorCode::MessageAttachmentNotFound,
            format!("attachment {guid} not found"),
            serde_json::json!({ "guid": guid }),
        )
    })?;

    if !attachment.transfer_complete {
        return Err(ApiError::with_details(
            ErrorCode::MessageAttachmentUnavailable,
            format!("attachment {guid} is not available"),
            serde_json::json!({ "guid": guid }),
        ));
    }

    let filename = attachment.filename.clone().ok_or_else(|| {
        ApiError::with_details(
            ErrorCode::MessageAttachmentUnavailable,
            format!("attachment {guid} is not available"),
            serde_json::json!({ "guid": guid }),
        )
    })?;

    let validated = validate_attachment_path_async(
        &state.blocking_io,
        state.attachment_root.as_ref().clone(),
        filename,
    )
    .await
    .map_err(|_| ApiError::internal("blocking attachment validation failed"))?
    .map_err(|_| {
        ApiError::with_details(
            ErrorCode::MessageAttachmentUnavailable,
            format!("attachment {guid} is not available"),
            serde_json::json!({ "guid": guid }),
        )
    })?;

    Ok((attachment, validated.canonical_path))
}

async fn serve_attachment_bytes(
    blocking_io: &crate::api::blocking_io::BlockingIoPool,
    attachment: Attachment,
    path: std::path::PathBuf,
    request: Request,
) -> Result<Response, ApiError> {
    let filename = sanitize_download_filename(
        attachment.transfer_name.as_deref(),
        attachment.mime_type.as_deref(),
        attachment.guid.as_str(),
    );
    let content_type = resolve_content_type(attachment.mime_type.as_deref());
    let disposition = content_disposition(&attachment.kind, &filename);
    crate::api::media::serve_media_bytes(
        blocking_io,
        crate::api::media::ServeMedia {
            path,
            content_type,
            content_disposition: disposition,
            unavailable: ErrorCode::MessageAttachmentUnavailable,
        },
        request,
    )
    .await
}

fn copy_conditional_headers(source: &HeaderMap, destination: &mut HeaderMap) {
    crate::api::media::copy_conditional_headers(source, destination);
}

use crate::api::{
    error::ErrorResponse,
    params::{ConditionalRequestHeaders, RangeRequestHeader},
};

#[cfg(test)]
mod tests {
    use std::{fs, io::Write, os::unix::fs::symlink, path::PathBuf};

    use axum::body::Body;
    use http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use sqlx::Connection;
    use tower::ServiceExt;

    use super::*;
    use crate::{
        api::router::{AppState, router},
        db::connect_pool,
        fixtures::FixtureDb,
        messages::{
            attachment_path::canonicalize_attachment_root, attachments::TRANSFER_STATE_COMPLETE,
        },
    };

    const FORBIDDEN_SUBSTRINGS: &[&str] = &[
        "chat.db",
        "Library/Messages",
        "/Users/",
        "filename",
        "resolved_path",
        "Attachments/",
    ];

    struct AttachmentFixture {
        db: FixtureDb,
        root: PathBuf,
        _root_dir: tempfile::TempDir,
    }

    impl AttachmentFixture {
        async fn new() -> Result<Self, Box<dyn std::error::Error>> {
            let root_dir = tempfile::tempdir()?;
            let root = root_dir.path().join("Attachments");
            fs::create_dir_all(&root)?;

            let db = FixtureDb::empty().await?;
            let mut connection =
                sqlx::SqliteConnection::connect(db.path().to_str().ok_or("invalid path")?).await?;

            for statement in [
                "DROP TRIGGER IF EXISTS verify_chat_insert",
                "DROP TRIGGER IF EXISTS verify_chat_update",
            ] {
                sqlx::query(statement).execute(&mut connection).await?;
            }

            connection.close().await.ok();
            Ok(Self {
                db,
                root,
                _root_dir: root_dir,
            })
        }

        fn app(&self, pool: sqlx::SqlitePool) -> Result<axum::Router, Box<dyn std::error::Error>> {
            let root = canonicalize_attachment_root(&self.root)?;
            Ok(router(AppState::with_attachment_roots(
                Some(pool),
                None,
                None,
                None,
                crate::contacts::ContactsSources::new(std::collections::HashMap::new()),
                root,
                PathBuf::from("/var/empty/reminders-attachments"),
                PathBuf::from("/var/empty/notes-attachments"),
                PathBuf::from("/var/empty/calendar-attachments"),
                None,
                None,
            )))
        }

        async fn seed_attachment(
            &self,
            guid: &str,
            filename: &str,
            bytes: &[u8],
            transfer_state: i64,
            mime_type: Option<&str>,
            transfer_name: Option<&str>,
        ) -> Result<(), Box<dyn std::error::Error>> {
            let file_path = self.root.join(filename);
            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&file_path, bytes)?;
            let stored_filename = fs::canonicalize(&file_path)?.to_string_lossy().into_owned();

            let mut connection =
                sqlx::SqliteConnection::connect(self.db.path().to_str().ok_or("invalid path")?)
                    .await?;
            sqlx::query(
                "INSERT INTO attachment (guid, original_guid, filename, mime_type, transfer_name, total_bytes, transfer_state) \
                 VALUES (?1, ?1, ?2, ?3, ?4, ?5, ?6)",
            )
            .bind(guid)
            .bind(stored_filename)
            .bind(mime_type)
            .bind(transfer_name)
            .bind(i64::try_from(bytes.len()).map_err(|e| -> Box<dyn std::error::Error> {
                Box::new(e)
            })?)
            .bind(transfer_state)
            .execute(&mut connection)
            .await?;
            connection.close().await.ok();
            Ok(())
        }
    }

    async fn response_bytes(
        app: axum::Router,
        request: Request<Body>,
    ) -> Result<(StatusCode, HeaderMap, Vec<u8>), Box<dyn std::error::Error>> {
        let response = app.oneshot(request).await?;
        let status = response.status();
        let headers = response.headers().clone();
        let body = response.into_body().collect().await?.to_bytes().to_vec();
        Ok((status, headers, body))
    }

    fn assert_safe_payload(payload: &str) {
        for forbidden in FORBIDDEN_SUBSTRINGS {
            assert!(
                !payload.contains(forbidden),
                "response leaked `{forbidden}`: {payload}"
            );
        }
    }

    #[tokio::test]
    async fn metadata_exposes_availability_and_content_url_without_paths()
    -> Result<(), Box<dyn std::error::Error>> {
        let fixture = AttachmentFixture::new().await?;
        fixture
            .seed_attachment(
                "at-meta",
                "photo.jpg",
                b"jpeg-bytes",
                TRANSFER_STATE_COMPLETE,
                Some("image/jpeg"),
                Some("vacation.jpg"),
            )
            .await?;

        let pool = connect_pool(fixture.db.path()).await?;
        let app = fixture.app(pool)?;
        let (status, headers, body) = response_bytes(
            app,
            Request::builder()
                .uri("/v1/attachments/at-meta")
                .body(Body::empty())?,
        )
        .await?;
        assert_eq!(status, StatusCode::OK);
        let payload = String::from_utf8(body)?;
        assert_safe_payload(&payload);
        let json: serde_json::Value = serde_json::from_str(&payload)?;
        assert_eq!(json["guid"], "at-meta");
        assert_eq!(json["present_on_disk"], true);
        assert_eq!(json["transfer_complete"], true);
        assert_eq!(json["content_url"], "/v1/attachments/at-meta/content");
        assert!(json.get("filename").is_none());
        assert!(headers.get("content-disposition").is_none());
        Ok(())
    }

    #[tokio::test]
    async fn get_head_range_and_conditional_requests_work() -> Result<(), Box<dyn std::error::Error>>
    {
        let fixture = AttachmentFixture::new().await?;
        fixture
            .seed_attachment(
                "at-bytes",
                "note.txt",
                b"0123456789",
                TRANSFER_STATE_COMPLETE,
                None,
                Some("note.txt"),
            )
            .await?;

        let pool = connect_pool(fixture.db.path()).await?;
        let app = fixture.app(pool)?;

        let (status, headers, body) = response_bytes(
            app.clone(),
            Request::builder()
                .uri("/v1/attachments/at-bytes/content")
                .body(Body::empty())?,
        )
        .await?;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, b"0123456789");
        assert_eq!(
            headers
                .get("content-type")
                .ok_or("missing content-type header")?,
            "application/octet-stream"
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
        assert!(
            headers
                .get("content-disposition")
                .ok_or("missing content-disposition header")?
                .to_str()?
                .contains("note.txt")
        );
        let etag = headers.get("etag").ok_or("missing etag header")?.clone();

        let (status, headers, body) = response_bytes(
            app.clone(),
            Request::builder()
                .uri("/v1/attachments/at-bytes/content")
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
                .uri("/v1/attachments/at-bytes/content")
                .body(Body::empty())?,
        )
        .await?;
        assert_eq!(status, StatusCode::OK);
        assert!(body.is_empty());

        let (status, _, body) = response_bytes(
            app.clone(),
            Request::builder()
                .uri("/v1/attachments/at-bytes/content")
                .header("If-None-Match", etag)
                .body(Body::empty())?,
        )
        .await?;
        assert_eq!(status, StatusCode::NOT_MODIFIED);
        assert!(body.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn invalid_range_missing_file_and_incomplete_transfer_are_handled()
    -> Result<(), Box<dyn std::error::Error>> {
        let fixture = AttachmentFixture::new().await?;
        fixture
            .seed_attachment(
                "at-range",
                "range.bin",
                b"12345",
                TRANSFER_STATE_COMPLETE,
                None,
                None,
            )
            .await?;
        fixture
            .seed_attachment("at-incomplete", "pending.bin", b"pending", 1, None, None)
            .await?;
        fixture
            .seed_attachment(
                "at-missing",
                "missing.bin",
                b"missing",
                TRANSFER_STATE_COMPLETE,
                None,
                None,
            )
            .await?;

        fs::remove_file(fixture.root.join("missing.bin"))?;

        let pool = connect_pool(fixture.db.path()).await?;
        let app = fixture.app(pool)?;

        let (status, _, body) = response_bytes(
            app.clone(),
            Request::builder()
                .uri("/v1/attachments/at-range/content")
                .header("Range", "bytes=100-200")
                .body(Body::empty())?,
        )
        .await?;
        assert_eq!(status, StatusCode::RANGE_NOT_SATISFIABLE);
        let payload = String::from_utf8(body)?;
        assert!(payload.contains("range_not_satisfiable"));

        for guid in ["at-incomplete", "at-missing", "at-absent"] {
            let uri = format!("/v1/attachments/{guid}/content");
            let (status, _, body) = response_bytes(
                app.clone(),
                Request::builder().uri(uri).body(Body::empty())?,
            )
            .await?;
            assert_eq!(status, StatusCode::NOT_FOUND, "guid {guid}");
            assert_safe_payload(&String::from_utf8(body)?);
        }
        Ok(())
    }

    #[tokio::test]
    async fn traversal_and_symlink_paths_are_denied() -> Result<(), Box<dyn std::error::Error>> {
        let fixture = AttachmentFixture::new().await?;
        let outside = fixture._root_dir.path().join("outside");
        fs::create_dir_all(&outside)?;
        fs::write(outside.join("secret.bin"), b"secret")?;

        let link = fixture.root.join("escape-link");
        symlink(outside.join("secret.bin"), &link)?;

        let mut connection =
            sqlx::SqliteConnection::connect(fixture.db.path().to_str().ok_or("invalid path")?)
                .await?;
        for (guid, filename) in [
            ("at-escape", "escape-link"),
            ("at-traversal", "../outside/secret.bin"),
        ] {
            let stored_filename = if filename == "escape-link" {
                fs::canonicalize(&link)?.to_string_lossy().into_owned()
            } else {
                fixture.root.join(filename).to_string_lossy().into_owned()
            };
            sqlx::query(
                "INSERT INTO attachment (guid, original_guid, filename, total_bytes, transfer_state) \
                 VALUES (?1, ?1, ?2, 6, ?3)",
            )
            .bind(guid)
            .bind(stored_filename)
            .bind(TRANSFER_STATE_COMPLETE)
            .execute(&mut connection)
            .await?;
        }
        connection.close().await.ok();

        let pool = connect_pool(fixture.db.path()).await?;
        let app = fixture.app(pool)?;

        for guid in ["at-escape", "at-traversal"] {
            let (status, _, body) = response_bytes(
                app.clone(),
                Request::builder()
                    .uri(format!("/v1/attachments/{guid}/content"))
                    .body(Body::empty())?,
            )
            .await?;
            assert_eq!(status, StatusCode::NOT_FOUND);
            assert_safe_payload(&String::from_utf8(body)?);
        }
        Ok(())
    }

    #[tokio::test]
    async fn large_attachment_is_streamed() -> Result<(), Box<dyn std::error::Error>> {
        let fixture = AttachmentFixture::new().await?;
        let mut temp_file = tempfile::NamedTempFile::new_in(&fixture.root)?;
        let size = 256 * 1024;
        let mut written = 0usize;
        let chunk = vec![b'x'; 8192];
        while written < size {
            temp_file.write_all(&chunk)?;
            written += chunk.len();
        }
        temp_file.flush()?;
        let file_name = temp_file
            .path()
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or("missing temp file name")?
            .to_owned();

        fixture
            .seed_attachment(
                "at-large",
                &file_name,
                &vec![b'y'; size],
                TRANSFER_STATE_COMPLETE,
                Some("application/octet-stream"),
                Some("large.bin"),
            )
            .await?;

        let pool = connect_pool(fixture.db.path()).await?;
        let app = fixture.app(pool)?;

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/v1/attachments/at-large/content")
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(response.status(), StatusCode::OK);
        let mut body = response.into_body();
        let mut total = 0usize;
        while let Some(frame) = body.frame().await {
            let frame = frame?;
            if let Some(chunk) = frame.data_ref() {
                total += chunk.len();
            }
        }
        assert_eq!(total, size);
        Ok(())
    }
}
