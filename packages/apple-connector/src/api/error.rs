//! Structured API errors mapped to HTTP status codes.

#![allow(dead_code)]

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    ValidationError,
    NotFound,
    RangeNotSatisfiable,
    ServiceUnavailable,
    Forbidden,
    Conflict,
    InternalError,
}

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
    pub fn validation(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            body: ErrorBody {
                code: ErrorCode::ValidationError,
                message: message.into(),
                details: None,
            },
        }
    }

    pub fn validation_with_details(message: impl Into<String>, details: serde_json::Value) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            body: ErrorBody {
                code: ErrorCode::ValidationError,
                message: message.into(),
                details: Some(details),
            },
        }
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            body: ErrorBody {
                code: ErrorCode::NotFound,
                message: message.into(),
                details: None,
            },
        }
    }

    pub fn range_not_satisfiable(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::RANGE_NOT_SATISFIABLE,
            body: ErrorBody {
                code: ErrorCode::RangeNotSatisfiable,
                message: message.into(),
                details: None,
            },
        }
    }

    pub fn service_unavailable(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::SERVICE_UNAVAILABLE,
            body: ErrorBody {
                code: ErrorCode::ServiceUnavailable,
                message: message.into(),
                details: None,
            },
        }
    }

    pub fn forbidden(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::FORBIDDEN,
            body: ErrorBody {
                code: ErrorCode::Forbidden,
                message: message.into(),
                details: None,
            },
        }
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::CONFLICT,
            body: ErrorBody {
                code: ErrorCode::Conflict,
                message: message.into(),
                details: None,
            },
        }
    }

    pub fn unprocessable(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::UNPROCESSABLE_ENTITY,
            body: ErrorBody {
                code: ErrorCode::ValidationError,
                message: message.into(),
                details: None,
            },
        }
    }

    pub fn unprocessable_with_details(
        message: impl Into<String>,
        details: serde_json::Value,
    ) -> Self {
        Self {
            status: StatusCode::UNPROCESSABLE_ENTITY,
            body: ErrorBody {
                code: ErrorCode::ValidationError,
                message: message.into(),
                details: Some(details),
            },
        }
    }

    pub fn eventkit_unavailable() -> Self {
        Self::service_unavailable(
            "EventKit is unavailable on this platform or could not be initialized",
        )
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            body: ErrorBody {
                code: ErrorCode::InternalError,
                message: message.into(),
                details: None,
            },
        }
    }

    pub fn method_not_allowed() -> Self {
        Self {
            status: StatusCode::METHOD_NOT_ALLOWED,
            body: ErrorBody {
                code: ErrorCode::ValidationError,
                message: "method not allowed".to_owned(),
                details: None,
            },
        }
    }

    pub fn status(&self) -> StatusCode {
        self.status
    }

    pub fn body(&self) -> &ErrorBody {
        &self.body
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(ErrorResponse { error: self.body })).into_response()
    }
}
