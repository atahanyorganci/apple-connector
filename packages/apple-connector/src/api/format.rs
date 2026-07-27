use axum::http::header::{ACCEPT, CONTENT_TYPE};
use axum::http::HeaderMap;

use crate::api::error::ApiError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponseFormat {
    Json,
    Ics,
    CalDav,
}

impl ResponseFormat {
    pub fn content_type(self) -> &'static str {
        match self {
            Self::Json => "application/json",
            Self::Ics => "text/calendar; charset=utf-8",
            Self::CalDav => "application/caldav+xml; charset=utf-8",
        }
    }
}

pub fn resolve_format(headers: &HeaderMap, format_query: Option<&str>) -> Result<ResponseFormat, ApiError> {
    if let Some(format) = format_query {
        return match format.to_ascii_lowercase().as_str() {
            "json" => Ok(ResponseFormat::Json),
            "ics" | "icalendar" => Ok(ResponseFormat::Ics),
            "caldav" | "xml" => Ok(ResponseFormat::CalDav),
            _ => Err(ApiError::validation_with_details(
                "format must be one of json, ics, or caldav",
                serde_json::json!({ "field": "format" }),
            )),
        };
    }

    if let Some(accept) = headers.get(ACCEPT).and_then(|v| v.to_str().ok()) {
        for part in accept.split(',') {
            let part = part.split(';').next().unwrap_or(part).trim();
            if part.eq_ignore_ascii_case("application/caldav+xml") {
                return Ok(ResponseFormat::CalDav);
            }
            if part.eq_ignore_ascii_case("text/calendar") {
                return Ok(ResponseFormat::Ics);
            }
            if part.eq_ignore_ascii_case("application/json") || part == "*/*" {
                return Ok(ResponseFormat::Json);
            }
        }
    }

    Ok(ResponseFormat::Json)
}

pub fn parse_request_format(headers: &HeaderMap) -> Result<ResponseFormat, ApiError> {
    let content_type = headers
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();
    if content_type.contains("caldav") {
        return Ok(ResponseFormat::CalDav);
    }
    if content_type.contains("text/calendar") {
        return Ok(ResponseFormat::Ics);
    }
    Err(ApiError::validation_with_details(
        "Content-Type must be text/calendar or application/caldav+xml",
        serde_json::json!({ "field": "content-type" }),
    ))
}
