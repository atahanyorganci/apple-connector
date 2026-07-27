use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::enums::{
    Availability, EventClass, EventStatus, InvitationStatus, PrivacyLevel, StoreType,
};

#[derive(Debug, Clone, PartialEq)]
pub struct CalendarAccount {
    pub row_id: i64,
    pub id: String,
    pub name: Option<String>,
    pub store_type: StoreType,
    pub disabled: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CalendarSummary {
    pub row_id: i64,
    pub id: String,
    pub title: Option<String>,
    pub color: Option<String>,
    pub account_row_id: i64,
    pub account_id: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CalendarDetail {
    pub summary: CalendarSummary,
    pub notes: Option<String>,
    pub sharing_status: Option<i64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EventSummary {
    pub row_id: i64,
    pub id: String,
    pub calendar_row_id: i64,
    pub calendar_id: String,
    pub summary: Option<String>,
    pub start: Option<DateTime<Utc>>,
    pub end: Option<DateTime<Utc>>,
    pub all_day: bool,
    pub status: EventStatus,
    pub hidden: bool,
    pub is_recurring: bool,
    pub occurrence_start: Option<DateTime<Utc>>,
    pub occurrence_end: Option<DateTime<Utc>>,
    pub event_class: EventClass,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EventDetail {
    pub summary: EventSummary,
    pub description: Option<String>,
    pub url: Option<String>,
    pub location: Option<EventLocation>,
    pub organizer: Option<EventParticipant>,
    pub attendees: Vec<EventParticipant>,
    pub recurrence: Option<RecurrenceRule>,
    pub exception_dates: Vec<DateTime<Utc>>,
    pub alarms: Vec<EventAlarm>,
    pub attachments: Vec<EventAttachment>,
    pub conference_url: Option<String>,
    pub travel_time_seconds: Option<i64>,
    pub invitation_status: InvitationStatus,
    pub availability: Availability,
    pub privacy_level: PrivacyLevel,
    pub series_id: Option<String>,
    pub series_row_id: Option<i64>,
    pub original_start: Option<DateTime<Utc>>,
    pub last_modified: Option<DateTime<Utc>>,
    pub creation_date: Option<DateTime<Utc>>,
    pub structured_data: Option<Vec<u8>>,
    pub app_link: Option<Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventLocation {
    pub title: Option<String>,
    pub address: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventParticipant {
    pub id: String,
    pub email: Option<String>,
    pub phone_number: Option<String>,
    pub name: Option<String>,
    pub is_self: bool,
    pub status: InvitationStatus,
    pub role: Option<i64>,
    pub comment: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecurrenceRule {
    pub frequency: i64,
    pub interval: i64,
    pub count: Option<i64>,
    pub end_date: Option<DateTime<Utc>>,
    pub specifier: Option<String>,
    pub raw_specifier: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventAlarm {
    pub id: String,
    pub trigger_interval_seconds: Option<i64>,
    pub trigger_date: Option<DateTime<Utc>>,
    pub alarm_type: i64,
    pub disabled: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EventAttachment {
    pub row_id: i64,
    pub id: String,
    pub filename: Option<String>,
    pub format: Option<String>,
    pub file_size: Option<i64>,
    pub local_path: Option<String>,
}

/// Interchange model for ICS/CalDAV serialization.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Event {
    pub uid: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<InterchangeStatus>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end: Option<i64>,
    #[serde(default)]
    pub all_day: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub organizer_email: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attendees: Vec<InterchangeAttendee>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recurrence_rule: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exception_dates: Vec<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InterchangeStatus {
    Confirmed,
    Tentative,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InterchangeAttendee {
    pub email: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub partstat: Option<String>,
}

impl From<&EventDetail> for Event {
    fn from(detail: &EventDetail) -> Self {
        Self {
            uid: detail.summary.id.clone(),
            summary: detail.summary.summary.clone(),
            description: detail.description.clone(),
            location: detail
                .location
                .as_ref()
                .and_then(|l| l.title.clone().or_else(|| l.address.clone())),
            url: detail.url.clone(),
            status: Some(match detail.summary.status {
                EventStatus::Confirmed => InterchangeStatus::Confirmed,
                EventStatus::Tentative => InterchangeStatus::Tentative,
                EventStatus::Cancelled => InterchangeStatus::Cancelled,
            }),
            start: detail
                .summary
                .occurrence_start
                .or(detail.summary.start)
                .map(|dt| dt.timestamp()),
            end: detail
                .summary
                .occurrence_end
                .or(detail.summary.end)
                .map(|dt| dt.timestamp()),
            all_day: detail.summary.all_day,
            organizer_email: detail.organizer.as_ref().and_then(|o| o.email.clone()),
            attendees: detail
                .attendees
                .iter()
                .filter_map(|a| {
                    a.email.as_ref().map(|email| InterchangeAttendee {
                        email: email.clone(),
                        name: a.name.clone(),
                        partstat: None,
                    })
                })
                .collect(),
            recurrence_rule: detail.recurrence.as_ref().and_then(|r| r.specifier.clone()),
            exception_dates: detail
                .exception_dates
                .iter()
                .map(|dt| dt.timestamp())
                .collect(),
        }
    }
}

impl Event {
    /// Convert the interchange model into the serde-icalendar wire format.
    pub fn to_ics_event(&self) -> serde_icalendar::CalendarEvent {
        use chrono::TimeZone;
        use serde_icalendar::{Attendee, CalendarEvent, EventDateTime, EventStatus, Organizer};

        fn ts_to_dt(ts: i64, all_day: bool) -> EventDateTime {
            EventDateTime {
                timestamp: Utc.timestamp_opt(ts, 0).single().unwrap_or_else(Utc::now),
                all_day,
                tzid: None,
            }
        }

        CalendarEvent {
            uid: Some(self.uid.clone()),
            summary: self.summary.clone(),
            description: self.description.clone(),
            location: self.location.clone(),
            url: self.url.clone(),
            status: self.status.map(|status| match status {
                InterchangeStatus::Confirmed => EventStatus::Confirmed,
                InterchangeStatus::Tentative => EventStatus::Tentative,
                InterchangeStatus::Cancelled => EventStatus::Cancelled,
            }),
            start: self.start.map(|ts| ts_to_dt(ts, self.all_day)),
            end: self.end.map(|ts| ts_to_dt(ts, self.all_day)),
            organizer: self.organizer_email.as_ref().map(|email| Organizer {
                email: email.clone(),
                name: None,
            }),
            attendees: self
                .attendees
                .iter()
                .map(|attendee| Attendee {
                    email: attendee.email.clone(),
                    name: attendee.name.clone(),
                    role: None,
                    partstat: attendee.partstat.clone(),
                    rsvp: None,
                })
                .collect(),
            alarms: Vec::new(),
            recurrence_rule: self.recurrence_rule.clone(),
            exception_dates: self
                .exception_dates
                .iter()
                .map(|ts| ts_to_dt(*ts, false))
                .collect(),
            sequence: None,
            extensions: None,
        }
    }
}
