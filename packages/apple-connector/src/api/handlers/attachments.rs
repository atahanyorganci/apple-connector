use axum::{
    Json,
    body::Body,
    extract::{Request, State},
    http::{HeaderMap, HeaderValue, Method, StatusCode, header},
    response::Response,
};
use tower::ServiceExt;
use tower_http::services::ServeFile;

use super::health::require_messages_db;
use crate::{
    api::{
        dto::{AttachmentDetailDto, convert::attachment_detail_to_dto},
        error::ApiError,
        params::AttachmentGuidPath,
        router::AppState,
    },
    db::run_timed_query,
    messages::{
        Attachment,
        attachment_path::{
            content_disposition, file_validators, if_none_match_satisfied, resolve_content_type,
            sanitize_download_filename, validate_attachment_path,
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
    axum::extract::Path(AttachmentGuidPath { guid }): axum::extract::Path<AttachmentGuidPath>,
) -> Result<Json<AttachmentDetailDto>, ApiError> {
    let pool = require_messages_db(&state.messages_db)?;
    let attachment_root = state.attachment_root.as_ref();
    let attachment = run_timed_query(|| async {
        MessageRepository::new(pool)
            .get_attachment_by_guid(&guid, attachment_root)
            .await
    })
    .await
    .map_err(|error| ApiError::internal(error.to_string()))?
    .ok_or_else(|| ApiError::not_found(format!("attachment {guid} not found")))?;

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
    axum::extract::Path(AttachmentGuidPath { guid }): axum::extract::Path<AttachmentGuidPath>,
    request: Request,
) -> Result<Response, ApiError> {
    let (attachment, validated_path) = resolve_content_attachment(&state, &guid).await?;
    serve_attachment_bytes(attachment, validated_path, request).await
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
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let (attachment, validated_path) = resolve_content_attachment(&state, &guid).await?;
    let mut request = Request::builder()
        .method(Method::HEAD)
        .uri("/")
        .body(Body::empty())
        .expect("head request");
    copy_conditional_headers(&headers, request.headers_mut());
    serve_attachment_bytes(attachment, validated_path, request).await
}

async fn resolve_content_attachment(
    state: &AppState,
    guid: &str,
) -> Result<(Attachment, std::path::PathBuf), ApiError> {
    let pool = require_messages_db(&state.messages_db)?;
    let attachment_root = state.attachment_root.as_ref();
    let attachment = run_timed_query(|| async {
        MessageRepository::new(pool)
            .get_attachment_by_guid(guid, attachment_root)
            .await
    })
    .await
    .map_err(|error| ApiError::internal(error.to_string()))?
    .ok_or_else(|| ApiError::not_found(format!("attachment {guid} not found")))?;

    if !attachment.transfer_complete {
        return Err(ApiError::not_found(format!(
            "attachment {guid} is not available"
        )));
    }

    let filename = attachment
        .filename
        .as_deref()
        .ok_or_else(|| ApiError::not_found(format!("attachment {guid} is not available")))?;

    let validated = validate_attachment_path(state.attachment_root.as_ref(), filename)
        .map_err(|_| ApiError::not_found(format!("attachment {guid} is not available")))?;

    Ok((attachment, validated.canonical_path))
}

async fn serve_attachment_bytes(
    attachment: Attachment,
    path: std::path::PathBuf,
    request: Request,
) -> Result<Response, ApiError> {
    let filename = sanitize_download_filename(
        attachment.transfer_name.as_deref(),
        attachment.mime_type.as_deref(),
        &attachment.guid,
    );
    let content_type = resolve_content_type(attachment.mime_type.as_deref());
    let disposition = content_disposition(&attachment.kind, &filename);
    let validators = file_validators(&path).map_err(|_| {
        ApiError::not_found(format!("attachment {} is not available", attachment.guid))
    })?;

    if let Some(if_none_match) = request.headers().get(header::IF_NONE_MATCH)
        && let Ok(value) = if_none_match.to_str()
        && if_none_match_satisfied(value, &validators.etag)
    {
        return Ok(Response::builder()
            .status(StatusCode::NOT_MODIFIED)
            .header(header::ETAG, &validators.etag)
            .header(header::LAST_MODIFIED, &validators.last_modified)
            .body(Body::empty())
            .expect("304 response"));
    }

    let serve_file = ServeFile::new(path);
    let response = serve_file
        .oneshot(request)
        .await
        .map_err(|_| ApiError::internal("attachment delivery failed"))?
        .map(Body::new);

    map_served_response(response, &content_type, &disposition, &validators)
}

fn map_served_response(
    mut response: Response<Body>,
    content_type: &str,
    disposition: &str,
    validators: &crate::messages::attachment_path::FileValidators,
) -> Result<Response<Body>, ApiError> {
    let status = response.status();
    if status == StatusCode::RANGE_NOT_SATISFIABLE {
        return Err(ApiError::range_not_satisfiable(
            "requested byte range is not satisfiable",
        ));
    }
    if status == StatusCode::NOT_FOUND {
        return Err(ApiError::not_found("attachment is not available"));
    }
    if status.is_server_error() {
        return Err(ApiError::internal("attachment delivery failed"));
    }

    let headers = response.headers_mut();
    if let Ok(value) = HeaderValue::from_str(content_type) {
        headers.insert(header::CONTENT_TYPE, value);
    }
    if let Ok(value) = HeaderValue::from_str(disposition) {
        headers.insert(header::CONTENT_DISPOSITION, value);
    }
    if let Ok(value) = HeaderValue::from_str(&validators.etag) {
        headers.insert(header::ETAG, value);
    }
    if let Ok(value) = HeaderValue::from_str(&validators.last_modified) {
        headers.insert(header::LAST_MODIFIED, value);
    }
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(header::ACCEPT_RANGES, HeaderValue::from_static("bytes"));

    Ok(response)
}

fn copy_conditional_headers(source: &HeaderMap, destination: &mut HeaderMap) {
    for name in [
        header::IF_NONE_MATCH,
        header::IF_MODIFIED_SINCE,
        header::RANGE,
    ] {
        if let Some(value) = source.get(&name) {
            destination.insert(name, value.clone());
        }
    }
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
        async fn new() -> Self {
            let root_dir = tempfile::tempdir().expect("attachment root tempdir");
            let root = root_dir.path().join("Attachments");
            fs::create_dir_all(&root).expect("create attachments dir");

            let db = FixtureDb::empty().await.expect("empty fixture");
            let mut connection = sqlx::SqliteConnection::connect(db.path().to_str().unwrap())
                .await
                .expect("connect");

            for statement in [
                "DROP TRIGGER IF EXISTS verify_chat_insert",
                "DROP TRIGGER IF EXISTS verify_chat_update",
            ] {
                sqlx::query(statement)
                    .execute(&mut connection)
                    .await
                    .expect("drop trigger");
            }

            connection.close().await.ok();
            Self {
                db,
                root,
                _root_dir: root_dir,
            }
        }

        fn app(&self, pool: sqlx::SqlitePool) -> axum::Router {
            let root = canonicalize_attachment_root(&self.root).expect("canonical root");
            router(AppState::with_attachment_roots(
                Some(pool),
                None,
                None,
                root,
                PathBuf::from("/var/empty/reminders-attachments"),
                PathBuf::from("/var/empty/notes-attachments"),
            ))
        }

        async fn seed_attachment(
            &self,
            guid: &str,
            filename: &str,
            bytes: &[u8],
            transfer_state: i64,
            mime_type: Option<&str>,
            transfer_name: Option<&str>,
        ) {
            let file_path = self.root.join(filename);
            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent).expect("parent dirs");
            }
            fs::write(&file_path, bytes).expect("write attachment bytes");
            let stored_filename = fs::canonicalize(&file_path)
                .expect("canonical file")
                .to_string_lossy()
                .into_owned();

            let mut connection = sqlx::SqliteConnection::connect(self.db.path().to_str().unwrap())
                .await
                .expect("connect");
            sqlx::query(
                "INSERT INTO attachment (guid, original_guid, filename, mime_type, transfer_name, total_bytes, transfer_state) \
                 VALUES (?1, ?1, ?2, ?3, ?4, ?5, ?6)",
            )
            .bind(guid)
            .bind(stored_filename)
            .bind(mime_type)
            .bind(transfer_name)
            .bind(bytes.len() as i64)
            .bind(transfer_state)
            .execute(&mut connection)
            .await
            .expect("insert attachment");
            connection.close().await.ok();
        }
    }

    async fn response_bytes(
        app: axum::Router,
        request: Request<Body>,
    ) -> (StatusCode, HeaderMap, Vec<u8>) {
        let response = app.oneshot(request).await.expect("response");
        let status = response.status();
        let headers = response.headers().clone();
        let body = response
            .into_body()
            .collect()
            .await
            .expect("body")
            .to_bytes()
            .to_vec();
        (status, headers, body)
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
    async fn metadata_exposes_availability_and_content_url_without_paths() {
        let fixture = AttachmentFixture::new().await;
        fixture
            .seed_attachment(
                "at-meta",
                "photo.jpg",
                b"jpeg-bytes",
                TRANSFER_STATE_COMPLETE,
                Some("image/jpeg"),
                Some("vacation.jpg"),
            )
            .await;

        let pool = connect_pool(fixture.db.path()).await.expect("pool");
        let app = fixture.app(pool);
        let (status, headers, body) = response_bytes(
            app,
            Request::builder()
                .uri("/v1/attachments/at-meta")
                .body(Body::empty())
                .expect("request"),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        let payload = String::from_utf8(body).expect("utf-8");
        assert_safe_payload(&payload);
        let json: serde_json::Value = serde_json::from_str(&payload).expect("json");
        assert_eq!(json["guid"], "at-meta");
        assert_eq!(json["present_on_disk"], true);
        assert_eq!(json["transfer_complete"], true);
        assert_eq!(json["content_url"], "/v1/attachments/at-meta/content");
        assert!(json.get("filename").is_none());
        assert!(headers.get("content-disposition").is_none());
    }

    #[tokio::test]
    async fn get_head_range_and_conditional_requests_work() {
        let fixture = AttachmentFixture::new().await;
        fixture
            .seed_attachment(
                "at-bytes",
                "note.txt",
                b"0123456789",
                TRANSFER_STATE_COMPLETE,
                None,
                Some("note.txt"),
            )
            .await;

        let pool = connect_pool(fixture.db.path()).await.expect("pool");
        let app = fixture.app(pool);

        let (status, headers, body) = response_bytes(
            app.clone(),
            Request::builder()
                .uri("/v1/attachments/at-bytes/content")
                .body(Body::empty())
                .expect("request"),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, b"0123456789");
        assert_eq!(
            headers.get("content-type").unwrap(),
            "application/octet-stream"
        );
        assert_eq!(headers.get("accept-ranges").unwrap(), "bytes");
        assert_eq!(headers.get("x-content-type-options").unwrap(), "nosniff");
        assert!(
            headers
                .get("content-disposition")
                .unwrap()
                .to_str()
                .unwrap()
                .contains("note.txt")
        );
        let etag = headers.get("etag").unwrap().clone();

        let (status, headers, body) = response_bytes(
            app.clone(),
            Request::builder()
                .uri("/v1/attachments/at-bytes/content")
                .header("Range", "bytes=0-4")
                .body(Body::empty())
                .expect("request"),
        )
        .await;
        assert_eq!(status, StatusCode::PARTIAL_CONTENT);
        assert_eq!(body, b"01234");
        assert!(
            headers
                .get("content-range")
                .unwrap()
                .to_str()
                .unwrap()
                .contains("0-4/10")
        );

        let (status, _, body) = response_bytes(
            app.clone(),
            Request::builder()
                .method("HEAD")
                .uri("/v1/attachments/at-bytes/content")
                .body(Body::empty())
                .expect("request"),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert!(body.is_empty());

        let (status, _, body) = response_bytes(
            app.clone(),
            Request::builder()
                .uri("/v1/attachments/at-bytes/content")
                .header("If-None-Match", etag)
                .body(Body::empty())
                .expect("request"),
        )
        .await;
        assert_eq!(status, StatusCode::NOT_MODIFIED);
        assert!(body.is_empty());
    }

    #[tokio::test]
    async fn invalid_range_missing_file_and_incomplete_transfer_are_handled() {
        let fixture = AttachmentFixture::new().await;
        fixture
            .seed_attachment(
                "at-range",
                "range.bin",
                b"12345",
                TRANSFER_STATE_COMPLETE,
                None,
                None,
            )
            .await;
        fixture
            .seed_attachment("at-incomplete", "pending.bin", b"pending", 1, None, None)
            .await;
        fixture
            .seed_attachment(
                "at-missing",
                "missing.bin",
                b"missing",
                TRANSFER_STATE_COMPLETE,
                None,
                None,
            )
            .await;

        fs::remove_file(fixture.root.join("missing.bin")).expect("remove missing file");

        let pool = connect_pool(fixture.db.path()).await.expect("pool");
        let app = fixture.app(pool);

        let (status, _, body) = response_bytes(
            app.clone(),
            Request::builder()
                .uri("/v1/attachments/at-range/content")
                .header("Range", "bytes=100-200")
                .body(Body::empty())
                .expect("request"),
        )
        .await;
        assert_eq!(status, StatusCode::RANGE_NOT_SATISFIABLE);
        let payload = String::from_utf8(body).expect("utf-8");
        assert!(payload.contains("range_not_satisfiable"));

        for guid in ["at-incomplete", "at-missing", "at-absent"] {
            let uri = format!("/v1/attachments/{guid}/content");
            let (status, _, body) = response_bytes(
                app.clone(),
                Request::builder()
                    .uri(uri)
                    .body(Body::empty())
                    .expect("request"),
            )
            .await;
            assert_eq!(status, StatusCode::NOT_FOUND, "guid {guid}");
            assert_safe_payload(&String::from_utf8(body).expect("utf-8"));
        }
    }

    #[tokio::test]
    async fn traversal_and_symlink_paths_are_denied() {
        let fixture = AttachmentFixture::new().await;
        let outside = fixture._root_dir.path().join("outside");
        fs::create_dir_all(&outside).expect("outside dir");
        fs::write(outside.join("secret.bin"), b"secret").expect("secret file");

        let link = fixture.root.join("escape-link");
        symlink(outside.join("secret.bin"), &link).expect("symlink");

        let mut connection = sqlx::SqliteConnection::connect(fixture.db.path().to_str().unwrap())
            .await
            .expect("connect");
        for (guid, filename) in [
            ("at-escape", "escape-link"),
            ("at-traversal", "../outside/secret.bin"),
        ] {
            let stored_filename = if filename == "escape-link" {
                fs::canonicalize(&link)
                    .expect("canonical link")
                    .to_string_lossy()
                    .into_owned()
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
            .await
            .expect("insert attachment");
        }
        connection.close().await.ok();

        let pool = connect_pool(fixture.db.path()).await.expect("pool");
        let app = fixture.app(pool);

        for guid in ["at-escape", "at-traversal"] {
            let (status, _, body) = response_bytes(
                app.clone(),
                Request::builder()
                    .uri(format!("/v1/attachments/{guid}/content"))
                    .body(Body::empty())
                    .expect("request"),
            )
            .await;
            assert_eq!(status, StatusCode::NOT_FOUND);
            assert_safe_payload(&String::from_utf8(body).expect("utf-8"));
        }
    }

    #[tokio::test]
    async fn large_attachment_is_streamed() {
        let fixture = AttachmentFixture::new().await;
        let mut temp_file = tempfile::NamedTempFile::new_in(&fixture.root).expect("temp file");
        let size = 256 * 1024;
        let mut written = 0usize;
        let chunk = vec![b'x'; 8192];
        while written < size {
            temp_file.write_all(&chunk).expect("write chunk");
            written += chunk.len();
        }
        temp_file.flush().expect("flush");
        let file_name = temp_file
            .path()
            .file_name()
            .and_then(|name| name.to_str())
            .expect("file name")
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
            .await;

        let pool = connect_pool(fixture.db.path()).await.expect("pool");
        let app = fixture.app(pool);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/v1/attachments/at-large/content")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::OK);
        let mut body = response.into_body();
        let mut total = 0usize;
        while let Some(frame) = body.frame().await {
            let frame = frame.expect("frame");
            if let Some(chunk) = frame.data_ref() {
                total += chunk.len();
            }
        }
        assert_eq!(total, size);
    }
}
