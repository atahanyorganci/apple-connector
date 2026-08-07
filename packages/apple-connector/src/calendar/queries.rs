//! Compile-time checked calendar queries.

use sqlx::SqlitePool;

use super::{
    row::{
        AlarmRow, AttachmentRow, CalendarResolveRow, CalendarRow, EventRow, ExceptionDateRow,
        LocationRow, ParticipantRow, RecurrenceRow, StoreRow,
    },
    search::EventFilterBinds,
};

pub async fn fetch_stores_ordered(pool: &SqlitePool) -> Result<Vec<StoreRow>, sqlx::Error> {
    sqlx::query_as!(
        StoreRow,
        r#"
        SELECT
          s.ROWID AS "row_id!",
          s.name,
          s.type AS store_type,
          s.disabled,
          s.external_id
        FROM Store s
        ORDER BY s.display_order, s.ROWID
        "#,
    )
    .fetch_all(pool)
    .await
}

pub async fn fetch_calendars_page(
    pool: &SqlitePool,
    cursor_row_id: Option<i64>,
    limit: i64,
) -> Result<Vec<CalendarRow>, sqlx::Error> {
    sqlx::query_as!(
        CalendarRow,
        r#"
        SELECT
          c.ROWID AS "row_id!",
          lower(c.UUID) AS "id!: String",
          c.title,
          c.color,
          c.store_id AS "store_id!",
          lower(s.external_id) AS "account_id!: String",
          c.notes,
          c.sharing_status
        FROM Calendar c
        JOIN Store s ON s.ROWID = c.store_id
        WHERE (?1 IS NULL OR c.ROWID < ?1)
        ORDER BY c.ROWID DESC
        LIMIT ?2
        "#,
        cursor_row_id,
        limit,
    )
    .fetch_all(pool)
    .await
}

pub async fn fetch_calendar_by_id(
    pool: &SqlitePool,
    calendar_id: &str,
) -> Result<Option<CalendarRow>, sqlx::Error> {
    sqlx::query_as!(
        CalendarRow,
        r#"
        SELECT
          c.ROWID AS "row_id!",
          lower(c.UUID) AS "id!: String",
          c.title,
          c.color,
          c.store_id AS "store_id!",
          lower(s.external_id) AS "account_id!: String",
          c.notes,
          c.sharing_status
        FROM Calendar c
        JOIN Store s ON s.ROWID = c.store_id
        WHERE lower(c.UUID) = lower(?1)
        "#,
        calendar_id,
    )
    .fetch_optional(pool)
    .await
}

pub async fn fetch_calendar_resolve_metadata(
    pool: &SqlitePool,
    calendar_id: &str,
) -> Result<Option<CalendarResolveRow>, sqlx::Error> {
    sqlx::query_as!(
        CalendarResolveRow,
        r#"
        SELECT
          lower(c.UUID) AS "api_id!: String",
          c.external_id,
          c.title,
          s.type AS store_type
        FROM Calendar c
        JOIN Store s ON s.ROWID = c.store_id
        WHERE lower(c.UUID) = lower(?1)
        "#,
        calendar_id,
    )
    .fetch_optional(pool)
    .await
}

pub async fn fetch_event_external_id(
    pool: &SqlitePool,
    event_id: &str,
) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar!(
        r#"
        SELECT external_id
        FROM CalendarItem
        WHERE lower(UUID) = lower(?1)
        "#,
        event_id,
    )
    .fetch_optional(pool)
    .await
    .map(|external_id| external_id.flatten())
}

pub async fn fetch_direct_events_page(
    pool: &SqlitePool,
    binds: &EventFilterBinds,
) -> Result<Vec<EventRow>, sqlx::Error> {
    sqlx::query_as!(
        EventRow,
        r#"
        SELECT
          ci.ROWID AS "row_id!",
          lower(ci.UUID) AS "id!: String",
          ci.calendar_id AS "calendar_row_id!",
          lower(c.UUID) AS "calendar_id!: String",
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
          lower(series.UUID) AS "series_id: String",
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
          ci.structured_data AS "structured_data: Vec<u8>",
          ci.app_link AS "app_link: Vec<u8>",
          CAST(NULL AS REAL) AS "occurrence_start: f64",
          CAST(NULL AS REAL) AS "occurrence_end: f64"
        FROM CalendarItem ci
        JOIN Calendar c ON c.ROWID = ci.calendar_id
        LEFT JOIN CalendarItem series ON series.ROWID = ci.orig_item_id
        JOIN Store s ON s.ROWID = c.store_id
        WHERE 1=1
          AND (?1 = 1 OR ci.hidden = 0)
          AND (?2 = 1 OR COALESCE(ci.status, 0) != 2)
          AND (?3 IS NULL OR lower(c.UUID) = lower(?3))
          AND (?4 IS NULL OR lower(s.external_id) = lower(?4))
          AND (?5 IS NULL OR ci.summary LIKE ?5)
          AND (?6 IS NULL OR ci.end_date >= ?6)
          AND (?7 IS NULL OR ci.start_date <= ?7)
          AND (
            ?8 IS NULL
            OR ci.last_modified < ?8
            OR (ci.last_modified = ?8 AND ci.ROWID < ?9)
          )
        ORDER BY ci.last_modified DESC, ci.ROWID DESC
        LIMIT ?10
        "#,
        binds.include_hidden,
        binds.include_cancelled,
        binds.calendar_id,
        binds.account_id,
        binds.q_pattern,
        binds.start_after,
        binds.start_before,
        binds.cursor_at,
        binds.cursor_row_id,
        binds.limit,
    )
    .fetch_all(pool)
    .await
}

pub async fn fetch_occurrence_events_page(
    pool: &SqlitePool,
    binds: &EventFilterBinds,
) -> Result<Vec<EventRow>, sqlx::Error> {
    sqlx::query_as!(
        EventRow,
        r#"
        SELECT
          ci.ROWID AS "row_id!",
          lower(ci.UUID) AS "id!: String",
          ci.calendar_id AS "calendar_row_id!",
          lower(c.UUID) AS "calendar_id!: String",
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
          lower(series.UUID) AS "series_id: String",
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
          ci.structured_data AS "structured_data: Vec<u8>",
          ci.app_link AS "app_link: Vec<u8>",
          oc.occurrence_start_date AS occurrence_start,
          oc.occurrence_end_date AS occurrence_end
        FROM OccurrenceCache oc
        JOIN CalendarItem ci ON ci.ROWID = oc.event_id
        JOIN Calendar c ON c.ROWID = ci.calendar_id
        LEFT JOIN CalendarItem series ON series.ROWID = ci.orig_item_id
        JOIN Store s ON s.ROWID = c.store_id
        WHERE 1=1
          AND (?1 = 1 OR ci.hidden = 0)
          AND (?2 = 1 OR COALESCE(ci.status, 0) != 2)
          AND (?3 IS NULL OR lower(c.UUID) = lower(?3))
          AND (?4 IS NULL OR lower(s.external_id) = lower(?4))
          AND (?5 IS NULL OR ci.summary LIKE ?5)
          AND (?6 IS NULL OR oc.occurrence_end_date >= ?6)
          AND (?7 IS NULL OR oc.occurrence_start_date <= ?7)
          AND (
            ?8 IS NULL
            OR oc.occurrence_start_date < ?8
            OR (oc.occurrence_start_date = ?8 AND ci.ROWID < ?9)
          )
        ORDER BY oc.occurrence_start_date DESC, ci.ROWID DESC
        LIMIT ?10
        "#,
        binds.include_hidden,
        binds.include_cancelled,
        binds.calendar_id,
        binds.account_id,
        binds.q_pattern,
        binds.start_after,
        binds.start_before,
        binds.cursor_at,
        binds.cursor_row_id,
        binds.limit,
    )
    .fetch_all(pool)
    .await
}

pub async fn fetch_event_by_id(
    pool: &SqlitePool,
    event_id: &str,
) -> Result<Option<EventRow>, sqlx::Error> {
    sqlx::query_as!(
        EventRow,
        r#"
        SELECT
          ci.ROWID AS "row_id!",
          lower(ci.UUID) AS "id!: String",
          ci.calendar_id AS "calendar_row_id!",
          lower(c.UUID) AS "calendar_id!: String",
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
          lower(series.UUID) AS "series_id: String",
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
          ci.structured_data AS "structured_data: Vec<u8>",
          ci.app_link AS "app_link: Vec<u8>",
          CAST(NULL AS REAL) AS "occurrence_start: f64",
          CAST(NULL AS REAL) AS "occurrence_end: f64"
        FROM CalendarItem ci
        JOIN Calendar c ON c.ROWID = ci.calendar_id
        LEFT JOIN CalendarItem series ON series.ROWID = ci.orig_item_id
        WHERE lower(ci.UUID) = lower(?1)
        "#,
        event_id,
    )
    .fetch_optional(pool)
    .await
}

pub async fn fetch_location_by_id(
    pool: &SqlitePool,
    location_id: i64,
) -> Result<Option<LocationRow>, sqlx::Error> {
    sqlx::query_as!(
        LocationRow,
        r#"
        SELECT
          ROWID AS "row_id!",
          title,
          address,
          latitude,
          longitude
        FROM Location
        WHERE ROWID = ?1
        "#,
        location_id,
    )
    .fetch_optional(pool)
    .await
}

pub async fn fetch_participants_by_owner_id(
    pool: &SqlitePool,
    owner_id: i64,
) -> Result<Vec<ParticipantRow>, sqlx::Error> {
    sqlx::query_as!(
        ParticipantRow,
        r#"
        SELECT
          ROWID AS "row_id!",
          lower(UUID) AS "id!: String",
          email,
          phone_number,
          status,
          role,
          is_self,
          comment
        FROM Participant
        WHERE owner_id = ?1
        "#,
        owner_id,
    )
    .fetch_all(pool)
    .await
}

pub async fn fetch_recurrence_by_owner_id(
    pool: &SqlitePool,
    owner_id: i64,
) -> Result<Option<RecurrenceRow>, sqlx::Error> {
    sqlx::query_as!(
        RecurrenceRow,
        r#"
        SELECT
          ROWID AS "row_id!",
          frequency,
          interval,
          count,
          end_date,
          specifier
        FROM Recurrence
        WHERE owner_id = ?1
        "#,
        owner_id,
    )
    .fetch_optional(pool)
    .await
}

pub async fn fetch_exception_dates_by_owner_id(
    pool: &SqlitePool,
    owner_id: i64,
) -> Result<Vec<ExceptionDateRow>, sqlx::Error> {
    sqlx::query_as!(
        ExceptionDateRow,
        r#"
        SELECT date AS "date!"
        FROM ExceptionDate
        WHERE owner_id = ?1
        "#,
        owner_id,
    )
    .fetch_all(pool)
    .await
}

pub async fn fetch_alarms_by_event_id(
    pool: &SqlitePool,
    event_row_id: i64,
) -> Result<Vec<AlarmRow>, sqlx::Error> {
    sqlx::query_as!(
        AlarmRow,
        r#"
        SELECT
          ROWID AS "row_id!",
          lower(UUID) AS "id!: String",
          trigger_interval,
          trigger_date,
          type AS alarm_type,
          disabled
        FROM Alarm
        WHERE calendaritem_owner_id = ?1
        "#,
        event_row_id,
    )
    .fetch_all(pool)
    .await
}

pub async fn fetch_attachments_by_owner_id(
    pool: &SqlitePool,
    owner_id: i64,
) -> Result<Vec<AttachmentRow>, sqlx::Error> {
    sqlx::query_as!(
        AttachmentRow,
        r#"
        SELECT
          af.ROWID AS "row_id!",
          lower(af.UUID) AS "id!: String",
          af.filename,
          af.format,
          af.file_size,
          af.local_path
        FROM Attachment a
        JOIN AttachmentFile af ON af.ROWID = a.file_id
        WHERE a.owner_id = ?1
        "#,
        owner_id,
    )
    .fetch_all(pool)
    .await
}

pub async fn fetch_attachment_by_event_and_id(
    pool: &SqlitePool,
    event_id: &str,
    attachment_id: &str,
) -> Result<Option<AttachmentRow>, sqlx::Error> {
    sqlx::query_as!(
        AttachmentRow,
        r#"
        SELECT
          af.ROWID AS "row_id!",
          lower(af.UUID) AS "id!: String",
          af.filename,
          af.format,
          af.file_size,
          af.local_path
        FROM Attachment a
        JOIN AttachmentFile af ON af.ROWID = a.file_id
        JOIN CalendarItem ci ON ci.ROWID = a.owner_id
        WHERE lower(ci.UUID) = lower(?1)
          AND lower(af.UUID) = lower(?2)
        "#,
        event_id,
        attachment_id,
    )
    .fetch_optional(pool)
    .await
}

pub async fn count_calendar_item_table(pool: &SqlitePool) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar!(
        r#"
        SELECT COUNT(*) AS "count!: i64"
        FROM sqlite_master
        WHERE type = 'table' AND name = 'CalendarItem'
        "#,
    )
    .fetch_one(pool)
    .await
}

pub async fn count_stores(pool: &SqlitePool) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar!(r#"SELECT COUNT(*) AS "count!: i64" FROM Store"#,)
        .fetch_one(pool)
        .await
}

pub async fn count_calendars(pool: &SqlitePool) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar!(r#"SELECT COUNT(*) AS "count!: i64" FROM Calendar"#,)
        .fetch_one(pool)
        .await
}

pub async fn count_events(pool: &SqlitePool) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar!(r#"SELECT COUNT(*) AS "count!: i64" FROM CalendarItem"#,)
        .fetch_one(pool)
        .await
}

pub async fn count_recurring_events(pool: &SqlitePool) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar!(
        r#"
        SELECT COUNT(*) AS "count!: i64"
        FROM CalendarItem
        WHERE has_recurrences = 1
        "#,
    )
    .fetch_one(pool)
    .await
}

pub async fn count_occurrences(pool: &SqlitePool) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar!(r#"SELECT COUNT(*) AS "count!: i64" FROM OccurrenceCache"#,)
        .fetch_one(pool)
        .await
}

pub async fn count_attachments(pool: &SqlitePool) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar!(r#"SELECT COUNT(*) AS "count!: i64" FROM Attachment"#,)
        .fetch_one(pool)
        .await
}

pub async fn count_hidden_events(pool: &SqlitePool) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar!(
        r#"
        SELECT COUNT(*) AS "count!: i64"
        FROM CalendarItem
        WHERE hidden = 1
        "#,
    )
    .fetch_one(pool)
    .await
}
