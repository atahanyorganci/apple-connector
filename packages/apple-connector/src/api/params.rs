//! Query, path, and header parameter types for the OpenAPI contract.

#![allow(dead_code)]

use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};

use super::{
    dto::pagination::{DEFAULT_PAGE_LIMIT, MAX_PAGE_LIMIT},
    error::ApiError,
};

pub const CURSOR_VERSION: &str = "v1";

#[derive(Debug, Clone, Deserialize, IntoParams, ToSchema)]
#[into_params(parameter_in = Query, style = Form)]
pub struct PageParams {
    /// Maximum number of items to return. Defaults to 50. Keyset pagination only; no offsets.
    #[param(minimum = 1, maximum = 200, default = 50, example = 50)]
    pub limit: Option<u32>,

    /// URL-safe versioned cursor for the next page. Results are ordered newest first.
    #[param(example = "v1.eyJkYXRlIjoxNzA0MDk2MDAwfQ")]
    pub cursor: Option<String>,
}

impl PageParams {
    pub fn validated_limit(&self) -> Result<u32, ApiError> {
        let limit = self.limit.unwrap_or(DEFAULT_PAGE_LIMIT);
        if !(1..=MAX_PAGE_LIMIT).contains(&limit) {
            return Err(ApiError::validation_with_details(
                format!("limit must be between 1 and {MAX_PAGE_LIMIT}"),
                serde_json::json!({
                    "field": "limit",
                    "minimum": 1,
                    "maximum": MAX_PAGE_LIMIT,
                    "default": DEFAULT_PAGE_LIMIT,
                }),
            ));
        }
        Ok(limit)
    }

    pub fn validated_cursor(&self) -> Result<Option<&str>, ApiError> {
        match &self.cursor {
            None => Ok(None),
            Some(cursor) if cursor.starts_with(&format!("{CURSOR_VERSION}.")) => Ok(Some(cursor)),
            Some(_) => Err(ApiError::validation_with_details(
                format!("cursor must start with `{CURSOR_VERSION}.`"),
                serde_json::json!({
                    "field": "cursor",
                    "expected_prefix": format!("{CURSOR_VERSION}."),
                }),
            )),
        }
    }
}

#[derive(Debug, Clone, Deserialize, IntoParams, ToSchema)]
#[into_params(parameter_in = Path)]
pub struct ChatIdPath {
    /// Internal chat row identifier.
    #[param(example = 42)]
    pub chat_id: i64,
}

#[derive(Debug, Clone, Deserialize, IntoParams, ToSchema)]
#[into_params(parameter_in = Path)]
pub struct MessageGuidPath {
    /// Message GUID.
    #[param(example = "A1B2C3D4-E5F6-7890-ABCD-EF1234567890")]
    pub guid: String,
}

#[derive(Debug, Clone, Deserialize, IntoParams, ToSchema)]
#[into_params(parameter_in = Path)]
pub struct AttachmentGuidPath {
    /// Attachment GUID.
    #[param(example = "at_0_1234567890ABCDEF")]
    pub guid: String,
}

#[derive(Debug, Clone, Deserialize, IntoParams, ToSchema)]
#[into_params(parameter_in = Header)]
pub struct ConditionalRequestHeaders {
    /// Validator for conditional GET/HEAD requests.
    #[param(rename = "If-None-Match", example = "\"abc123\"")]
    pub if_none_match: Option<String>,

    /// Timestamp validator for conditional GET/HEAD requests.
    #[param(
        rename = "If-Modified-Since",
        example = "Mon, 01 Jan 2024 12:00:00 GMT"
    )]
    pub if_modified_since: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::PageParams;

    #[test]
    fn default_limit_is_50_and_max_is_200() {
        let page = PageParams {
            limit: None,
            cursor: None,
        };
        assert_eq!(page.validated_limit().expect("default limit"), 50);

        let invalid_high = PageParams {
            limit: Some(201),
            cursor: None,
        };
        assert!(invalid_high.validated_limit().is_err());

        let invalid_zero = PageParams {
            limit: Some(0),
            cursor: None,
        };
        assert!(invalid_zero.validated_limit().is_err());
    }

    #[test]
    fn cursor_must_be_versioned_and_url_safe_prefix() {
        let valid = PageParams {
            limit: None,
            cursor: Some("v1.c2afe".to_owned()),
        };
        assert_eq!(valid.validated_cursor().expect("cursor"), Some("v1.c2afe"));

        let invalid = PageParams {
            limit: None,
            cursor: Some("offset:10".to_owned()),
        };
        assert!(invalid.validated_cursor().is_err());
    }
}

#[derive(Debug, Clone, Deserialize, IntoParams, ToSchema)]
#[into_params(parameter_in = Header)]
pub struct RangeRequestHeader {
    /// Byte range for partial content requests.
    #[param(rename = "Range", example = "bytes=0-1023")]
    pub range: Option<String>,
}
