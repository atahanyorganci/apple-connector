pub const STORE_SELECT: &str = r#"
SELECT
  s.ROWID AS row_id,
  s.name,
  s.type AS store_type,
  s.disabled,
  s.external_id
FROM Store s
"#;

pub const CALENDAR_SELECT: &str = r#"
SELECT
  c.ROWID AS row_id,
  lower(c.UUID) AS id,
  c.title,
  c.color,
  c.store_id,
  lower(s.external_id) AS account_id,
  c.notes,
  c.sharing_status
FROM Calendar c
JOIN Store s ON s.ROWID = c.store_id
"#;

pub const EVENT_SELECT: &str = r#"
SELECT
  ci.ROWID AS row_id,
  lower(ci.UUID) AS id,
  ci.calendar_id AS calendar_row_id,
  lower(c.UUID) AS calendar_id,
  ci.summary,
  ci.description,
  ci.start_date,
  ci.end_date,
  ci.all_day,
  ci.status,
  ci.hidden,
  ci.has_recurrences,
  ci.url,
  ci.last_modified,
  ci.creation_date,
  ci.orig_item_id,
  ci.orig_date,
  lower(series.UUID) AS series_id,
  ci.invitation_status,
  ci.availability,
  ci.privacy_level,
  ci.conference_url,
  ci.travel_time,
  ci.location_id,
  ci.organizer_id,
  ci.entity_type,
  ci.birthday_id,
  ci.special_day,
  ci.structured_data,
  ci.app_link,
  NULL AS occurrence_start,
  NULL AS occurrence_end
FROM CalendarItem ci
JOIN Calendar c ON c.ROWID = ci.calendar_id
LEFT JOIN CalendarItem series ON series.ROWID = ci.orig_item_id
"#;

pub const OCCURRENCE_EVENT_SELECT: &str = r#"
SELECT
  ci.ROWID AS row_id,
  lower(ci.UUID) AS id,
  ci.calendar_id AS calendar_row_id,
  lower(c.UUID) AS calendar_id,
  ci.summary,
  ci.description,
  ci.start_date,
  ci.end_date,
  ci.all_day,
  ci.status,
  ci.hidden,
  ci.has_recurrences,
  ci.url,
  ci.last_modified,
  ci.creation_date,
  ci.orig_item_id,
  ci.orig_date,
  lower(series.UUID) AS series_id,
  ci.invitation_status,
  ci.availability,
  ci.privacy_level,
  ci.conference_url,
  ci.travel_time,
  ci.location_id,
  ci.organizer_id,
  ci.entity_type,
  ci.birthday_id,
  ci.special_day,
  ci.structured_data,
  ci.app_link,
  oc.occurrence_start_date AS occurrence_start,
  oc.occurrence_end_date AS occurrence_end
FROM OccurrenceCache oc
JOIN CalendarItem ci ON ci.ROWID = oc.event_id
JOIN Calendar c ON c.ROWID = ci.calendar_id
LEFT JOIN CalendarItem series ON series.ROWID = ci.orig_item_id
"#;

pub const LOCATION_SELECT: &str = r#"
SELECT ROWID AS row_id, title, address, latitude, longitude
FROM Location
"#;

pub const PARTICIPANT_SELECT: &str = r#"
SELECT
  ROWID AS row_id,
  lower(UUID) AS id,
  email,
  phone_number,
  status,
  role,
  is_self,
  comment
FROM Participant
"#;

pub const RECURRENCE_SELECT: &str = r#"
SELECT ROWID AS row_id, frequency, interval, count, end_date, specifier
FROM Recurrence
"#;

pub const ALARM_SELECT: &str = r#"
SELECT
  ROWID AS row_id,
  lower(UUID) AS id,
  trigger_interval,
  trigger_date,
  type AS alarm_type,
  disabled
FROM Alarm
"#;

pub const ATTACHMENT_SELECT: &str = r#"
SELECT
  af.ROWID AS row_id,
  lower(af.UUID) AS id,
  af.filename,
  af.format,
  af.file_size,
  af.local_path
FROM Attachment a
JOIN AttachmentFile af ON af.ROWID = a.file_id
"#;

pub const EXCEPTION_DATE_SELECT: &str = r#"
SELECT date FROM ExceptionDate
"#;
