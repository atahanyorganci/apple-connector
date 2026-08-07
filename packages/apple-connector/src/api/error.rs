//! Structured API errors mapped to HTTP status codes.

#![allow(dead_code)]

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use utoipa::ToSchema;

pub use super::error_codes::ErrorCode;

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ErrorBody {
    pub code: ErrorCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ErrorResponse {
    pub error: ErrorBody,
}

#[derive(Debug, Clone)]
pub struct ApiError {
    status: StatusCode,
    body: ErrorBody,
}

impl ApiError {
    pub fn new(code: ErrorCode) -> Self {
        Self::with_message(code, code.default_message())
    }

    pub fn with_message(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            status: code.http_status(),
            body: ErrorBody {
                code,
                message: message.into(),
                details: None,
            },
        }
    }

    pub fn with_details(
        code: ErrorCode,
        message: impl Into<String>,
        details: serde_json::Value,
    ) -> Self {
        Self {
            status: code.http_status(),
            body: ErrorBody {
                code,
                message: message.into(),
                details: Some(details),
            },
        }
    }

    /// Transitional helper — prefer typed `ErrorCode` constructors in new code.
    pub fn validation(message: impl Into<String>) -> Self {
        Self::with_message(ErrorCode::ValidationError, message)
    }

    /// Transitional helper — prefer typed `ErrorCode` constructors in new code.
    pub fn validation_with_details(message: impl Into<String>, details: serde_json::Value) -> Self {
        Self::with_details(ErrorCode::ValidationError, message, details)
    }

    /// Transitional helper — prefer domain-specific `*_not_found` codes.
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::with_message(ErrorCode::ResourceNotFound, message)
    }

    pub fn range_not_satisfiable(message: impl Into<String>) -> Self {
        Self::with_message(ErrorCode::ByteRangeNotSatisfiable, message)
    }

    /// Transitional helper — prefer domain-specific unavailable codes.
    pub fn service_unavailable(message: impl Into<String>) -> Self {
        Self::with_message(ErrorCode::ServiceUnavailable, message)
    }

    /// Transitional helper — prefer typed permission codes.
    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::with_message(ErrorCode::Forbidden, message)
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self::with_message(ErrorCode::Conflict, message)
    }

    pub fn unprocessable(message: impl Into<String>) -> Self {
        Self::with_message(ErrorCode::UnprocessableEntity, message)
    }

    pub fn unprocessable_with_details(
        message: impl Into<String>,
        details: serde_json::Value,
    ) -> Self {
        Self::with_details(ErrorCode::UnprocessableEntity, message, details)
    }

    pub fn eventkit_unavailable() -> Self {
        Self::new(ErrorCode::EventkitUnavailable)
    }

    pub fn contacts_unavailable() -> Self {
        Self::new(ErrorCode::ContactsUnavailable)
    }

    /// Prefer logging the cause and returning this without leaking internals.
    pub fn internal(_message: impl Into<String>) -> Self {
        Self::new(ErrorCode::InternalError)
    }

    /// Map database errors without leaking driver details. Query bounds surface as 504.
    pub fn from_sqlx(error: sqlx::Error) -> Self {
        match error {
            sqlx::Error::PoolTimedOut => Self::new(ErrorCode::QueryTimeout),
            _ => Self::new(ErrorCode::InternalError),
        }
    }

    pub fn method_not_allowed() -> Self {
        Self::new(ErrorCode::MethodNotAllowed)
    }

    pub fn request_timeout() -> Self {
        Self::new(ErrorCode::RequestTimeout)
    }

    pub fn status(&self) -> StatusCode {
        self.status
    }

    pub fn body(&self) -> &ErrorBody {
        &self.body
    }
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.body.message)
    }
}

impl std::error::Error for ApiError {}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(ErrorResponse { error: self.body })).into_response()
    }
}
