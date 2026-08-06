use chrono::{DateTime, Utc};

pub use crate::apple_types::{core_data_secs_from_timestamp, parse_core_data_timestamp};

#[allow(dead_code)]
#[derive(Debug, sqlx::FromRow, Clone)]
pub struct ReminderRow {
    pub row_id: i64,
    pub id: String,
    pub title: Option<String>,
    pub notes: Option<String>,
    pub completed: bool,
    pub flagged: bool,
    pub priority: i64,
    pub all_day: bool,
    pub list_row_id: i64,
    pub list_id: String,
    pub list_name: Option<String>,
    pub parent_row_id: Option<i64>,
    pub parent_id: Option<String>,
    pub display_order: i64,
    pub due_date: Option<f64>,
    pub completion_date: Option<f64>,
    pub creation_date: Option<f64>,
    pub last_modified_date: Option<f64>,
    pub list_ent: i64,
    pub list_smart_type: Option<String>,
    pub list_sharing_status: Option<i64>,
    pub list_shared_owner_name: Option<String>,
    pub list_shared_owner_address: Option<String>,
    pub list_filter_data: Option<Vec<u8>>,
    pub list_membership_data: Option<Vec<u8>>,
}

#[derive(Debug, sqlx::FromRow, Clone)]
pub struct ListRow {
    pub row_id: i64,
    pub id: String,
    pub name: Option<String>,
    pub ent: i64,
    pub smart_list_type: Option<String>,
    pub sharing_status: Option<i64>,
    pub shared_owner_name: Option<String>,
    pub shared_owner_address: Option<String>,
    pub filter_data: Option<Vec<u8>>,
    pub membership_data: Option<Vec<u8>>,
    pub last_modified_date: Option<f64>,
}

#[derive(Debug, sqlx::FromRow, Clone)]
pub struct SectionRow {
    pub row_id: i64,
    pub id: String,
    pub display_name: Option<String>,
    pub canonical_name: Option<String>,
    pub list_row_id: i64,
}

#[allow(dead_code)]
#[derive(Debug, sqlx::FromRow, Clone)]
pub struct ObjectRow {
    pub row_id: i64,
    pub ent: i64,
    pub reminder_row_id: Option<i64>,
    pub object_type: Option<i64>,
    pub title: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub radius: Option<f64>,
    pub time_interval: Option<f64>,
    pub date_components_data: Option<Vec<u8>>,
    pub trigger_row_id: Option<i64>,
    pub frequency: Option<i64>,
    pub recurrence_interval: Option<i64>,
    pub occurrence_count: Option<i64>,
    pub hashtag_label_row_id: Option<i64>,
    pub tag_name: Option<String>,
}

#[derive(Debug, sqlx::FromRow, Clone)]
pub struct AttachmentRow {
    pub row_id: i64,
    pub id: String,
    pub filename: Option<String>,
    pub uti: Option<String>,
    pub sha512: Option<String>,
    pub kind_raw: Option<String>,
    pub reminder_row_id: i64,
    pub modified_at: Option<f64>,
}

#[allow(dead_code)]
pub fn core_data_secs_from_datetime(dt: DateTime<Utc>) -> f64 {
    core_data_secs_from_timestamp(dt)
}

#[allow(dead_code)]
pub fn format_uuid_blob(blob: &[u8]) -> Option<String> {
    if blob.len() != 16 {
        return None;
    }
    Some(format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        blob[0],
        blob[1],
        blob[2],
        blob[3],
        blob[4],
        blob[5],
        blob[6],
        blob[7],
        blob[8],
        blob[9],
        blob[10],
        blob[11],
        blob[12],
        blob[13],
        blob[14],
        blob[15],
    ))
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};

    use super::{core_data_secs_from_datetime, format_uuid_blob, parse_core_data_timestamp};
    use crate::apple_types::CORE_DATA_EPOCH_UNIX_SECS;

    #[test]
    fn unset_zero_is_none() {
        assert_eq!(parse_core_data_timestamp(Some(0.0)), None);
        assert_eq!(parse_core_data_timestamp(None), None);
    }

    #[test]
    fn parses_known_unix_instant() -> Result<(), Box<dyn std::error::Error>> {
        let unix_secs = Utc
            .with_ymd_and_hms(2026, 1, 15, 12, 0, 0)
            .single()
            .ok_or("invalid timestamp")?
            .timestamp();
        let core_data_secs = (unix_secs - CORE_DATA_EPOCH_UNIX_SECS) as f64;
        let parsed = parse_core_data_timestamp(Some(core_data_secs)).ok_or("missing timestamp")?;
        assert_eq!(parsed.to_rfc3339(), "2026-01-15T12:00:00+00:00");
        Ok(())
    }

    #[test]
    fn core_data_secs_round_trip() -> Result<(), Box<dyn std::error::Error>> {
        let dt = Utc
            .with_ymd_and_hms(2026, 1, 15, 12, 0, 0)
            .single()
            .ok_or("invalid timestamp")?;
        let core_data_secs = core_data_secs_from_datetime(dt);
        let parsed = parse_core_data_timestamp(Some(core_data_secs)).ok_or("missing timestamp")?;
        assert_eq!(parsed, dt);
        Ok(())
    }

    #[test]
    fn formats_uuid_blob() {
        let blob = [
            0xaa, 0xaa, 0xaa, 0xaa, 0xaa, 0xaa, 0xaa, 0xaa, 0xaa, 0xaa, 0xaa, 0xaa, 0xaa, 0xaa,
            0xaa, 0xaa,
        ];
        assert_eq!(
            format_uuid_blob(&blob),
            Some("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa".to_owned())
        );
    }
}
