//! Extractors that keep malformed requests inside the typed error catalog.
//!
//! Axum's own `Json` and `Query` rejections answer with plain text, so a request body carrying an
//! unknown enum value escapes `ErrorCode` entirely and hands the client a bare string instead of
//! the `{ "error": { "code", "message", "details" } }` shape every other failure uses.

use axum::{
    extract::{
        FromRequest, FromRequestParts, Json, Query, Request,
        rejection::{JsonRejection, QueryRejection},
    },
    http::request::Parts,
};

use crate::api::error::{ApiError, ErrorCode};

/// `Json`, with rejections mapped into the typed error catalog.
pub struct ApiJson<T>(pub T);

impl<S, T> FromRequest<S> for ApiJson<T>
where
    Json<T>: FromRequest<S, Rejection = JsonRejection>,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request(request: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Json(value) = Json::<T>::from_request(request, state)
            .await
            .map_err(json_error)?;
        Ok(Self(value))
    }
}

/// `Query`, with rejections mapped into the typed error catalog.
pub struct ApiQuery<T>(pub T);

impl<S, T> FromRequestParts<S> for ApiQuery<T>
where
    Query<T>: FromRequestParts<S, Rejection = QueryRejection>,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Query(value) = Query::<T>::from_request_parts(parts, state)
            .await
            .map_err(query_error)?;
        Ok(Self(value))
    }
}

/// The rejection text describes the caller's own payload — which field, which value — so it is
/// safe to return and is the only thing that makes the error actionable.
fn json_error(rejection: JsonRejection) -> ApiError {
    let code = match rejection {
        // Well-formed JSON that does not match the schema: the request is understood but cannot
        // be processed.
        JsonRejection::JsonDataError(_) => ErrorCode::UnprocessableEntity,
        _ => ErrorCode::ValidationError,
    };
    ApiError::with_details(
        code,
        "request body could not be read",
        serde_json::json!({ "reason": rejection.body_text() }),
    )
}

fn query_error(rejection: QueryRejection) -> ApiError {
    ApiError::with_details(
        ErrorCode::InvalidParameter,
        "query parameters could not be read",
        serde_json::json!({ "reason": rejection.body_text() }),
    )
}

#[cfg(test)]
mod tests {
    use axum::{
        Router,
        body::Body,
        http::{Request, StatusCode},
        routing::{get, post},
    };
    use http_body_util::BodyExt;
    use serde::Deserialize;
    use tower::ServiceExt;

    use super::{ApiJson, ApiQuery};

    #[derive(Debug, Deserialize)]
    enum Span {
        #[serde(rename = "this")]
        This,
    }

    #[derive(Debug, Deserialize)]
    struct SpanBody {
        #[allow(dead_code)]
        span: Span,
    }

    #[derive(Debug, Deserialize)]
    struct SpanParams {
        #[allow(dead_code)]
        span: Span,
    }

    async fn send(
        app: Router,
        request: Request<Body>,
    ) -> Result<(StatusCode, serde_json::Value), Box<dyn std::error::Error>> {
        let response = app.oneshot(request).await?;
        let status = response.status();
        let bytes = response.into_body().collect().await?.to_bytes();
        let payload = serde_json::from_slice(&bytes)
            .unwrap_or(serde_json::json!({ "raw": String::from_utf8_lossy(&bytes) }));
        Ok((status, payload))
    }

    fn app() -> Router {
        Router::new()
            .route(
                "/body",
                post(|ApiJson(_): ApiJson<SpanBody>| async { "ok" }),
            )
            .route(
                "/query",
                get(|ApiQuery(_): ApiQuery<SpanParams>| async { "ok" }),
            )
    }

    #[tokio::test]
    async fn an_unknown_body_enum_value_stays_in_the_error_catalog()
    -> Result<(), Box<dyn std::error::Error>> {
        let (status, body) = send(
            app(),
            Request::builder()
                .method("POST")
                .uri("/body")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"span":"all"}"#))?,
        )
        .await?;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(body["error"]["code"], "unprocessable_entity");
        assert!(body["error"]["details"]["reason"].is_string());
        Ok(())
    }

    #[tokio::test]
    async fn malformed_json_is_a_bad_request() -> Result<(), Box<dyn std::error::Error>> {
        let (status, body) = send(
            app(),
            Request::builder()
                .method("POST")
                .uri("/body")
                .header("content-type", "application/json")
                .body(Body::from("{not json"))?,
        )
        .await?;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body["error"]["code"], "validation_error");
        Ok(())
    }

    #[tokio::test]
    async fn an_unknown_query_enum_value_stays_in_the_error_catalog()
    -> Result<(), Box<dyn std::error::Error>> {
        let (status, body) = send(
            app(),
            Request::builder()
                .uri("/query?span=all")
                .body(Body::empty())?,
        )
        .await?;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body["error"]["code"], "invalid_parameter");
        assert!(body["error"]["details"]["reason"].is_string());
        Ok(())
    }

    #[tokio::test]
    async fn a_valid_value_still_reaches_the_handler() -> Result<(), Box<dyn std::error::Error>> {
        let (status, _) = send(
            app(),
            Request::builder()
                .uri("/query?span=this")
                .body(Body::empty())?,
        )
        .await?;
        assert_eq!(status, StatusCode::OK);
        Ok(())
    }
}
