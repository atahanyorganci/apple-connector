//! Compile-time checked reminder queries.

#[cfg(test)]
fn record_test_query() {
    crate::db::query_budget::bump();
}

#[cfg(not(test))]
fn record_test_query() {}

use sqlx::{SqliteExecutor, SqlitePool};

use super::{
    row::{AttachmentRow, ListRow, ObjectRow, ReminderRow, SectionRow},
    search::ReminderFilterBinds,
};

pub async fn fetch_lists_page<'e, E>(
    executor: E,
    cursor_row_id: Option<i64>,
    limit: i64,
) -> Result<Vec<ListRow>, sqlx::Error>
where
    E: SqliteExecutor<'e>,
{
    sqlx::query_as!(
        ListRow,
        r#"
        SELECT
          l.Z_PK AS "row_id!",
          CAST(
            lower(
              substr(hex(l.ZIDENTIFIER), 1, 8) || '-' ||
              substr(hex(l.ZIDENTIFIER), 9, 4) || '-' ||
              substr(hex(l.ZIDENTIFIER), 13, 4) || '-' ||
              substr(hex(l.ZIDENTIFIER), 17, 4) || '-' ||
              substr(hex(l.ZIDENTIFIER), 21, 12)
            ) AS TEXT
          ) AS "id!: String",
          l.ZNAME AS name,
          l.Z_ENT AS "ent!",
          l.ZSMARTLISTTYPE AS smart_list_type,
          l.ZSHARINGSTATUS AS sharing_status,
          l.ZSHAREDOWNERNAME AS shared_owner_name,
          l.ZSHAREDOWNERADDRESS AS shared_owner_address,
          l.ZFILTERDATA AS "filter_data: Vec<u8>",
          l.ZMEMBERSHIPSOFREMINDERSINSECTIONSASDATA AS "membership_data?: Vec<u8>",
          CAST(NULL AS REAL) AS "last_modified_date: f64"
        FROM ZREMCDBASELIST l
        WHERE l.ZMARKEDFORDELETION = 0
          AND (? IS NULL OR l.Z_PK < ?)
        ORDER BY l.Z_PK DESC
        LIMIT ?
        "#,
        cursor_row_id,
        cursor_row_id,
        limit,
    )
    .fetch_all(executor)
    .await
}

pub async fn fetch_list_by_row_id<'e, E>(
    executor: E,
    list_row_id: i64,
) -> Result<Option<ListRow>, sqlx::Error>
where
    E: SqliteExecutor<'e>,
{
    sqlx::query_as!(
        ListRow,
        r#"
        SELECT
          l.Z_PK AS "row_id!",
          CAST(
            lower(
              substr(hex(l.ZIDENTIFIER), 1, 8) || '-' ||
              substr(hex(l.ZIDENTIFIER), 9, 4) || '-' ||
              substr(hex(l.ZIDENTIFIER), 13, 4) || '-' ||
              substr(hex(l.ZIDENTIFIER), 17, 4) || '-' ||
              substr(hex(l.ZIDENTIFIER), 21, 12)
            ) AS TEXT
          ) AS "id!: String",
          l.ZNAME AS name,
          l.Z_ENT AS "ent!",
          l.ZSMARTLISTTYPE AS smart_list_type,
          l.ZSHARINGSTATUS AS sharing_status,
          l.ZSHAREDOWNERNAME AS shared_owner_name,
          l.ZSHAREDOWNERADDRESS AS shared_owner_address,
          l.ZFILTERDATA AS "filter_data: Vec<u8>",
          l.ZMEMBERSHIPSOFREMINDERSINSECTIONSASDATA AS "membership_data?: Vec<u8>",
          CAST(NULL AS REAL) AS "last_modified_date: f64"
        FROM ZREMCDBASELIST l
        WHERE l.ZMARKEDFORDELETION = 0
          AND l.Z_PK = ?
        "#,
        list_row_id,
    )
    .fetch_optional(executor)
    .await
}

pub async fn fetch_list_by_uuid<'e, E>(
    executor: E,
    id: &str,
) -> Result<Option<ListRow>, sqlx::Error>
where
    E: SqliteExecutor<'e>,
{
    sqlx::query_as!(
        ListRow,
        r#"
        SELECT
          l.Z_PK AS "row_id!",
          CAST(
            lower(
              substr(hex(l.ZIDENTIFIER), 1, 8) || '-' ||
              substr(hex(l.ZIDENTIFIER), 9, 4) || '-' ||
              substr(hex(l.ZIDENTIFIER), 13, 4) || '-' ||
              substr(hex(l.ZIDENTIFIER), 17, 4) || '-' ||
              substr(hex(l.ZIDENTIFIER), 21, 12)
            ) AS TEXT
          ) AS "id!: String",
          l.ZNAME AS name,
          l.Z_ENT AS "ent!",
          l.ZSMARTLISTTYPE AS smart_list_type,
          l.ZSHARINGSTATUS AS sharing_status,
          l.ZSHAREDOWNERNAME AS shared_owner_name,
          l.ZSHAREDOWNERADDRESS AS shared_owner_address,
          l.ZFILTERDATA AS "filter_data: Vec<u8>",
          l.ZMEMBERSHIPSOFREMINDERSINSECTIONSASDATA AS "membership_data?: Vec<u8>",
          CAST(NULL AS REAL) AS "last_modified_date: f64"
        FROM ZREMCDBASELIST l
        WHERE l.ZMARKEDFORDELETION = 0
          AND lower(
            substr(hex(l.ZIDENTIFIER), 1, 8) || '-' ||
            substr(hex(l.ZIDENTIFIER), 9, 4) || '-' ||
            substr(hex(l.ZIDENTIFIER), 13, 4) || '-' ||
            substr(hex(l.ZIDENTIFIER), 17, 4) || '-' ||
            substr(hex(l.ZIDENTIFIER), 21, 12)
          ) = ?
        "#,
        id,
    )
    .fetch_optional(executor)
    .await
}

pub async fn fetch_filtered_reminders<'e, E>(
    executor: E,
    binds: &ReminderFilterBinds,
) -> Result<Vec<ReminderRow>, sqlx::Error>
where
    E: SqliteExecutor<'e>,
{
    sqlx::query_as!(
        ReminderRow,
        r#"
        SELECT
          r.Z_PK AS "row_id!",
          CAST(
            lower(
              substr(hex(r.ZIDENTIFIER), 1, 8) || '-' ||
              substr(hex(r.ZIDENTIFIER), 9, 4) || '-' ||
              substr(hex(r.ZIDENTIFIER), 13, 4) || '-' ||
              substr(hex(r.ZIDENTIFIER), 17, 4) || '-' ||
              substr(hex(r.ZIDENTIFIER), 21, 12)
            ) AS TEXT
          ) AS "id!: String",
          r.ZTITLE AS title,
          r.ZNOTES AS notes,
          (r.ZCOMPLETED != 0) AS "completed!: bool",
          (r.ZFLAGGED != 0) AS "flagged!: bool",
          r.ZPRIORITY AS "priority!",
          (r.ZALLDAY != 0) AS "all_day!: bool",
          r.ZLIST AS "list_row_id!",
          CAST(
            lower(
              substr(hex(l.ZIDENTIFIER), 1, 8) || '-' ||
              substr(hex(l.ZIDENTIFIER), 9, 4) || '-' ||
              substr(hex(l.ZIDENTIFIER), 13, 4) || '-' ||
              substr(hex(l.ZIDENTIFIER), 17, 4) || '-' ||
              substr(hex(l.ZIDENTIFIER), 21, 12)
            ) AS TEXT
          ) AS "list_id!: String",
          l.ZNAME AS list_name,
          r.ZPARENTREMINDER AS parent_row_id,
          CAST(
            CASE
              WHEN p.ZIDENTIFIER IS NULL THEN NULL
              ELSE lower(
                substr(hex(p.ZIDENTIFIER), 1, 8) || '-' ||
                substr(hex(p.ZIDENTIFIER), 9, 4) || '-' ||
                substr(hex(p.ZIDENTIFIER), 13, 4) || '-' ||
                substr(hex(p.ZIDENTIFIER), 17, 4) || '-' ||
                substr(hex(p.ZIDENTIFIER), 21, 12)
              )
            END AS TEXT
          ) AS "parent_id: String",
          r.ZICSDISPLAYORDER AS "display_order!",
          CAST(r.ZDUEDATE AS REAL) AS due_date,
          CAST(r.ZCOMPLETIONDATE AS REAL) AS completion_date,
          CAST(r.ZCREATIONDATE AS REAL) AS creation_date,
          CAST(r.ZLASTMODIFIEDDATE AS REAL) AS last_modified_date,
          l.Z_ENT AS "list_ent!",
          l.ZSMARTLISTTYPE AS list_smart_type,
          l.ZSHARINGSTATUS AS list_sharing_status,
          l.ZSHAREDOWNERNAME AS list_shared_owner_name,
          l.ZSHAREDOWNERADDRESS AS list_shared_owner_address,
          l.ZFILTERDATA AS "list_filter_data: Vec<u8>",
          l.ZMEMBERSHIPSOFREMINDERSINSECTIONSASDATA AS "list_membership_data: Vec<u8>"
        FROM ZREMCDREMINDER r
        JOIN ZREMCDBASELIST l ON r.ZLIST = l.Z_PK
        LEFT JOIN ZREMCDREMINDER p ON r.ZPARENTREMINDER = p.Z_PK
        WHERE r.ZMARKEDFORDELETION = 0
          AND l.ZMARKEDFORDELETION = 0
          AND (? IS NULL OR r.ZCOMPLETED = ?)
          AND (? IS NULL OR r.ZFLAGGED = ?)
          AND (? IS NULL OR r.ZLIST = ?)
          AND (
            ? IS NULL
            OR lower(
              substr(hex(l.ZIDENTIFIER), 1, 8) || '-' ||
              substr(hex(l.ZIDENTIFIER), 9, 4) || '-' ||
              substr(hex(l.ZIDENTIFIER), 13, 4) || '-' ||
              substr(hex(l.ZIDENTIFIER), 17, 4) || '-' ||
              substr(hex(l.ZIDENTIFIER), 21, 12)
            ) = ?
          )
          AND (
            ? IS NULL
            OR (? = 1 AND r.ZDUEDATE IS NOT NULL)
            OR (? = 0 AND r.ZDUEDATE IS NULL)
          )
          AND (? IS NULL OR r.ZDUEDATE <= ?)
          AND (? IS NULL OR r.ZDUEDATE >= ?)
          AND (? IS NULL OR r.ZPRIORITY >= ?)
          AND (
            ? IS NULL
            OR (? = 1 AND r.ZNOTES IS NOT NULL AND trim(r.ZNOTES) != '')
            OR (? = 0 AND (r.ZNOTES IS NULL OR trim(r.ZNOTES) = ''))
          )
          AND (? IS NULL OR r.ZPARENTREMINDER IS NULL)
          AND (
            ? IS NULL
            OR (
              lower(r.ZTITLE) LIKE '%' || lower(?) || '%'
              OR lower(coalesce(r.ZNOTES, '')) LIKE '%' || lower(?) || '%'
            )
          )
          AND (
            ? IS NULL
            OR r.ZLASTMODIFIEDDATE < ?
            OR (r.ZLASTMODIFIEDDATE = ? AND r.Z_PK < ?)
          )
        ORDER BY r.ZLASTMODIFIEDDATE DESC, r.Z_PK DESC
        LIMIT ?
        "#,
        binds.completed,
        binds.completed,
        binds.flagged,
        binds.flagged,
        binds.list_row_id,
        binds.list_row_id,
        binds.list_uuid,
        binds.list_uuid,
        binds.has_due_date,
        binds.has_due_date,
        binds.has_due_date,
        binds.due_before,
        binds.due_before,
        binds.due_after,
        binds.due_after,
        binds.priority_min,
        binds.priority_min,
        binds.has_notes,
        binds.has_notes,
        binds.has_notes,
        binds.top_level_only,
        binds.q,
        binds.q,
        binds.q,
        binds.cursor_modified_at,
        binds.cursor_modified_at,
        binds.cursor_modified_at,
        binds.cursor_row_id,
        binds.limit,
    )
    .fetch_all(executor)
    .await
}

pub async fn fetch_reminder_by_uuid<'e, E>(
    executor: E,
    id: &str,
) -> Result<Option<ReminderRow>, sqlx::Error>
where
    E: SqliteExecutor<'e>,
{
    sqlx::query_as!(
        ReminderRow,
        r#"
        SELECT
          r.Z_PK AS "row_id!",
          CAST(
            lower(
              substr(hex(r.ZIDENTIFIER), 1, 8) || '-' ||
              substr(hex(r.ZIDENTIFIER), 9, 4) || '-' ||
              substr(hex(r.ZIDENTIFIER), 13, 4) || '-' ||
              substr(hex(r.ZIDENTIFIER), 17, 4) || '-' ||
              substr(hex(r.ZIDENTIFIER), 21, 12)
            ) AS TEXT
          ) AS "id!: String",
          r.ZTITLE AS title,
          r.ZNOTES AS notes,
          (r.ZCOMPLETED != 0) AS "completed!: bool",
          (r.ZFLAGGED != 0) AS "flagged!: bool",
          r.ZPRIORITY AS "priority!",
          (r.ZALLDAY != 0) AS "all_day!: bool",
          r.ZLIST AS "list_row_id!",
          CAST(
            lower(
              substr(hex(l.ZIDENTIFIER), 1, 8) || '-' ||
              substr(hex(l.ZIDENTIFIER), 9, 4) || '-' ||
              substr(hex(l.ZIDENTIFIER), 13, 4) || '-' ||
              substr(hex(l.ZIDENTIFIER), 17, 4) || '-' ||
              substr(hex(l.ZIDENTIFIER), 21, 12)
            ) AS TEXT
          ) AS "list_id!: String",
          l.ZNAME AS list_name,
          r.ZPARENTREMINDER AS parent_row_id,
          CAST(
            CASE
              WHEN p.ZIDENTIFIER IS NULL THEN NULL
              ELSE lower(
                substr(hex(p.ZIDENTIFIER), 1, 8) || '-' ||
                substr(hex(p.ZIDENTIFIER), 9, 4) || '-' ||
                substr(hex(p.ZIDENTIFIER), 13, 4) || '-' ||
                substr(hex(p.ZIDENTIFIER), 17, 4) || '-' ||
                substr(hex(p.ZIDENTIFIER), 21, 12)
              )
            END AS TEXT
          ) AS "parent_id: String",
          r.ZICSDISPLAYORDER AS "display_order!",
          CAST(r.ZDUEDATE AS REAL) AS due_date,
          CAST(r.ZCOMPLETIONDATE AS REAL) AS completion_date,
          CAST(r.ZCREATIONDATE AS REAL) AS creation_date,
          CAST(r.ZLASTMODIFIEDDATE AS REAL) AS last_modified_date,
          l.Z_ENT AS "list_ent!",
          l.ZSMARTLISTTYPE AS list_smart_type,
          l.ZSHARINGSTATUS AS list_sharing_status,
          l.ZSHAREDOWNERNAME AS list_shared_owner_name,
          l.ZSHAREDOWNERADDRESS AS list_shared_owner_address,
          l.ZFILTERDATA AS "list_filter_data: Vec<u8>",
          l.ZMEMBERSHIPSOFREMINDERSINSECTIONSASDATA AS "list_membership_data: Vec<u8>"
        FROM ZREMCDREMINDER r
        JOIN ZREMCDBASELIST l ON r.ZLIST = l.Z_PK
        LEFT JOIN ZREMCDREMINDER p ON r.ZPARENTREMINDER = p.Z_PK
        WHERE r.ZMARKEDFORDELETION = 0
          AND l.ZMARKEDFORDELETION = 0
          AND lower(
            substr(hex(r.ZIDENTIFIER), 1, 8) || '-' ||
            substr(hex(r.ZIDENTIFIER), 9, 4) || '-' ||
            substr(hex(r.ZIDENTIFIER), 13, 4) || '-' ||
            substr(hex(r.ZIDENTIFIER), 17, 4) || '-' ||
            substr(hex(r.ZIDENTIFIER), 21, 12)
          ) = ?
        "#,
        id,
    )
    .fetch_optional(executor)
    .await
}

pub async fn fetch_subtasks_for_parent<'e, E>(
    executor: E,
    parent_row_id: i64,
) -> Result<Vec<ReminderRow>, sqlx::Error>
where
    E: SqliteExecutor<'e>,
{
    sqlx::query_as!(
        ReminderRow,
        r#"
        SELECT
          r.Z_PK AS "row_id!",
          CAST(
            lower(
              substr(hex(r.ZIDENTIFIER), 1, 8) || '-' ||
              substr(hex(r.ZIDENTIFIER), 9, 4) || '-' ||
              substr(hex(r.ZIDENTIFIER), 13, 4) || '-' ||
              substr(hex(r.ZIDENTIFIER), 17, 4) || '-' ||
              substr(hex(r.ZIDENTIFIER), 21, 12)
            ) AS TEXT
          ) AS "id!: String",
          r.ZTITLE AS title,
          r.ZNOTES AS notes,
          (r.ZCOMPLETED != 0) AS "completed!: bool",
          (r.ZFLAGGED != 0) AS "flagged!: bool",
          r.ZPRIORITY AS "priority!",
          (r.ZALLDAY != 0) AS "all_day!: bool",
          r.ZLIST AS "list_row_id!",
          CAST(
            lower(
              substr(hex(l.ZIDENTIFIER), 1, 8) || '-' ||
              substr(hex(l.ZIDENTIFIER), 9, 4) || '-' ||
              substr(hex(l.ZIDENTIFIER), 13, 4) || '-' ||
              substr(hex(l.ZIDENTIFIER), 17, 4) || '-' ||
              substr(hex(l.ZIDENTIFIER), 21, 12)
            ) AS TEXT
          ) AS "list_id!: String",
          l.ZNAME AS list_name,
          r.ZPARENTREMINDER AS parent_row_id,
          CAST(
            CASE
              WHEN p.ZIDENTIFIER IS NULL THEN NULL
              ELSE lower(
                substr(hex(p.ZIDENTIFIER), 1, 8) || '-' ||
                substr(hex(p.ZIDENTIFIER), 9, 4) || '-' ||
                substr(hex(p.ZIDENTIFIER), 13, 4) || '-' ||
                substr(hex(p.ZIDENTIFIER), 17, 4) || '-' ||
                substr(hex(p.ZIDENTIFIER), 21, 12)
              )
            END AS TEXT
          ) AS "parent_id: String",
          r.ZICSDISPLAYORDER AS "display_order!",
          CAST(r.ZDUEDATE AS REAL) AS due_date,
          CAST(r.ZCOMPLETIONDATE AS REAL) AS completion_date,
          CAST(r.ZCREATIONDATE AS REAL) AS creation_date,
          CAST(r.ZLASTMODIFIEDDATE AS REAL) AS last_modified_date,
          l.Z_ENT AS "list_ent!",
          l.ZSMARTLISTTYPE AS list_smart_type,
          l.ZSHARINGSTATUS AS list_sharing_status,
          l.ZSHAREDOWNERNAME AS list_shared_owner_name,
          l.ZSHAREDOWNERADDRESS AS list_shared_owner_address,
          l.ZFILTERDATA AS "list_filter_data: Vec<u8>",
          l.ZMEMBERSHIPSOFREMINDERSINSECTIONSASDATA AS "list_membership_data: Vec<u8>"
        FROM ZREMCDREMINDER r
        JOIN ZREMCDBASELIST l ON r.ZLIST = l.Z_PK
        LEFT JOIN ZREMCDREMINDER p ON r.ZPARENTREMINDER = p.Z_PK
        WHERE r.ZMARKEDFORDELETION = 0
          AND l.ZMARKEDFORDELETION = 0
          AND r.ZPARENTREMINDER = ?
        ORDER BY r.ZICSDISPLAYORDER ASC, r.Z_PK ASC
        "#,
        parent_row_id,
    )
    .fetch_all(executor)
    .await
}

pub async fn fetch_sections_for_list<'e, E>(
    executor: E,
    list_row_id: i64,
) -> Result<Vec<SectionRow>, sqlx::Error>
where
    E: SqliteExecutor<'e>,
{
    sqlx::query_as!(
        SectionRow,
        r#"
        SELECT
          s.Z_PK AS "row_id!",
          CAST(
            lower(
              substr(hex(s.ZIDENTIFIER), 1, 8) || '-' ||
              substr(hex(s.ZIDENTIFIER), 9, 4) || '-' ||
              substr(hex(s.ZIDENTIFIER), 13, 4) || '-' ||
              substr(hex(s.ZIDENTIFIER), 17, 4) || '-' ||
              substr(hex(s.ZIDENTIFIER), 21, 12)
            ) AS TEXT
          ) AS "id!: String",
          s.ZDISPLAYNAME AS display_name,
          s.ZCANONICALNAME AS canonical_name,
          s.ZLIST AS "list_row_id!"
        FROM ZREMCDBASESECTION s
        WHERE s.ZMARKEDFORDELETION = 0
          AND s.ZLIST = ?
        ORDER BY s.Z_PK ASC
        "#,
        list_row_id,
    )
    .fetch_all(executor)
    .await
}

pub async fn fetch_attachment_by_uuid<'e, E>(
    executor: E,
    id: &str,
) -> Result<Option<AttachmentRow>, sqlx::Error>
where
    E: SqliteExecutor<'e>,
{
    sqlx::query_as!(
        AttachmentRow,
        r#"
        SELECT
          sa.Z_PK AS "row_id!",
          CAST(
            lower(
              substr(hex(sa.ZIDENTIFIER), 1, 8) || '-' ||
              substr(hex(sa.ZIDENTIFIER), 9, 4) || '-' ||
              substr(hex(sa.ZIDENTIFIER), 13, 4) || '-' ||
              substr(hex(sa.ZIDENTIFIER), 17, 4) || '-' ||
              substr(hex(sa.ZIDENTIFIER), 21, 12)
            ) AS TEXT
          ) AS "id!: String",
          sa.ZFILENAME AS filename,
          sa.ZUTI AS uti,
          sa.ZSHA512SUM AS sha512,
          sa.ZATTACHMENTTYPERAWVALUE AS kind_raw,
          sa.ZREMINDER AS "reminder_row_id!",
          CAST(sa.ZLASTMODIFIEDDATE AS REAL) AS modified_at
        FROM ZREMCDSAVEDATTACHMENT sa
        WHERE sa.ZMARKEDFORDELETION = 0
          AND lower(
            substr(hex(sa.ZIDENTIFIER), 1, 8) || '-' ||
            substr(hex(sa.ZIDENTIFIER), 9, 4) || '-' ||
            substr(hex(sa.ZIDENTIFIER), 13, 4) || '-' ||
            substr(hex(sa.ZIDENTIFIER), 17, 4) || '-' ||
            substr(hex(sa.ZIDENTIFIER), 21, 12)
          ) = ?
        "#,
        id,
    )
    .fetch_optional(executor)
    .await
}

pub async fn fetch_attachments_for_reminder<'e, E>(
    executor: E,
    reminder_row_id: i64,
) -> Result<Vec<AttachmentRow>, sqlx::Error>
where
    E: SqliteExecutor<'e>,
{
    sqlx::query_as!(
        AttachmentRow,
        r#"
        SELECT
          sa.Z_PK AS "row_id!",
          CAST(
            lower(
              substr(hex(sa.ZIDENTIFIER), 1, 8) || '-' ||
              substr(hex(sa.ZIDENTIFIER), 9, 4) || '-' ||
              substr(hex(sa.ZIDENTIFIER), 13, 4) || '-' ||
              substr(hex(sa.ZIDENTIFIER), 17, 4) || '-' ||
              substr(hex(sa.ZIDENTIFIER), 21, 12)
            ) AS TEXT
          ) AS "id!: String",
          sa.ZFILENAME AS filename,
          sa.ZUTI AS uti,
          sa.ZSHA512SUM AS sha512,
          sa.ZATTACHMENTTYPERAWVALUE AS kind_raw,
          sa.ZREMINDER AS "reminder_row_id!",
          CAST(sa.ZLASTMODIFIEDDATE AS REAL) AS modified_at
        FROM ZREMCDSAVEDATTACHMENT sa
        WHERE sa.ZMARKEDFORDELETION = 0
          AND sa.ZREMINDER = ?
        "#,
        reminder_row_id,
    )
    .fetch_all(executor)
    .await
}

#[derive(Debug, sqlx::FromRow)]
pub struct ReminderTagRow {
    pub reminder_row_id: i64,
    pub tag_name: String,
}

pub async fn fetch_tags_for_reminder_ids<'e, E>(
    executor: E,
    hashtag_ent: i64,
    ids_json: &str,
) -> Result<Vec<ReminderTagRow>, sqlx::Error>
where
    E: SqliteExecutor<'e>,
{
    sqlx::query_as!(
        ReminderTagRow,
        r#"
        SELECT
          o.ZREMINDER AS "reminder_row_id!",
          label.ZNAME AS "tag_name!: String"
        FROM ZREMCDOBJECT o
        JOIN ZREMCDHASHTAGLABEL label ON o.ZHASHTAGLABEL = label.Z_PK
        WHERE o.ZMARKEDFORDELETION = 0
          AND o.Z_ENT = ?
          AND o.ZREMINDER IN (SELECT value FROM json_each(?))
        "#,
        hashtag_ent,
        ids_json,
    )
    .fetch_all(executor)
    .await
}

pub async fn fetch_objects_for_reminder<'e, E>(
    executor: E,
    reminder_row_id: i64,
) -> Result<Vec<ObjectRow>, sqlx::Error>
where
    E: SqliteExecutor<'e>,
{
    sqlx::query_as!(
        ObjectRow,
        r#"
        SELECT
          o.Z_PK AS "row_id!",
          o.Z_ENT AS "ent!",
          o.ZREMINDER AS reminder_row_id,
          o.ZTYPE AS object_type,
          o.ZTITLE AS title,
          o.ZLATITUDE AS latitude,
          o.ZLONGITUDE AS longitude,
          o.ZRADIUS AS radius,
          o.ZTIMEINTERVAL AS time_interval,
          o.ZDATECOMPONENTSDATA AS "date_components_data: Vec<u8>",
          o.Z16_TRIGGER AS trigger_row_id,
          o.ZFREQUENCY AS frequency,
          o.ZINTERVAL AS recurrence_interval,
          o.ZOCCURRENCECOUNT AS occurrence_count,
          o.ZHASHTAGLABEL AS hashtag_label_row_id,
          CAST(NULL AS TEXT) AS "tag_name: String"
        FROM ZREMCDOBJECT o
        WHERE o.ZMARKEDFORDELETION = 0
          AND o.ZREMINDER = ?
        "#,
        reminder_row_id,
    )
    .fetch_all(executor)
    .await
}

pub async fn fetch_recurrence_objects_for_reminder<'e, E>(
    executor: E,
    reminder_row_id: i64,
    recurrence_ent: i64,
) -> Result<Vec<ObjectRow>, sqlx::Error>
where
    E: SqliteExecutor<'e>,
{
    sqlx::query_as!(
        ObjectRow,
        r#"
        SELECT
          o.Z_PK AS "row_id!",
          o.Z_ENT AS "ent!",
          o.ZREMINDER AS reminder_row_id,
          o.ZTYPE AS object_type,
          o.ZTITLE AS title,
          o.ZLATITUDE AS latitude,
          o.ZLONGITUDE AS longitude,
          o.ZRADIUS AS radius,
          o.ZTIMEINTERVAL AS time_interval,
          o.ZDATECOMPONENTSDATA AS "date_components_data: Vec<u8>",
          o.Z16_TRIGGER AS trigger_row_id,
          o.ZFREQUENCY AS frequency,
          o.ZINTERVAL AS recurrence_interval,
          o.ZOCCURRENCECOUNT AS occurrence_count,
          o.ZHASHTAGLABEL AS hashtag_label_row_id,
          CAST(NULL AS TEXT) AS "tag_name: String"
        FROM ZREMCDOBJECT o
        WHERE o.ZMARKEDFORDELETION = 0
          AND o.ZREMINDER = ?
          AND o.Z_ENT = ?
        "#,
        reminder_row_id,
        recurrence_ent,
    )
    .fetch_all(executor)
    .await
}

pub async fn fetch_reminder_uuid_for_row<'e, E>(
    executor: E,
    row_id: i64,
) -> Result<Option<String>, sqlx::Error>
where
    E: SqliteExecutor<'e>,
{
    sqlx::query_scalar!(
        r#"
        SELECT CAST(
          lower(
            substr(hex(r.ZIDENTIFIER), 1, 8) || '-' ||
            substr(hex(r.ZIDENTIFIER), 9, 4) || '-' ||
            substr(hex(r.ZIDENTIFIER), 13, 4) || '-' ||
            substr(hex(r.ZIDENTIFIER), 17, 4) || '-' ||
            substr(hex(r.ZIDENTIFIER), 21, 12)
          ) AS TEXT
        ) AS "id!: String"
        FROM ZREMCDREMINDER r
        WHERE r.Z_PK = ?
          AND r.ZMARKEDFORDELETION = 0
        "#,
        row_id,
    )
    .fetch_optional(executor)
    .await
}

#[derive(Debug)]
pub struct ListResolveRow {
    pub api_id: String,
    pub external_id: Option<String>,
    pub title: String,
    pub ent: i64,
    pub smart_list_type: Option<String>,
}

pub async fn fetch_list_resolve_metadata(
    pool: &SqlitePool,
    list_id: &str,
) -> Result<Option<ListResolveRow>, sqlx::Error> {
    sqlx::query_as!(
        ListResolveRow,
        r#"
        SELECT
          CAST(
            lower(
              substr(hex(l.ZIDENTIFIER), 1, 8) || '-' ||
              substr(hex(l.ZIDENTIFIER), 9, 4) || '-' ||
              substr(hex(l.ZIDENTIFIER), 13, 4) || '-' ||
              substr(hex(l.ZIDENTIFIER), 17, 4) || '-' ||
              substr(hex(l.ZIDENTIFIER), 21, 12)
            ) AS TEXT
          ) AS "api_id!: String",
          l.ZEXTERNALIDENTIFIER AS external_id,
          COALESCE(l.ZNAME, '') AS "title!: String",
          l.Z_ENT AS "ent!",
          l.ZSMARTLISTTYPE AS smart_list_type
        FROM ZREMCDBASELIST l
        WHERE lower(
          substr(hex(l.ZIDENTIFIER), 1, 8) || '-' ||
          substr(hex(l.ZIDENTIFIER), 9, 4) || '-' ||
          substr(hex(l.ZIDENTIFIER), 13, 4) || '-' ||
          substr(hex(l.ZIDENTIFIER), 17, 4) || '-' ||
          substr(hex(l.ZIDENTIFIER), 21, 12)
        ) = ?
        "#,
        list_id,
    )
    .fetch_optional(pool)
    .await
}

pub async fn fetch_reminder_external_id(
    pool: &SqlitePool,
    reminder_id: &str,
) -> Result<Option<String>, sqlx::Error> {
    Ok(sqlx::query_scalar!(
        r#"
        SELECT r.ZEXTERNALIDENTIFIER
        FROM ZREMCDREMINDER r
        WHERE lower(
          substr(hex(r.ZIDENTIFIER), 1, 8) || '-' ||
          substr(hex(r.ZIDENTIFIER), 9, 4) || '-' ||
          substr(hex(r.ZIDENTIFIER), 13, 4) || '-' ||
          substr(hex(r.ZIDENTIFIER), 17, 4) || '-' ||
          substr(hex(r.ZIDENTIFIER), 21, 12)
        ) = ?
        "#,
        reminder_id,
    )
    .fetch_optional(pool)
    .await?
    .flatten())
}

#[allow(dead_code)]
pub async fn fetch_list_membership_data<'e, E>(
    executor: E,
    list_row_id: i64,
) -> Result<Option<Vec<u8>>, sqlx::Error>
where
    E: SqliteExecutor<'e>,
{
    Ok(sqlx::query_scalar!(
        r#"
        SELECT ZMEMBERSHIPSOFREMINDERSINSECTIONSASDATA AS "membership_data?: Vec<u8>"
        FROM ZREMCDBASELIST
        WHERE Z_PK = ?
          AND ZMARKEDFORDELETION = 0
        "#,
        list_row_id,
    )
    .fetch_optional(executor)
    .await?
    .flatten())
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ListMembershipRow {
    pub list_row_id: i64,
    pub membership_data: Option<Vec<u8>>,
}

pub async fn list_exists<'e, E>(executor: E, list_row_id: i64) -> Result<bool, sqlx::Error>
where
    E: SqliteExecutor<'e>,
{
    record_test_query();
    sqlx::query_scalar!(
        r#"
        SELECT EXISTS(
            SELECT 1
            FROM ZREMCDBASELIST
            WHERE Z_PK = ?1
              AND ZMARKEDFORDELETION = 0
        ) AS "exists!: bool"
        "#,
        list_row_id,
    )
    .fetch_one(executor)
    .await
}

pub async fn fetch_list_membership_data_batch<'e, E>(
    executor: E,
    list_row_ids_json: &str,
) -> Result<Vec<ListMembershipRow>, sqlx::Error>
where
    E: SqliteExecutor<'e>,
{
    record_test_query();
    sqlx::query_as!(
        ListMembershipRow,
        r#"
        SELECT
            Z_PK AS "list_row_id!",
            ZMEMBERSHIPSOFREMINDERSINSECTIONSASDATA AS "membership_data?: Vec<u8>"
        FROM ZREMCDBASELIST
        WHERE Z_PK IN (SELECT value FROM json_each(?1))
          AND ZMARKEDFORDELETION = 0
        "#,
        list_row_ids_json,
    )
    .fetch_all(executor)
    .await
}
