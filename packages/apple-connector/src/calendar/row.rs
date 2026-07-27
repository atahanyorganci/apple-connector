use chrono::{DateTime, Utc};

pub const CORE_DATA_EPOCH_UNIX_SECS: i64 = 978_307_200;

#[derive(Debug, sqlx::FromRow, Clone)]
pub struct StoreRow {
    pub row_id: i64,
    pub name: Option<String>,
    pub store_type: Option<i64>,
    pub disabled: Option<i64>,
    pub external_id: Option<String>,
}

#[derive(Debug, sqlx::FromRow, Clone)]
pub struct CalendarRow {
    pub row_id: i64,
    pub id: String,
    pub title: Option<String>,
    pub color: Option<String>,
    pub store_id: i64,
    pub account_id: String,
    pub notes: Option<String>,
    pub sharing_status: Option<i64>,
}

#[derive(Debug, sqlx::FromRow, Clone)]
pub struct EventRow {
    pub row_id: i64,
    pub id: String,
    pub calendar_row_id: i64,
    pub calendar_id: String,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub start_date: Option<f64>,
    pub end_date: Option<f64>,
    pub all_day: Option<i64>,
    pub status: Option<i64>,
    pub hidden: Option<i64>,
    pub has_recurrences: Option<i64>,
    pub url: Option<String>,
    pub last_modified: Option<f64>,
    pub creation_date: Option<f64>,
    pub orig_item_id: Option<i64>,
    pub orig_date: Option<f64>,
    pub series_id: Option<String>,
    pub invitation_status: Option<i64>,
    pub availability: Option<i64>,
    pub privacy_level: Option<i64>,
    pub conference_url: Option<String>,
    pub travel_time: Option<i64>,
    pub location_id: Option<i64>,
    pub organizer_id: Option<i64>,
    pub entity_type: Option<i64>,
    pub birthday_id: Option<i64>,
    pub special_day: Option<String>,
    pub structured_data: Option<Vec<u8>>,
    pub app_link: Option<Vec<u8>>,
    pub occurrence_start: Option<f64>,
    pub occurrence_end: Option<f64>,
}

#[derive(Debug, sqlx::FromRow, Clone)]
pub struct LocationRow {
    #[allow(dead_code)]
    pub row_id: i64,
    pub title: Option<String>,
    pub address: Option<String>,
    pub latitude: Option<i64>,
    pub longitude: Option<i64>,
}

#[derive(Debug, sqlx::FromRow, Clone)]
pub struct ParticipantRow {
    pub row_id: i64,
    pub id: String,
    pub email: Option<String>,
    pub phone_number: Option<String>,
    pub status: Option<i64>,
    pub role: Option<i64>,
    pub is_self: Option<i64>,
    pub comment: Option<String>,
}

#[derive(Debug, sqlx::FromRow, Clone)]
pub struct RecurrenceRow {
    #[allow(dead_code)]
    pub row_id: i64,
    pub frequency: Option<i64>,
    pub interval: Option<i64>,
    pub count: Option<i64>,
    pub end_date: Option<f64>,
    pub specifier: Option<String>,
}

#[derive(Debug, sqlx::FromRow, Clone)]
pub struct AlarmRow {
    #[allow(dead_code)]
    pub row_id: i64,
    pub id: String,
    pub trigger_interval: Option<i64>,
    pub trigger_date: Option<f64>,
    pub alarm_type: Option<i64>,
    pub disabled: Option<i64>,
}

#[derive(Debug, sqlx::FromRow, Clone)]
pub struct AttachmentRow {
    pub row_id: i64,
    pub id: String,
    pub filename: Option<String>,
    pub format: Option<String>,
    pub file_size: Option<i64>,
    pub local_path: Option<String>,
}

#[derive(Debug, sqlx::FromRow, Clone)]
pub struct ExceptionDateRow {
    pub date: f64,
}

pub fn parse_core_data_timestamp(secs: Option<f64>) -> Option<DateTime<Utc>> {
    let secs = secs?;
    if secs <= 0.0 {
        return None;
    }
    let whole_secs = secs.trunc() as i64 + CORE_DATA_EPOCH_UNIX_SECS;
    let nanos = ((secs.fract()) * 1_000_000_000.0).round() as u32;
    DateTime::from_timestamp(whole_secs, nanos)
}

pub fn core_data_secs_from_timestamp(dt: DateTime<Utc>) -> f64 {
    (dt.timestamp() - CORE_DATA_EPOCH_UNIX_SECS) as f64
        + f64::from(dt.timestamp_subsec_nanos()) / 1_000_000_000.0
}

pub fn microdegrees_to_degrees(value: Option<i64>) -> Option<f64> {
    value.map(|v| f64::from(v as i32) / 1_000_000.0)
}
