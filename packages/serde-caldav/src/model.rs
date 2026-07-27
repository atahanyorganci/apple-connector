use serde::{Deserialize, Serialize};
use serde_icalendar::CalendarEvent;

/// CalDAV calendar-object resource wrapping an embedded ICS payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CalDavCalendarObject {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub href: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub etag: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    pub event: CalendarEvent,
}

/// CalDAV calendar collection resource.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CalDavCalendarResource {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub href: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub displayname: Option<String>,
}

/// CalDAV multistatus response envelope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CalDavMultistatus {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub responses: Vec<CalDavResponse>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CalDavResponse {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub href: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub calendar_object: Option<CalDavCalendarObject>,
}
