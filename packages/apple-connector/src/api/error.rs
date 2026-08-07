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

    /// Prefer typed `ErrorCode` constructors (`new` / `with_message` / `with_details`).
    pub fn range_not_satisfiable(message: impl Into<String>) -> Self {
        Self::with_message(ErrorCode::ByteRangeNotSatisfiable, message)
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
