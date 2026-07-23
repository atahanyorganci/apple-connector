use chrono::{DateTime, Utc};

/// Seconds between the Unix epoch (1970-01-01) and the Core Data epoch (2001-01-01).
pub const CORE_DATA_EPOCH_UNIX_SECS: i64 = 978_307_200;

#[derive(Debug, sqlx::FromRow, Clone)]
pub struct NoteRow {
    pub row_id: i64,
    pub id: String,
    pub title: Option<String>,
    pub snippet: Option<String>,
    pub created_at: Option<f64>,
    pub modified_at: Option<f64>,
    pub folder_row_id: Option<i64>,
    pub folder_id: Option<String>,
    pub folder_name: Option<String>,
    pub folder_type: Option<i64>,
    pub is_pinned: bool,
    pub has_checklist: bool,
    pub is_locked: bool,
    pub marked_for_deletion: bool,
}

#[derive(Debug, sqlx::FromRow, Clone)]
pub struct NoteDetailRow {
    pub row_id: i64,
    pub id: String,
    pub title: Option<String>,
    pub snippet: Option<String>,
    pub created_at: Option<f64>,
    pub modified_at: Option<f64>,
    pub folder_row_id: Option<i64>,
    pub folder_id: Option<String>,
    pub folder_name: Option<String>,
    pub folder_type: Option<i64>,
    pub is_pinned: bool,
    pub has_checklist: bool,
    pub is_locked: bool,
    pub marked_for_deletion: bool,
    pub note_data: Option<Vec<u8>>,
}

#[derive(Debug, sqlx::FromRow, Clone)]
pub struct FolderRow {
    pub row_id: i64,
    pub id: String,
    pub title: Option<String>,
    pub folder_type: Option<i64>,
    pub parent_row_id: Option<i64>,
    pub parent_id: Option<String>,
    pub account_row_id: Option<i64>,
    pub account_id: Option<String>,
    pub modified_at: Option<f64>,
}

#[derive(Debug, sqlx::FromRow, Clone)]
pub struct AttachmentRow {
    pub row_id: i64,
    pub id: String,
    pub filename: Option<String>,
    pub uti: Option<String>,
    pub note_row_id: i64,
    pub note_id: String,
    pub file_size: Option<i64>,
    pub modified_at: Option<f64>,
    pub account_id: Option<String>,
}

pub fn parse_core_data_timestamp(secs: Option<f64>) -> Option<DateTime<Utc>> {
    parse_core_data_timestamp_f64(secs)
}

pub fn parse_core_data_timestamp_f64(secs: Option<f64>) -> Option<DateTime<Utc>> {
    let secs = secs?;
    if secs <= 0.0 {
        return None;
    }
    let whole_secs = secs.trunc() as i64 + CORE_DATA_EPOCH_UNIX_SECS;
    let nanos = ((secs.fract()) * 1_000_000_000.0).round() as u32;
    DateTime::from_timestamp(whole_secs, nanos)
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};

    use super::parse_core_data_timestamp;

    #[test]
    fn unset_zero_is_none() {
        assert_eq!(parse_core_data_timestamp(Some(0.0)), None);
        assert_eq!(parse_core_data_timestamp(None), None);
    }

    #[test]
    fn parses_known_unix_instant() {
        let unix_secs = Utc
            .with_ymd_and_hms(2026, 1, 15, 12, 0, 0)
            .unwrap()
            .timestamp();
        let core_data_secs = (unix_secs - 978_307_200) as f64;
        let parsed = parse_core_data_timestamp(Some(core_data_secs)).unwrap();
        assert_eq!(parsed.to_rfc3339(), "2026-01-15T12:00:00+00:00");
    }
}
