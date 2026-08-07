//! Shared attachment/media byte serving (Range, ETag, HEAD, 206/304/416).

use std::path::PathBuf;

use axum::{
    body::Body,
    http::{HeaderMap, HeaderValue, Request, StatusCode, header},
    response::Response,
};
use tower::ServiceExt;
use tower_http::services::ServeFile;

use super::{
    blocking_io::BlockingIoPool,
    error::{ApiError, ErrorCode},
};
use crate::messages::attachment_path::{
    FileValidators, file_validators_async, if_none_match_satisfied,
};

/// Inputs for serving a previously validated on-disk media object.
pub struct ServeMedia {
    pub path: PathBuf,
    pub content_type: String,
    pub content_disposition: String,
    /// Mapped when ServeFile returns 404 or validators cannot be read.
    pub unavailable: ErrorCode,
}

pub async fn serve_media_bytes(
    blocking_io: &BlockingIoPool,
    media: ServeMedia,
    request: Request<Body>,
) -> Result<Response, ApiError> {
    let validators = file_validators_async(blocking_io, media.path.clone())
        .await
        .map_err(|_| ApiError::internal("blocking attachment metadata read failed"))?
        .map_err(|_| ApiError::new(media.unavailable))?;

    if let Some(if_none_match) = request.headers().get(header::IF_NONE_MATCH)
        && let Ok(value) = if_none_match.to_str()
        && if_none_match_satisfied(value, &validators.etag)
    {
        return Response::builder()
            .status(StatusCode::NOT_MODIFIED)
            .header(header::ETAG, &validators.etag)
            .header(header::LAST_MODIFIED, &validators.last_modified)
            .body(Body::empty())
            .map_err(|_| ApiError::internal("failed to build not modified response"));
    }

    let serve_file = ServeFile::new(media.path);
    let response = serve_file
        .oneshot(request)
        .await
        .map_err(|_| ApiError::internal("attachment delivery failed"))?
        .map(Body::new);

    map_served_response(
        response,
        &media.content_type,
        &media.content_disposition,
        &validators,
        media.unavailable,
    )
}

fn map_served_response(
    mut response: Response<Body>,
    content_type: &str,
    disposition: &str,
    validators: &FileValidators,
    unavailable: ErrorCode,
) -> Result<Response<Body>, ApiError> {
    let status = response.status();
    if status == StatusCode::RANGE_NOT_SATISFIABLE {
        return Err(ApiError::range_not_satisfiable(
            "requested byte range is not satisfiable",
        ));
    }
    if status == StatusCode::NOT_FOUND {
        return Err(ApiError::new(unavailable));
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

pub fn copy_conditional_headers(source: &HeaderMap, destination: &mut HeaderMap) {
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
