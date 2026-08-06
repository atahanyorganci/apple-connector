//! Compile-time checked Notes queries.

use sqlx::{SqliteExecutor, SqlitePool};

use super::{
    row::{AttachmentRow, FolderRow, NoteDetailRow, NoteRow},
    search::NoteFilterBinds,
};

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct EntityNameRow {
    pub ent: i64,
    pub name: String,
}

pub async fn fetch_entity_name_rows(pool: &SqlitePool) -> Result<Vec<EntityNameRow>, sqlx::Error> {
    sqlx::query_as!(
        EntityNameRow,
        r#"
        SELECT
            Z_ENT AS "ent!",
            Z_NAME AS "name!"
        FROM Z_PRIMARYKEY
        WHERE Z_NAME IN (
            'ICNote', 'ICFolder', 'ICAttachment', 'ICAccount', 'ICHashtag'
        )
        "#,
    )
    .fetch_all(pool)
    .await
}

pub async fn count_folders(pool: &SqlitePool, folder_ent: i64) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar!(
        r#"
        SELECT COUNT(*) AS "count!: i64"
        FROM ZICCLOUDSYNCINGOBJECT
        WHERE Z_ENT = ?1 AND ZMARKEDFORDELETION = 0 AND ZFOLDERTYPE != 1
        "#,
        folder_ent,
    )
    .fetch_one(pool)
    .await
}

pub async fn count_notes(pool: &SqlitePool, note_ent: i64) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar!(
        r#"
        SELECT COUNT(*) AS "count!: i64"
        FROM ZICCLOUDSYNCINGOBJECT n
        LEFT JOIN ZICCLOUDSYNCINGOBJECT f ON n.ZFOLDER = f.Z_PK
        WHERE n.Z_ENT = ?1 AND n.ZMARKEDFORDELETION = 0
          AND (f.Z_PK IS NULL OR (f.ZMARKEDFORDELETION = 0 AND f.ZFOLDERTYPE != 1))
        "#,
        note_ent,
    )
    .fetch_one(pool)
    .await
}

pub async fn count_pinned_notes(pool: &SqlitePool, note_ent: i64) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar!(
        r#"
        SELECT COUNT(*) AS "count!: i64"
        FROM ZICCLOUDSYNCINGOBJECT
        WHERE Z_ENT = ?1 AND ZMARKEDFORDELETION = 0 AND ZISPINNED = 1
        "#,
        note_ent,
    )
    .fetch_one(pool)
    .await
}

pub async fn count_locked_notes(pool: &SqlitePool, note_ent: i64) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar!(
        r#"
        SELECT COUNT(*) AS "count!: i64"
        FROM ZICCLOUDSYNCINGOBJECT
        WHERE Z_ENT = ?1 AND ZMARKEDFORDELETION = 0 AND ZISPASSWORDPROTECTED = 1
        "#,
        note_ent,
    )
    .fetch_one(pool)
    .await
}

pub async fn count_notes_with_checklist(
    pool: &SqlitePool,
    note_ent: i64,
) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar!(
        r#"
        SELECT COUNT(*) AS "count!: i64"
        FROM ZICCLOUDSYNCINGOBJECT
        WHERE Z_ENT = ?1 AND ZMARKEDFORDELETION = 0 AND ZHASCHECKLIST = 1
        "#,
        note_ent,
    )
    .fetch_one(pool)
    .await
}

pub async fn count_attachments(pool: &SqlitePool, attachment_ent: i64) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar!(
        r#"
        SELECT COUNT(*) AS "count!: i64"
        FROM ZICCLOUDSYNCINGOBJECT
        WHERE Z_ENT = ?1 AND ZMARKEDFORDELETION = 0
        "#,
        attachment_ent,
    )
    .fetch_one(pool)
    .await
}

pub async fn count_deleted_notes(pool: &SqlitePool, note_ent: i64) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar!(
        r#"
        SELECT COUNT(*) AS "count!: i64"
        FROM ZICCLOUDSYNCINGOBJECT n
        LEFT JOIN ZICCLOUDSYNCINGOBJECT f ON n.ZFOLDER = f.Z_PK
        WHERE n.Z_ENT = ?1 AND (n.ZMARKEDFORDELETION = 1 OR f.ZFOLDERTYPE = 1)
        "#,
        note_ent,
    )
    .fetch_one(pool)
    .await
}

pub async fn list_folders(
    pool: &SqlitePool,
    folder_ent: i64,
    include_deleted: i64,
    cursor_row_id: Option<i64>,
    limit: i64,
) -> Result<Vec<FolderRow>, sqlx::Error> {
    sqlx::query_as!(
        FolderRow,
        r#"
        SELECT
            f.Z_PK AS "row_id!",
            f.ZIDENTIFIER AS "id!",
            f.ZTITLE2 AS title,
            f.ZFOLDERTYPE AS folder_type,
            f.ZPARENT AS parent_row_id,
            p.ZIDENTIFIER AS parent_id,
            f.ZACCOUNT8 AS account_row_id,
            a.ZIDENTIFIER AS account_id,
            CAST(f.ZFOLDERMODIFICATIONDATE AS REAL) AS "modified_at?: f64"
        FROM ZICCLOUDSYNCINGOBJECT f
        LEFT JOIN ZICCLOUDSYNCINGOBJECT p ON f.ZPARENT = p.Z_PK
        LEFT JOIN ZICCLOUDSYNCINGOBJECT a ON f.ZACCOUNT8 = a.Z_PK
        WHERE f.Z_ENT = ?1
          AND f.ZMARKEDFORDELETION = 0
          AND (?2 = 1 OR f.ZFOLDERTYPE != 1)
          AND (?3 IS NULL OR f.Z_PK < ?3)
        ORDER BY f.Z_PK DESC
        LIMIT ?4
        "#,
        folder_ent,
        include_deleted,
        cursor_row_id,
        limit,
    )
    .fetch_all(pool)
    .await
}

pub async fn get_folder_by_row_id(
    pool: &SqlitePool,
    folder_ent: i64,
    folder_row_id: i64,
) -> Result<Option<FolderRow>, sqlx::Error> {
    sqlx::query_as!(
        FolderRow,
        r#"
        SELECT
            f.Z_PK AS "row_id!",
            f.ZIDENTIFIER AS "id!",
            f.ZTITLE2 AS title,
            f.ZFOLDERTYPE AS folder_type,
            f.ZPARENT AS parent_row_id,
            p.ZIDENTIFIER AS parent_id,
            f.ZACCOUNT8 AS account_row_id,
            a.ZIDENTIFIER AS account_id,
            CAST(f.ZFOLDERMODIFICATIONDATE AS REAL) AS "modified_at?: f64"
        FROM ZICCLOUDSYNCINGOBJECT f
        LEFT JOIN ZICCLOUDSYNCINGOBJECT p ON f.ZPARENT = p.Z_PK
        LEFT JOIN ZICCLOUDSYNCINGOBJECT a ON f.ZACCOUNT8 = a.Z_PK
        WHERE f.Z_ENT = ?1
          AND f.ZMARKEDFORDELETION = 0
          AND f.ZFOLDERTYPE != 1
          AND f.Z_PK = ?2
        "#,
        folder_ent,
        folder_row_id,
    )
    .fetch_optional(pool)
    .await
}

pub async fn get_folder_by_identifier(
    pool: &SqlitePool,
    folder_ent: i64,
    identifier: &str,
) -> Result<Option<FolderRow>, sqlx::Error> {
    sqlx::query_as!(
        FolderRow,
        r#"
        SELECT
            f.Z_PK AS "row_id!",
            f.ZIDENTIFIER AS "id!",
            f.ZTITLE2 AS title,
            f.ZFOLDERTYPE AS folder_type,
            f.ZPARENT AS parent_row_id,
            p.ZIDENTIFIER AS parent_id,
            f.ZACCOUNT8 AS account_row_id,
            a.ZIDENTIFIER AS account_id,
            CAST(f.ZFOLDERMODIFICATIONDATE AS REAL) AS "modified_at?: f64"
        FROM ZICCLOUDSYNCINGOBJECT f
        LEFT JOIN ZICCLOUDSYNCINGOBJECT p ON f.ZPARENT = p.Z_PK
        LEFT JOIN ZICCLOUDSYNCINGOBJECT a ON f.ZACCOUNT8 = a.Z_PK
        WHERE f.Z_ENT = ?1
          AND f.ZMARKEDFORDELETION = 0
          AND f.ZFOLDERTYPE != 1
          AND lower(f.ZIDENTIFIER) = ?2
        "#,
        folder_ent,
        identifier,
    )
    .fetch_optional(pool)
    .await
}

pub async fn fetch_filtered_notes<'e, E>(
    executor: E,
    binds: &NoteFilterBinds,
) -> Result<Vec<NoteRow>, sqlx::Error>
where
    E: SqliteExecutor<'e>,
{
    sqlx::query_as!(
        NoteRow,
        r#"
        SELECT
            n.Z_PK AS "row_id!",
            n.ZIDENTIFIER AS "id!",
            n.ZTITLE1 AS title,
            n.ZSNIPPET AS snippet,
            CAST(n.ZCREATIONDATE1 AS REAL) AS "created_at?: f64",
            CAST(n.ZMODIFICATIONDATE1 AS REAL) AS "modified_at?: f64",
            n.ZFOLDER AS folder_row_id,
            f.ZIDENTIFIER AS folder_id,
            f.ZTITLE2 AS folder_name,
            f.ZFOLDERTYPE AS folder_type,
            n.ZISPINNED AS "is_pinned!: bool",
            n.ZHASCHECKLIST AS "has_checklist!: bool",
            n.ZISPASSWORDPROTECTED AS "is_locked!: bool",
            n.ZMARKEDFORDELETION AS "marked_for_deletion!: bool"
        FROM ZICCLOUDSYNCINGOBJECT n
        LEFT JOIN ZICCLOUDSYNCINGOBJECT f ON n.ZFOLDER = f.Z_PK
        WHERE n.Z_ENT = ?1
          AND (?2 = 1 OR n.ZMARKEDFORDELETION = 0)
          AND (
            ?2 = 1
            OR f.Z_PK IS NULL
            OR (f.ZMARKEDFORDELETION = 0 AND f.ZFOLDERTYPE != 1)
          )
          AND (?3 IS NULL OR n.ZISPINNED = ?3)
          AND (?4 IS NULL OR n.ZISPASSWORDPROTECTED = ?4)
          AND (?5 IS NULL OR n.ZHASCHECKLIST = ?5)
          AND (?6 IS NULL OR n.ZFOLDER = ?6)
          AND (?7 IS NULL OR lower(f.ZIDENTIFIER) = ?7)
          AND (?8 IS NULL OR n.ZMODIFICATIONDATE1 <= ?8)
          AND (?9 IS NULL OR n.ZMODIFICATIONDATE1 >= ?9)
          AND (
            ?10 IS NULL
            OR (
              ?10 = 1
              AND EXISTS (
                SELECT 1 FROM ZICCLOUDSYNCINGOBJECT a
                WHERE a.ZNOTE = n.Z_PK AND a.Z_ENT = ?11 AND a.ZMARKEDFORDELETION = 0
              )
            )
            OR (
              ?10 = 0
              AND NOT EXISTS (
                SELECT 1 FROM ZICCLOUDSYNCINGOBJECT a
                WHERE a.ZNOTE = n.Z_PK AND a.Z_ENT = ?11 AND a.ZMARKEDFORDELETION = 0
              )
            )
          )
          AND (
            ?12 IS NULL
            OR lower(coalesce(n.ZTITLE1, '')) LIKE '%' || lower(?12) || '%'
            OR lower(coalesce(n.ZSNIPPET, '')) LIKE '%' || lower(?12) || '%'
          )
          AND (
            ?13 IS NULL
            OR n.ZMODIFICATIONDATE1 < ?13
            OR (n.ZMODIFICATIONDATE1 = ?13 AND n.Z_PK < ?14)
          )
        ORDER BY n.ZMODIFICATIONDATE1 DESC, n.Z_PK DESC
        LIMIT ?15
        "#,
        binds.note_ent,
        binds.include_deleted,
        binds.is_pinned,
        binds.is_locked,
        binds.has_checklist,
        binds.folder_row_id,
        binds.folder_identifier,
        binds.modified_before,
        binds.modified_after,
        binds.has_attachments,
        binds.attachment_ent,
        binds.q,
        binds.cursor_modified_at,
        binds.cursor_row_id,
        binds.limit,
    )
    .fetch_all(executor)
    .await
}

pub async fn fetch_filtered_note_details<'e, E>(
    executor: E,
    binds: &NoteFilterBinds,
) -> Result<Vec<NoteDetailRow>, sqlx::Error>
where
    E: SqliteExecutor<'e>,
{
    sqlx::query_as!(
        NoteDetailRow,
        r#"
        SELECT
            n.Z_PK AS "row_id!",
            n.ZIDENTIFIER AS "id!",
            n.ZTITLE1 AS title,
            n.ZSNIPPET AS snippet,
            CAST(n.ZCREATIONDATE1 AS REAL) AS "created_at?: f64",
            CAST(n.ZMODIFICATIONDATE1 AS REAL) AS "modified_at?: f64",
            n.ZFOLDER AS folder_row_id,
            f.ZIDENTIFIER AS folder_id,
            f.ZTITLE2 AS folder_name,
            f.ZFOLDERTYPE AS folder_type,
            n.ZISPINNED AS "is_pinned!: bool",
            n.ZHASCHECKLIST AS "has_checklist!: bool",
            n.ZISPASSWORDPROTECTED AS "is_locked!: bool",
            n.ZMARKEDFORDELETION AS "marked_for_deletion!: bool",
            nd.ZDATA AS "note_data?: Vec<u8>"
        FROM ZICCLOUDSYNCINGOBJECT n
        LEFT JOIN ZICCLOUDSYNCINGOBJECT f ON n.ZFOLDER = f.Z_PK
        LEFT JOIN ZICNOTEDATA nd ON n.ZNOTEDATA = nd.Z_PK
        WHERE n.Z_ENT = ?1
          AND (?2 = 1 OR n.ZMARKEDFORDELETION = 0)
          AND (
            ?2 = 1
            OR f.Z_PK IS NULL
            OR (f.ZMARKEDFORDELETION = 0 AND f.ZFOLDERTYPE != 1)
          )
          AND (?3 IS NULL OR n.ZISPINNED = ?3)
          AND (?4 IS NULL OR n.ZISPASSWORDPROTECTED = ?4)
          AND (?5 IS NULL OR n.ZHASCHECKLIST = ?5)
          AND (?6 IS NULL OR n.ZFOLDER = ?6)
          AND (?7 IS NULL OR lower(f.ZIDENTIFIER) = ?7)
          AND (?8 IS NULL OR n.ZMODIFICATIONDATE1 <= ?8)
          AND (?9 IS NULL OR n.ZMODIFICATIONDATE1 >= ?9)
          AND (
            ?10 IS NULL
            OR (
              ?10 = 1
              AND EXISTS (
                SELECT 1 FROM ZICCLOUDSYNCINGOBJECT a
                WHERE a.ZNOTE = n.Z_PK AND a.Z_ENT = ?11 AND a.ZMARKEDFORDELETION = 0
              )
            )
            OR (
              ?10 = 0
              AND NOT EXISTS (
                SELECT 1 FROM ZICCLOUDSYNCINGOBJECT a
                WHERE a.ZNOTE = n.Z_PK AND a.Z_ENT = ?11 AND a.ZMARKEDFORDELETION = 0
              )
            )
          )
          AND (
            ?12 IS NULL
            OR lower(coalesce(n.ZTITLE1, '')) LIKE '%' || lower(?12) || '%'
            OR lower(coalesce(n.ZSNIPPET, '')) LIKE '%' || lower(?12) || '%'
          )
          AND (
            ?13 IS NULL
            OR n.ZMODIFICATIONDATE1 < ?13
            OR (n.ZMODIFICATIONDATE1 = ?13 AND n.Z_PK < ?14)
          )
        ORDER BY n.ZMODIFICATIONDATE1 DESC, n.Z_PK DESC
        LIMIT ?15
        "#,
        binds.note_ent,
        binds.include_deleted,
        binds.is_pinned,
        binds.is_locked,
        binds.has_checklist,
        binds.folder_row_id,
        binds.folder_identifier,
        binds.modified_before,
        binds.modified_after,
        binds.has_attachments,
        binds.attachment_ent,
        binds.q,
        binds.cursor_modified_at,
        binds.cursor_row_id,
        binds.limit,
    )
    .fetch_all(executor)
    .await
}

pub async fn get_note_by_identifier(
    pool: &SqlitePool,
    note_ent: i64,
    identifier: &str,
) -> Result<Option<NoteDetailRow>, sqlx::Error> {
    sqlx::query_as!(
        NoteDetailRow,
        r#"
        SELECT
            n.Z_PK AS "row_id!",
            n.ZIDENTIFIER AS "id!",
            n.ZTITLE1 AS title,
            n.ZSNIPPET AS snippet,
            CAST(n.ZCREATIONDATE1 AS REAL) AS "created_at?: f64",
            CAST(n.ZMODIFICATIONDATE1 AS REAL) AS "modified_at?: f64",
            n.ZFOLDER AS folder_row_id,
            f.ZIDENTIFIER AS folder_id,
            f.ZTITLE2 AS folder_name,
            f.ZFOLDERTYPE AS folder_type,
            n.ZISPINNED AS "is_pinned!: bool",
            n.ZHASCHECKLIST AS "has_checklist!: bool",
            n.ZISPASSWORDPROTECTED AS "is_locked!: bool",
            n.ZMARKEDFORDELETION AS "marked_for_deletion!: bool",
            nd.ZDATA AS "note_data?: Vec<u8>"
        FROM ZICCLOUDSYNCINGOBJECT n
        LEFT JOIN ZICCLOUDSYNCINGOBJECT f ON n.ZFOLDER = f.Z_PK
        LEFT JOIN ZICNOTEDATA nd ON n.ZNOTEDATA = nd.Z_PK
        WHERE n.Z_ENT = ?1
          AND n.ZMARKEDFORDELETION = 0
          AND (f.Z_PK IS NULL OR (f.ZMARKEDFORDELETION = 0 AND f.ZFOLDERTYPE != 1))
          AND lower(n.ZIDENTIFIER) = ?2
        "#,
        note_ent,
        identifier,
    )
    .fetch_optional(pool)
    .await
}

pub async fn fetch_tags_for_note(
    pool: &SqlitePool,
    note_row_id: i64,
) -> Result<Vec<(Option<String>,)>, sqlx::Error> {
    sqlx::query_as!(
        TagNameRow,
        r#"
        SELECT o.ZALTTEXT AS tag_name
        FROM ZICCLOUDSYNCINGOBJECT o
        WHERE o.ZMARKEDFORDELETION = 0
          AND o.ZTYPEUTI1 = 'com.apple.notes.inlinetextattachment.hashtag'
          AND o.ZNOTE1 = ?1
        ORDER BY o.Z_PK
        "#,
        note_row_id,
    )
    .fetch_all(pool)
    .await
    .map(|rows| rows.into_iter().map(|row| (row.tag_name,)).collect())
}

#[derive(Debug, sqlx::FromRow)]
struct TagNameRow {
    tag_name: Option<String>,
}

pub async fn list_attachments_for_note(
    pool: &SqlitePool,
    attachment_ent: i64,
    note_row_id: i64,
) -> Result<Vec<AttachmentRow>, sqlx::Error> {
    sqlx::query_as!(
        AttachmentRow,
        r#"
        SELECT
            a.Z_PK AS "row_id!",
            a.ZIDENTIFIER AS "id!",
            a.ZFILENAME AS filename,
            a.ZTYPEUTI AS uti,
            a.ZNOTE AS "note_row_id!",
            n.ZIDENTIFIER AS "note_id!",
            a.ZFILESIZE AS file_size,
            CAST(a.ZMODIFICATIONDATE1 AS REAL) AS "modified_at?: f64",
            acc.ZIDENTIFIER AS account_id
        FROM ZICCLOUDSYNCINGOBJECT a
        JOIN ZICCLOUDSYNCINGOBJECT n ON a.ZNOTE = n.Z_PK
        LEFT JOIN ZICCLOUDSYNCINGOBJECT acc ON a.ZACCOUNT6 = acc.Z_PK
        WHERE a.Z_ENT = ?1
          AND a.ZMARKEDFORDELETION = 0
          AND a.ZNOTE = ?2
        ORDER BY a.Z_PK ASC
        "#,
        attachment_ent,
        note_row_id,
    )
    .fetch_all(pool)
    .await
}

pub async fn get_attachment_by_identifier(
    pool: &SqlitePool,
    attachment_ent: i64,
    identifier: &str,
) -> Result<Option<AttachmentRow>, sqlx::Error> {
    sqlx::query_as!(
        AttachmentRow,
        r#"
        SELECT
            a.Z_PK AS "row_id!",
            a.ZIDENTIFIER AS "id!",
            a.ZFILENAME AS filename,
            a.ZTYPEUTI AS uti,
            a.ZNOTE AS "note_row_id!",
            n.ZIDENTIFIER AS "note_id!",
            a.ZFILESIZE AS file_size,
            CAST(a.ZMODIFICATIONDATE1 AS REAL) AS "modified_at?: f64",
            acc.ZIDENTIFIER AS account_id
        FROM ZICCLOUDSYNCINGOBJECT a
        JOIN ZICCLOUDSYNCINGOBJECT n ON a.ZNOTE = n.Z_PK
        LEFT JOIN ZICCLOUDSYNCINGOBJECT acc ON a.ZACCOUNT6 = acc.Z_PK
        WHERE a.Z_ENT = ?1
          AND a.ZMARKEDFORDELETION = 0
          AND lower(a.ZIDENTIFIER) = ?2
        "#,
        attachment_ent,
        identifier,
    )
    .fetch_optional(pool)
    .await
}

#[derive(Debug, sqlx::FromRow)]
pub struct NoteAttachmentFlagRow {
    pub note_row_id: i64,
}

pub async fn fetch_note_row_ids_with_attachments<'e, E>(
    executor: E,
    attachment_ent: i64,
    ids_json: &str,
) -> Result<Vec<NoteAttachmentFlagRow>, sqlx::Error>
where
    E: SqliteExecutor<'e>,
{
    sqlx::query_as!(
        NoteAttachmentFlagRow,
        r#"
        SELECT a.ZNOTE AS "note_row_id!"
        FROM ZICCLOUDSYNCINGOBJECT a
        WHERE a.Z_ENT = ?1
          AND a.ZMARKEDFORDELETION = 0
          AND a.ZNOTE IN (SELECT value FROM json_each(?2))
        "#,
        attachment_ent,
        ids_json,
    )
    .fetch_all(executor)
    .await
}
