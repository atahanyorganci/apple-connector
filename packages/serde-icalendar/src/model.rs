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

/// Build the RFC-token enums used for ROLE, PARTSTAT and CUTYPE.
///
/// Each keeps an `Other` arm so a token this crate does not know is preserved
/// verbatim rather than dropped, and serializes as the token itself rather than
/// as a Rust variant name.
macro_rules! rfc_token_enum {
    ($(#[$meta:meta])* $name:ident { $($variant:ident => $token:literal),+ $(,)? }) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
        #[serde(from = "String", into = "String")]
        pub enum $name {
            $($variant,)+
            /// A token outside the RFC set, preserved as written.
            Other(String),
        }

        impl $name {
            pub fn as_token(&self) -> &str {
                match self {
                    $(Self::$variant => $token,)+
                    Self::Other(value) => value,
                }
            }

            pub fn from_token(value: &str) -> Self {
                match value.to_ascii_uppercase().as_str() {
                    $($token => Self::$variant,)+
                    _ => Self::Other(value.to_owned()),
                }
            }
        }

        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self::from_token(&value)
            }
        }

        impl From<$name> for String {
            fn from(value: $name) -> Self {
                value.as_token().to_owned()
            }
        }
    };
}

rfc_token_enum! {
    /// ROLE parameter, RFC 5545 §3.2.16.
    Role {
        Chair => "CHAIR",
        ReqParticipant => "REQ-PARTICIPANT",
        OptParticipant => "OPT-PARTICIPANT",
        NonParticipant => "NON-PARTICIPANT",
    }
}

rfc_token_enum! {
    /// PARTSTAT parameter, RFC 5545 §3.2.12.
    ParticipationStatus {
        NeedsAction => "NEEDS-ACTION",
        Accepted => "ACCEPTED",
        Declined => "DECLINED",
        Tentative => "TENTATIVE",
        Delegated => "DELEGATED",
        Completed => "COMPLETED",
        InProcess => "IN-PROCESS",
    }
}

rfc_token_enum! {
    /// CUTYPE parameter, RFC 5545 §3.2.3.
    CalendarUserType {
        Individual => "INDIVIDUAL",
        Group => "GROUP",
        Resource => "RESOURCE",
        Room => "ROOM",
        Unknown => "UNKNOWN",
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Organizer {
    pub email: String,
    /// The CN parameter.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sent_by: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dir: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    /// Parameters this crate does not model, kept so a round trip is lossless.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub parameters: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Attendee {
    pub email: String,
    /// The CN parameter.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<Role>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub partstat: Option<ParticipationStatus>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cutype: Option<CalendarUserType>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rsvp: Option<bool>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub delegated_from: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub delegated_to: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub members: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sent_by: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dir: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    /// Parameters this crate does not model, kept so a round trip is lossless.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub parameters: BTreeMap<String, String>,
}

/// When a VALARM fires, RFC 5545 §3.8.6.3.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AlarmTrigger {
    /// A duration relative to the event's start or end, kept as written so the
    /// exact ISO 8601 spelling survives a round trip.
    Duration {
        value: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        related: Option<TriggerRelation>,
    },
    /// An absolute instant.
    DateTime { timestamp: DateTime<Utc> },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TriggerRelation {
    Start,
    End,
}

/// A VALARM component.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Alarm {
    /// The ACTION token, for example `DISPLAY`, `AUDIO` or `EMAIL`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trigger: Option<AlarmTrigger>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    /// The DURATION between repeats, kept as written.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repeat: Option<u32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attendees: Vec<Attendee>,
}
