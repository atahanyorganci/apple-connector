use chrono::{DateTime, Utc};

/// Seconds between the Unix epoch (1970-01-01) and the Core Data epoch (2001-01-01).
pub const CORE_DATA_EPOCH_UNIX_SECS: i64 = 978_307_200;

pub const UUID_SQL: &str = r#"lower(
  substr(hex(r.ZIDENTIFIER), 1, 8) || '-' ||
  substr(hex(r.ZIDENTIFIER), 9, 4) || '-' ||
  substr(hex(r.ZIDENTIFIER), 13, 4) || '-' ||
  substr(hex(r.ZIDENTIFIER), 17, 4) || '-' ||
  substr(hex(r.ZIDENTIFIER), 21, 12)
)"#;

pub const LIST_UUID_SQL: &str = r#"lower(
  substr(hex(l.ZIDENTIFIER), 1, 8) || '-' ||
  substr(hex(l.ZIDENTIFIER), 9, 4) || '-' ||
  substr(hex(l.ZIDENTIFIER), 13, 4) || '-' ||
  substr(hex(l.ZIDENTIFIER), 17, 4) || '-' ||
  substr(hex(l.ZIDENTIFIER), 21, 12)
)"#;

pub const SECTION_UUID_SQL: &str = r#"lower(
  substr(hex(s.ZIDENTIFIER), 1, 8) || '-' ||
  substr(hex(s.ZIDENTIFIER), 9, 4) || '-' ||
  substr(hex(s.ZIDENTIFIER), 13, 4) || '-' ||
  substr(hex(s.ZIDENTIFIER), 17, 4) || '-' ||
  substr(hex(s.ZIDENTIFIER), 21, 12)
)"#;

pub const ATTACHMENT_UUID_SQL: &str = r#"lower(
  substr(hex(sa.ZIDENTIFIER), 1, 8) || '-' ||
  substr(hex(sa.ZIDENTIFIER), 9, 4) || '-' ||
  substr(hex(sa.ZIDENTIFIER), 13, 4) || '-' ||
  substr(hex(sa.ZIDENTIFIER), 17, 4) || '-' ||
  substr(hex(sa.ZIDENTIFIER), 21, 12)
)"#;

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

#[allow(dead_code)]
pub fn core_data_secs_from_datetime(dt: DateTime<Utc>) -> f64 {
    (dt.timestamp() - CORE_DATA_EPOCH_UNIX_SECS) as f64
        + f64::from(dt.timestamp_subsec_nanos()) / 1_000_000_000.0
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

    #[test]
    fn core_data_secs_round_trip() {
        let dt = Utc.with_ymd_and_hms(2026, 1, 15, 12, 0, 0).unwrap();
        let core_data_secs = core_data_secs_from_datetime(dt);
        let parsed = parse_core_data_timestamp(Some(core_data_secs)).unwrap();
        assert_eq!(parsed, dt);
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
