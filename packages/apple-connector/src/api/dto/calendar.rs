use serde::Serialize;
use utoipa::ToSchema;

use super::pagination::PageMetaDto;
use crate::apple_types::{
    CalendarAccountId, CalendarAttachmentId, CalendarId, EventId, UnixTimestamp,
};

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum EventStatusDto {
    Confirmed,
    Tentative,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum InvitationStatusDto {
    Unknown,
    Accepted,
    Declined,
    Tentative,
    NeedsAction,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AvailabilityDto {
    Busy,
    Free,
    Tentative,
    Unavailable,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PrivacyLevelDto {
    Default,
    Public,
    Private,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum StoreTypeDto {
    Local,
    CalDav,
    Exchange,
    Subscription,
    Birthday,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum EventClassDto {
    Standard,
    Birthday,
    SpecialDay,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CalendarAccountDto {
    pub id: CalendarAccountId,
    pub row_id: i64,
    pub name: Option<String>,
    pub store_type: StoreTypeDto,
    pub disabled: bool,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CalendarAccountPageDto {
    pub items: Vec<CalendarAccountDto>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CalendarSummaryDto {
    pub id: CalendarId,
    pub row_id: i64,
    pub title: Option<String>,
    pub color: Option<String>,
    pub account_id: CalendarAccountId,
    pub account_row_id: i64,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CalendarDetailDto {
    #[serde(flatten)]
    pub summary: CalendarSummaryDto,
    pub notes: Option<String>,
    pub sharing_status: Option<i64>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CalendarPageDto {
    pub items: Vec<CalendarSummaryDto>,
    pub page: PageMetaDto,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct EventSummaryDto {
    pub id: EventId,
    pub row_id: i64,
    pub calendar_id: CalendarId,
    pub calendar_row_id: i64,
    pub summary: Option<String>,
    pub start: Option<UnixTimestamp>,
    pub end: Option<UnixTimestamp>,
    pub all_day: bool,
    pub status: EventStatusDto,
    pub hidden: bool,
    pub is_recurring: bool,
    pub occurrence_start: Option<UnixTimestamp>,
    pub occurrence_end: Option<UnixTimestamp>,
    pub event_class: EventClassDto,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct EventLocationDto {
    pub title: Option<String>,
    pub address: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct EventParticipantDto {
    pub id: String,
    pub email: Option<String>,
    pub phone_number: Option<String>,
    pub name: Option<String>,
    pub is_self: bool,
    pub status: InvitationStatusDto,
    pub role: Option<i64>,
    pub comment: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RecurrenceRuleDto {
    pub frequency: i64,
    pub interval: i64,
    pub count: Option<i64>,
    pub end_date: Option<UnixTimestamp>,
    pub specifier: Option<String>,
    pub raw_specifier: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct EventAlarmDto {
    pub id: String,
    pub trigger_interval_seconds: Option<i64>,
    pub trigger_date: Option<UnixTimestamp>,
    pub alarm_type: i64,
    pub disabled: bool,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct EventAttachmentSummaryDto {
    pub id: CalendarAttachmentId,
    pub row_id: i64,
    pub filename: Option<String>,
    pub format: Option<String>,
    pub file_size: Option<i64>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct EventAttachmentDetailDto {
    #[serde(flatten)]
    pub summary: EventAttachmentSummaryDto,
    pub local_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct EventDetailDto {
    #[serde(flatten)]
    pub summary: EventSummaryDto,
    pub description: Option<String>,
    pub url: Option<String>,
    pub location: Option<EventLocationDto>,
    pub organizer: Option<EventParticipantDto>,
    pub attendees: Vec<EventParticipantDto>,
    pub recurrence: Option<RecurrenceRuleDto>,
    pub exception_dates: Vec<UnixTimestamp>,
    pub alarms: Vec<EventAlarmDto>,
    pub attachments: Vec<EventAttachmentSummaryDto>,
    pub conference_url: Option<String>,
    pub travel_time_seconds: Option<i64>,
    pub invitation_status: InvitationStatusDto,
    pub availability: AvailabilityDto,
    pub privacy_level: PrivacyLevelDto,
    pub series_id: Option<EventId>,
    pub series_row_id: Option<i64>,
    pub original_start: Option<UnixTimestamp>,
    pub last_modified: Option<UnixTimestamp>,
    pub creation_date: Option<UnixTimestamp>,
    pub has_structured_data: bool,
    pub has_app_link: bool,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct EventPageDto {
    pub items: Vec<EventSummaryDto>,
    pub page: PageMetaDto,
}
