use std::collections::BTreeMap;

use chrono::{DateTime, NaiveDate, NaiveDateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};

/// Vendor or unknown iCalendar properties preserved for round-trip.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtensionBag {
    #[serde(flatten)]
    pub properties: BTreeMap<String, String>,
}

/// RFC 5545 calendar event (VEVENT component).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CalendarEvent {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uid: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<EventStatus>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start: Option<EventDateTime>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end: Option<EventDateTime>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub organizer: Option<Organizer>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attendees: Vec<Attendee>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub alarms: Vec<Alarm>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recurrence_rule: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exception_dates: Vec<EventDateTime>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sequence: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extensions: Option<ExtensionBag>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventStatus {
    Confirmed,
    Tentative,
    Cancelled,
}

/// A point in time as RFC 5545 expresses it.
///
/// The four forms are kept distinct because they serialize differently and
/// cannot be recovered from a bare UTC instant: a floating time has no zone at
/// all, and a DATE is a calendar day rather than an instant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EventDateTime {
    /// A `VALUE=DATE` value. RFC 5545 §3.8.2.2 makes a DATE-valued `DTEND`
    /// **exclusive**: a one-day event ends on the following day.
    Date { date: NaiveDate },
    /// A UTC instant, written with a trailing `Z`.
    Utc { timestamp: DateTime<Utc> },
    /// A wall-clock time in a named zone, alongside the instant it denotes.
    Zoned {
        timestamp: DateTime<Utc>,
        tzid: String,
    },
    /// A wall-clock time with no zone. It denotes whatever local time the
    /// reader is in, so no instant is implied.
    Floating { local: NaiveDateTime },
}

impl EventDateTime {
    pub fn date(date: NaiveDate) -> Self {
        Self::Date { date }
    }

    pub fn utc(timestamp: DateTime<Utc>) -> Self {
        Self::Utc { timestamp }
    }

    pub fn zoned(timestamp: DateTime<Utc>, tzid: impl Into<String>) -> Self {
        Self::Zoned {
            timestamp,
            tzid: tzid.into(),
        }
    }

    pub fn floating(local: NaiveDateTime) -> Self {
        Self::Floating { local }
    }

    /// Best-effort UTC instant.
    ///
    /// A DATE resolves to midnight UTC on that day and a floating time is read
    /// as if it were UTC; neither is a statement about the originating zone.
    pub fn timestamp(&self) -> DateTime<Utc> {
        match self {
            Self::Date { date } => date
                .and_hms_opt(0, 0, 0)
                .map(|midnight| Utc.from_utc_datetime(&midnight))
                .unwrap_or_default(),
            Self::Utc { timestamp } | Self::Zoned { timestamp, .. } => *timestamp,
            Self::Floating { local } => Utc.from_utc_datetime(local),
        }
    }

    pub fn is_all_day(&self) -> bool {
        matches!(self, Self::Date { .. })
    }

    pub fn tzid(&self) -> Option<&str> {
        match self {
            Self::Zoned { tzid, .. } => Some(tzid),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Organizer {
    pub email: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Attendee {
    pub email: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub partstat: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rsvp: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Alarm {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trigger: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
}
