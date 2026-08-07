//! Compile-time checked AddressBook queries.

use sqlx::SqliteExecutor;

#[cfg(test)]
fn record_test_query() {
    crate::db::query_budget::bump();
}

#[cfg(not(test))]
fn record_test_query() {}

use super::{
    row::{
        AddressRow, ContactRow, ContainerRow, EmailRow, GroupRow, PhoneRow, PhotoRow, SocialRow,
        UrlRow,
    },
    search::ContactFilterBinds,
};

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct EntityNameRow {
    pub ent: i64,
    pub name: String,
}

pub async fn fetch_entity_name_rows<'e, E>(executor: E) -> Result<Vec<EntityNameRow>, sqlx::Error>
where
    E: SqliteExecutor<'e>,
{
    sqlx::query_as!(
        EntityNameRow,
        r#"
        SELECT
            Z_ENT AS "ent!",
            Z_NAME AS "name!"
        FROM Z_PRIMARYKEY
        WHERE Z_NAME IN (
            'ABCDContact', 'ABCDGroup', 'CNCDContainer'
        )
        "#,
    )
    .fetch_all(executor)
    .await
}

pub async fn fetch_containers<'e, E>(
    executor: E,
    container_ent: i64,
) -> Result<Vec<ContainerRow>, sqlx::Error>
where
    E: SqliteExecutor<'e>,
{
    sqlx::query_as!(
        ContainerRow,
        r#"
        SELECT
            r.Z_PK AS "row_id!",
            r.ZUNIQUEID AS "unique_id!",
            r.ZNAME AS name,
            r.ZTYPE AS container_type
        FROM ZABCDRECORD r
        WHERE r.Z_ENT = ?1
        "#,
        container_ent,
    )
    .fetch_all(executor)
    .await
}

pub async fn fetch_container_by_api_id<'e, E>(
    executor: E,
    container_ent: i64,
    container_id: &str,
) -> Result<Option<ContainerRow>, sqlx::Error>
where
    E: SqliteExecutor<'e>,
{
    sqlx::query_as!(
        ContainerRow,
        r#"
        SELECT
            r.Z_PK AS "row_id!",
            r.ZUNIQUEID AS "unique_id!",
            r.ZNAME AS name,
            r.ZTYPE AS container_type
        FROM ZABCDRECORD r
        WHERE r.Z_ENT = ?1
          AND lower(substr(r.ZUNIQUEID, 1, instr(r.ZUNIQUEID, ':') - 1)) = lower(?2)
        "#,
        container_ent,
        container_id,
    )
    .fetch_optional(executor)
    .await
}

pub async fn fetch_groups<'e, E>(
    executor: E,
    group_ent: i64,
    cursor_row_id: Option<i64>,
    limit: i64,
) -> Result<Vec<GroupRow>, sqlx::Error>
where
    E: SqliteExecutor<'e>,
{
    sqlx::query_as!(
        GroupRow,
        r#"
        SELECT
            r.Z_PK AS "row_id!",
            r.ZUNIQUEID AS "unique_id!",
            r.ZNAME AS name,
            r.ZCONTAINER AS container_row_id,
            c.ZUNIQUEID AS container_unique_id,
            r.ZTYPE AS group_type
        FROM ZABCDRECORD r
        LEFT JOIN ZABCDRECORD c ON c.Z_PK = r.ZCONTAINER
        WHERE r.Z_ENT = ?1
          AND (?2 IS NULL OR r.Z_PK < ?2)
        ORDER BY r.Z_PK DESC
        LIMIT ?3
        "#,
        group_ent,
        cursor_row_id,
        limit,
    )
    .fetch_all(executor)
    .await
}

pub async fn fetch_group_by_api_id<'e, E>(
    executor: E,
    group_ent: i64,
    group_id: &str,
) -> Result<Option<GroupRow>, sqlx::Error>
where
    E: SqliteExecutor<'e>,
{
    sqlx::query_as!(
        GroupRow,
        r#"
        SELECT
            r.Z_PK AS "row_id!",
            r.ZUNIQUEID AS "unique_id!",
            r.ZNAME AS name,
            r.ZCONTAINER AS container_row_id,
            c.ZUNIQUEID AS container_unique_id,
            r.ZTYPE AS group_type
        FROM ZABCDRECORD r
        LEFT JOIN ZABCDRECORD c ON c.Z_PK = r.ZCONTAINER
        WHERE r.Z_ENT = ?1
          AND lower(substr(r.ZUNIQUEID, 1, instr(r.ZUNIQUEID, ':') - 1)) = lower(?2)
        "#,
        group_ent,
        group_id,
    )
    .fetch_optional(executor)
    .await
}

pub async fn fetch_filtered_contacts<'e, E>(
    executor: E,
    contact_ent: i64,
    binds: &ContactFilterBinds,
) -> Result<Vec<ContactRow>, sqlx::Error>
where
    E: SqliteExecutor<'e>,
{
    sqlx::query_as!(
        ContactRow,
        r#"
        SELECT
            r.Z_PK AS "row_id!",
            r.ZUNIQUEID AS "unique_id!",
            r.ZFIRSTNAME AS first_name,
            r.ZLASTNAME AS last_name,
            r.ZMIDDLENAME AS middle_name,
            r.ZNICKNAME AS nickname,
            r.ZORGANIZATION AS organization,
            r.ZJOBTITLE AS job_title,
            r.ZDEPARTMENT AS department,
            r.ZNAME AS display_name,
            r.ZCONTAINER AS container_row_id,
            c.ZUNIQUEID AS container_unique_id,
            CAST(r.ZCREATIONDATE AS REAL) AS creation_date,
            CAST(r.ZMODIFICATIONDATE AS REAL) AS modification_date,
            CAST(r.ZBIRTHDAY AS REAL) AS birthday,
            n.ZTEXT AS note_text,
            CASE WHEN l.ZDATA IS NOT NULL OR r.ZIMAGEDATA IS NOT NULL THEN 1 ELSE 0 END AS has_photo
        FROM ZABCDRECORD r
        LEFT JOIN ZABCDRECORD c ON c.Z_PK = r.ZCONTAINER
        LEFT JOIN ZABCDNOTE n ON n.ZCONTACT = r.Z_PK
        LEFT JOIN ZABCDLIKENESS l ON l.ZOWNER = r.Z_PK AND l.ZISPRIMARY = 1
        WHERE r.Z_ENT = ?1
          AND (
            ?2 IS NULL
            OR (
                r.ZFIRSTNAME LIKE ?2
                OR r.ZLASTNAME LIKE ?2
                OR r.ZORGANIZATION LIKE ?2
                OR r.ZNAME LIKE ?2
            )
          )
          AND (
            ?3 IS NULL
            OR lower(substr(c.ZUNIQUEID, 1, instr(c.ZUNIQUEID, ':') - 1)) = lower(?3)
          )
          AND (
            ?4 IS NULL
            OR r.Z_PK IN (
                SELECT pg.Z_22CONTACTS
                FROM Z_22PARENTGROUPS pg
                JOIN ZABCDRECORD g ON g.Z_PK = pg.Z_19PARENTGROUPS1
                WHERE lower(substr(g.ZUNIQUEID, 1, instr(g.ZUNIQUEID, ':') - 1)) = lower(?4)
            )
          )
          AND (?5 IS NULL OR r.Z_PK < ?5)
        ORDER BY r.Z_PK DESC
        LIMIT ?6
        "#,
        contact_ent,
        binds.q_pattern,
        binds.container_id,
        binds.group_id,
        binds.cursor_row_id,
        binds.limit,
    )
    .fetch_all(executor)
    .await
}

pub async fn fetch_group_contacts<'e, E>(
    executor: E,
    contact_ent: i64,
    group_id: &str,
    cursor_row_id: Option<i64>,
    limit: i64,
) -> Result<Vec<ContactRow>, sqlx::Error>
where
    E: SqliteExecutor<'e>,
{
    sqlx::query_as!(
        ContactRow,
        r#"
        SELECT
            r.Z_PK AS "row_id!",
            r.ZUNIQUEID AS "unique_id!",
            r.ZFIRSTNAME AS first_name,
            r.ZLASTNAME AS last_name,
            r.ZMIDDLENAME AS middle_name,
            r.ZNICKNAME AS nickname,
            r.ZORGANIZATION AS organization,
            r.ZJOBTITLE AS job_title,
            r.ZDEPARTMENT AS department,
            r.ZNAME AS display_name,
            r.ZCONTAINER AS container_row_id,
            c.ZUNIQUEID AS container_unique_id,
            CAST(r.ZCREATIONDATE AS REAL) AS creation_date,
            CAST(r.ZMODIFICATIONDATE AS REAL) AS modification_date,
            CAST(r.ZBIRTHDAY AS REAL) AS birthday,
            n.ZTEXT AS note_text,
            CASE WHEN l.ZDATA IS NOT NULL OR r.ZIMAGEDATA IS NOT NULL THEN 1 ELSE 0 END AS has_photo
        FROM ZABCDRECORD r
        LEFT JOIN ZABCDRECORD c ON c.Z_PK = r.ZCONTAINER
        LEFT JOIN ZABCDNOTE n ON n.ZCONTACT = r.Z_PK
        LEFT JOIN ZABCDLIKENESS l ON l.ZOWNER = r.Z_PK AND l.ZISPRIMARY = 1
        WHERE r.Z_ENT = ?1
          AND r.Z_PK IN (
              SELECT pg.Z_22CONTACTS
              FROM Z_22PARENTGROUPS pg
              JOIN ZABCDRECORD g ON g.Z_PK = pg.Z_19PARENTGROUPS1
              WHERE lower(substr(g.ZUNIQUEID, 1, instr(g.ZUNIQUEID, ':') - 1)) = lower(?2)
          )
          AND (?3 IS NULL OR r.Z_PK < ?3)
        ORDER BY r.Z_PK DESC
        LIMIT ?4
        "#,
        contact_ent,
        group_id,
        cursor_row_id,
        limit,
    )
    .fetch_all(executor)
    .await
}

pub async fn fetch_contact_by_api_id<'e, E>(
    executor: E,
    contact_ent: i64,
    contact_id: &str,
) -> Result<Option<ContactRow>, sqlx::Error>
where
    E: SqliteExecutor<'e>,
{
    sqlx::query_as!(
        ContactRow,
        r#"
        SELECT
            r.Z_PK AS "row_id!",
            r.ZUNIQUEID AS "unique_id!",
            r.ZFIRSTNAME AS first_name,
            r.ZLASTNAME AS last_name,
            r.ZMIDDLENAME AS middle_name,
            r.ZNICKNAME AS nickname,
            r.ZORGANIZATION AS organization,
            r.ZJOBTITLE AS job_title,
            r.ZDEPARTMENT AS department,
            r.ZNAME AS display_name,
            r.ZCONTAINER AS container_row_id,
            c.ZUNIQUEID AS container_unique_id,
            CAST(r.ZCREATIONDATE AS REAL) AS creation_date,
            CAST(r.ZMODIFICATIONDATE AS REAL) AS modification_date,
            CAST(r.ZBIRTHDAY AS REAL) AS birthday,
            n.ZTEXT AS note_text,
            CASE WHEN l.ZDATA IS NOT NULL OR r.ZIMAGEDATA IS NOT NULL THEN 1 ELSE 0 END AS has_photo
        FROM ZABCDRECORD r
        LEFT JOIN ZABCDRECORD c ON c.Z_PK = r.ZCONTAINER
        LEFT JOIN ZABCDNOTE n ON n.ZCONTACT = r.Z_PK
        LEFT JOIN ZABCDLIKENESS l ON l.ZOWNER = r.Z_PK AND l.ZISPRIMARY = 1
        WHERE r.Z_ENT = ?1
          AND lower(substr(r.ZUNIQUEID, 1, instr(r.ZUNIQUEID, ':') - 1)) = lower(?2)
        "#,
        contact_ent,
        contact_id,
    )
    .fetch_optional(executor)
    .await
}

pub async fn fetch_contact_photo<'e, E>(
    executor: E,
    contact_ent: i64,
    contact_id_prefix: &str,
) -> Result<Option<PhotoRow>, sqlx::Error>
where
    E: SqliteExecutor<'e>,
{
    sqlx::query_as!(
        PhotoRow,
        r#"
        SELECT
            COALESCE(l.ZDATA, r.ZIMAGEDATA) AS "photo_data: Vec<u8>",
            r.ZIMAGETYPE AS image_type
        FROM ZABCDRECORD r
        LEFT JOIN ZABCDLIKENESS l ON l.ZOWNER = r.Z_PK AND l.ZISPRIMARY = 1
        WHERE r.Z_ENT = ?1
          AND lower(r.ZUNIQUEID) LIKE lower(?2) || '%'
        LIMIT 1
        "#,
        contact_ent,
        contact_id_prefix,
    )
    .fetch_optional(executor)
    .await
}

pub async fn fetch_container_resolve_metadata<'e, E>(
    executor: E,
    container_ent: i64,
    api_id: &str,
) -> Result<Option<ContainerResolveRow>, sqlx::Error>
where
    E: SqliteExecutor<'e>,
{
    sqlx::query_as!(
        ContainerResolveRow,
        r#"
        SELECT
            CAST(lower(substr(r.ZUNIQUEID, 1, instr(r.ZUNIQUEID, ':') - 1)) AS TEXT) AS "api_id: String",
            r.ZUNIQUEID AS "external_id!",
            r.ZNAME AS name,
            r.ZTYPE AS container_type
        FROM ZABCDRECORD r
        WHERE r.Z_ENT = ?1
          AND lower(substr(r.ZUNIQUEID, 1, instr(r.ZUNIQUEID, ':') - 1)) = lower(?2)
        "#,
        container_ent,
        api_id,
    )
    .fetch_optional(executor)
    .await
}

pub async fn fetch_group_resolve_metadata<'e, E>(
    executor: E,
    group_ent: i64,
    api_id: &str,
) -> Result<Option<GroupResolveRow>, sqlx::Error>
where
    E: SqliteExecutor<'e>,
{
    sqlx::query_as!(
        GroupResolveRow,
        r#"
        SELECT
            CAST(lower(substr(r.ZUNIQUEID, 1, instr(r.ZUNIQUEID, ':') - 1)) AS TEXT) AS "api_id: String",
            r.ZNAME AS name,
            r.ZTYPE AS group_type,
            r.ZCONTAINER AS container_id
        FROM ZABCDRECORD r
        WHERE r.Z_ENT = ?1
          AND lower(substr(r.ZUNIQUEID, 1, instr(r.ZUNIQUEID, ':') - 1)) = lower(?2)
        "#,
        group_ent,
        api_id,
    )
    .fetch_optional(executor)
    .await
}

pub async fn fetch_contact_external_id<'e, E>(
    executor: E,
    contact_ent: i64,
    api_id: &str,
) -> Result<Option<String>, sqlx::Error>
where
    E: SqliteExecutor<'e>,
{
    sqlx::query_scalar!(
        r#"
        SELECT r.ZUNIQUEID AS "unique_id!: String"
        FROM ZABCDRECORD r
        WHERE r.Z_ENT = ?1
          AND lower(substr(r.ZUNIQUEID, 1, instr(r.ZUNIQUEID, ':') - 1)) = lower(?2)
        "#,
        contact_ent,
        api_id,
    )
    .fetch_optional(executor)
    .await
}

pub async fn fetch_group_external_id<'e, E>(
    executor: E,
    group_ent: i64,
    api_id: &str,
) -> Result<Option<String>, sqlx::Error>
where
    E: SqliteExecutor<'e>,
{
    sqlx::query_scalar!(
        r#"
        SELECT r.ZUNIQUEID AS "unique_id!: String"
        FROM ZABCDRECORD r
        WHERE r.Z_ENT = ?1
          AND lower(substr(r.ZUNIQUEID, 1, instr(r.ZUNIQUEID, ':') - 1)) = lower(?2)
        "#,
        group_ent,
        api_id,
    )
    .fetch_optional(executor)
    .await
}

#[allow(dead_code)]
pub async fn fetch_phones_for_contact<'e, E>(
    executor: E,
    contact_row_id: i64,
) -> Result<Vec<PhoneRow>, sqlx::Error>
where
    E: SqliteExecutor<'e>,
{
    sqlx::query_as!(
        PhoneRow,
        r#"
        SELECT
            ZUNIQUEID AS "unique_id!",
            ZFULLNUMBER AS number,
            ZLABEL AS label,
            ZISPRIMARY AS is_primary,
            ZORDERINGINDEX AS ordering_index
        FROM ZABCDPHONENUMBER
        WHERE ZOWNER = ?1
        ORDER BY ZORDERINGINDEX
        "#,
        contact_row_id,
    )
    .fetch_all(executor)
    .await
}

#[allow(dead_code)]
pub async fn fetch_emails_for_contact<'e, E>(
    executor: E,
    contact_row_id: i64,
) -> Result<Vec<EmailRow>, sqlx::Error>
where
    E: SqliteExecutor<'e>,
{
    sqlx::query_as!(
        EmailRow,
        r#"
        SELECT
            ZUNIQUEID AS "unique_id!",
            ZADDRESS AS address,
            ZLABEL AS label,
            ZISPRIMARY AS is_primary,
            ZORDERINGINDEX AS ordering_index
        FROM ZABCDEMAILADDRESS
        WHERE ZOWNER = ?1
        ORDER BY ZORDERINGINDEX
        "#,
        contact_row_id,
    )
    .fetch_all(executor)
    .await
}

#[allow(dead_code)]
pub async fn fetch_addresses_for_contact<'e, E>(
    executor: E,
    contact_row_id: i64,
) -> Result<Vec<AddressRow>, sqlx::Error>
where
    E: SqliteExecutor<'e>,
{
    sqlx::query_as!(
        AddressRow,
        r#"
        SELECT
            ZUNIQUEID AS "unique_id!",
            ZSTREET AS street,
            ZCITY AS city,
            ZSTATE AS state,
            ZZIPCODE AS postal_code,
            ZCOUNTRYNAME AS country,
            ZLABEL AS label,
            ZISPRIMARY AS is_primary,
            ZORDERINGINDEX AS ordering_index
        FROM ZABCDPOSTALADDRESS
        WHERE ZOWNER = ?1
        ORDER BY ZORDERINGINDEX
        "#,
        contact_row_id,
    )
    .fetch_all(executor)
    .await
}

#[allow(dead_code)]
pub async fn fetch_urls_for_contact<'e, E>(
    executor: E,
    contact_row_id: i64,
) -> Result<Vec<UrlRow>, sqlx::Error>
where
    E: SqliteExecutor<'e>,
{
    sqlx::query_as!(
        UrlRow,
        r#"
        SELECT
            ZUNIQUEID AS "unique_id!",
            ZURL AS url,
            ZLABEL AS label,
            ZISPRIMARY AS is_primary,
            ZORDERINGINDEX AS ordering_index
        FROM ZABCDURLADDRESS
        WHERE ZOWNER = ?1
        ORDER BY ZORDERINGINDEX
        "#,
        contact_row_id,
    )
    .fetch_all(executor)
    .await
}

#[allow(dead_code)]
pub async fn fetch_socials_for_contact<'e, E>(
    executor: E,
    contact_row_id: i64,
) -> Result<Vec<SocialRow>, sqlx::Error>
where
    E: SqliteExecutor<'e>,
{
    sqlx::query_as!(
        SocialRow,
        r#"
        SELECT
            ZUNIQUEID AS "unique_id!",
            ZSERVICENAME AS service,
            ZUSERNAME AS username,
            ZURLSTRING AS url,
            ZLABEL AS label,
            ZISPRIMARY AS is_primary,
            ZORDERINGINDEX AS ordering_index
        FROM ZABCDSOCIALPROFILE
        WHERE ZOWNER = ?1
        ORDER BY ZORDERINGINDEX
        "#,
        contact_row_id,
    )
    .fetch_all(executor)
    .await
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ContainerResolveRow {
    pub api_id: Option<String>,
    pub external_id: String,
    pub name: Option<String>,
    pub container_type: Option<i64>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct GroupResolveRow {
    pub api_id: Option<String>,
    pub name: Option<String>,
    pub group_type: Option<i64>,
    pub container_id: Option<i64>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PhoneOwnedRow {
    pub owner: i64,
    pub unique_id: String,
    pub number: Option<String>,
    pub label: Option<String>,
    pub is_primary: Option<i64>,
    pub ordering_index: Option<i64>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct EmailOwnedRow {
    pub owner: i64,
    pub unique_id: String,
    pub address: Option<String>,
    pub label: Option<String>,
    pub is_primary: Option<i64>,
    pub ordering_index: Option<i64>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AddressOwnedRow {
    pub owner: i64,
    pub unique_id: String,
    pub street: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub postal_code: Option<String>,
    pub country: Option<String>,
    pub label: Option<String>,
    pub is_primary: Option<i64>,
    pub ordering_index: Option<i64>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct UrlOwnedRow {
    pub owner: i64,
    pub unique_id: String,
    pub url: Option<String>,
    pub label: Option<String>,
    pub is_primary: Option<i64>,
    pub ordering_index: Option<i64>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SocialOwnedRow {
    pub owner: i64,
    pub unique_id: String,
    pub service: Option<String>,
    pub username: Option<String>,
    pub url: Option<String>,
    pub label: Option<String>,
    pub is_primary: Option<i64>,
    pub ordering_index: Option<i64>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct GroupOwnedRow {
    pub owner: i64,
    pub unique_id: String,
}

const CONTACT_ROW_BY_API_IDS_SQL: &str = r#"
        SELECT
            r.Z_PK AS row_id,
            r.ZUNIQUEID AS unique_id,
            r.ZFIRSTNAME AS first_name,
            r.ZLASTNAME AS last_name,
            r.ZMIDDLENAME AS middle_name,
            r.ZNICKNAME AS nickname,
            r.ZORGANIZATION AS organization,
            r.ZJOBTITLE AS job_title,
            r.ZDEPARTMENT AS department,
            r.ZNAME AS display_name,
            r.ZCONTAINER AS container_row_id,
            c.ZUNIQUEID AS container_unique_id,
            CAST(r.ZCREATIONDATE AS REAL) AS creation_date,
            CAST(r.ZMODIFICATIONDATE AS REAL) AS modification_date,
            CAST(r.ZBIRTHDAY AS REAL) AS birthday,
            n.ZTEXT AS note_text,
            CASE WHEN l.ZDATA IS NOT NULL OR r.ZIMAGEDATA IS NOT NULL THEN 1 ELSE 0 END AS has_photo
        FROM ZABCDRECORD r
        LEFT JOIN ZABCDRECORD c ON c.Z_PK = r.ZCONTAINER
        LEFT JOIN ZABCDNOTE n ON n.ZCONTACT = r.Z_PK
        LEFT JOIN ZABCDLIKENESS l ON l.ZOWNER = r.Z_PK AND l.ZISPRIMARY = 1
        WHERE r.Z_ENT = ?1
          AND lower(substr(r.ZUNIQUEID, 1, instr(r.ZUNIQUEID, ':') - 1)) IN (
            SELECT lower(value) FROM json_each(?2)
          )
"#;

const CONTACT_ROW_BY_ROW_IDS_SQL: &str = r#"
        SELECT
            r.Z_PK AS row_id,
            r.ZUNIQUEID AS unique_id,
            r.ZFIRSTNAME AS first_name,
            r.ZLASTNAME AS last_name,
            r.ZMIDDLENAME AS middle_name,
            r.ZNICKNAME AS nickname,
            r.ZORGANIZATION AS organization,
            r.ZJOBTITLE AS job_title,
            r.ZDEPARTMENT AS department,
            r.ZNAME AS display_name,
            r.ZCONTAINER AS container_row_id,
            c.ZUNIQUEID AS container_unique_id,
            CAST(r.ZCREATIONDATE AS REAL) AS creation_date,
            CAST(r.ZMODIFICATIONDATE AS REAL) AS modification_date,
            CAST(r.ZBIRTHDAY AS REAL) AS birthday,
            n.ZTEXT AS note_text,
            CASE WHEN l.ZDATA IS NOT NULL OR r.ZIMAGEDATA IS NOT NULL THEN 1 ELSE 0 END AS has_photo
        FROM ZABCDRECORD r
        LEFT JOIN ZABCDRECORD c ON c.Z_PK = r.ZCONTAINER
        LEFT JOIN ZABCDNOTE n ON n.ZCONTACT = r.Z_PK
        LEFT JOIN ZABCDLIKENESS l ON l.ZOWNER = r.Z_PK AND l.ZISPRIMARY = 1
        WHERE r.Z_ENT = ?1
          AND r.Z_PK IN (SELECT value FROM json_each(?2))
"#;

pub async fn fetch_contacts_by_api_ids<'e, E>(
    executor: E,
    contact_ent: i64,
    api_ids_json: &str,
) -> Result<Vec<ContactRow>, sqlx::Error>
where
    E: SqliteExecutor<'e>,
{
    sqlx::query_as::<_, ContactRow>(CONTACT_ROW_BY_API_IDS_SQL)
        .bind(contact_ent)
        .bind(api_ids_json)
        .fetch_all(executor)
        .await
}

pub async fn fetch_contacts_by_row_ids<'e, E>(
    executor: E,
    contact_ent: i64,
    row_ids_json: &str,
) -> Result<Vec<ContactRow>, sqlx::Error>
where
    E: SqliteExecutor<'e>,
{
    sqlx::query_as::<_, ContactRow>(CONTACT_ROW_BY_ROW_IDS_SQL)
        .bind(contact_ent)
        .bind(row_ids_json)
        .fetch_all(executor)
        .await
}

pub async fn fetch_phones_for_contact_ids<'e, E>(
    executor: E,
    row_ids_json: &str,
) -> Result<Vec<PhoneOwnedRow>, sqlx::Error>
where
    E: SqliteExecutor<'e>,
{
    record_test_query();
    sqlx::query_as!(
        PhoneOwnedRow,
        r#"
        SELECT
            ZOWNER AS "owner!",
            ZUNIQUEID AS "unique_id!",
            ZFULLNUMBER AS number,
            ZLABEL AS label,
            ZISPRIMARY AS is_primary,
            ZORDERINGINDEX AS ordering_index
        FROM ZABCDPHONENUMBER
        WHERE ZOWNER IN (SELECT value FROM json_each(?1))
        ORDER BY ZOWNER, ZORDERINGINDEX
        "#,
        row_ids_json,
    )
    .fetch_all(executor)
    .await
}

pub async fn fetch_emails_for_contact_ids<'e, E>(
    executor: E,
    row_ids_json: &str,
) -> Result<Vec<EmailOwnedRow>, sqlx::Error>
where
    E: SqliteExecutor<'e>,
{
    record_test_query();
    sqlx::query_as!(
        EmailOwnedRow,
        r#"
        SELECT
            ZOWNER AS "owner!",
            ZUNIQUEID AS "unique_id!",
            ZADDRESS AS address,
            ZLABEL AS label,
            ZISPRIMARY AS is_primary,
            ZORDERINGINDEX AS ordering_index
        FROM ZABCDEMAILADDRESS
        WHERE ZOWNER IN (SELECT value FROM json_each(?1))
        ORDER BY ZOWNER, ZORDERINGINDEX
        "#,
        row_ids_json,
    )
    .fetch_all(executor)
    .await
}

pub async fn fetch_addresses_for_contact_ids<'e, E>(
    executor: E,
    row_ids_json: &str,
) -> Result<Vec<AddressOwnedRow>, sqlx::Error>
where
    E: SqliteExecutor<'e>,
{
    record_test_query();
    sqlx::query_as!(
        AddressOwnedRow,
        r#"
        SELECT
            ZOWNER AS "owner!",
            ZUNIQUEID AS "unique_id!",
            ZSTREET AS street,
            ZCITY AS city,
            ZSTATE AS state,
            ZZIPCODE AS postal_code,
            ZCOUNTRYNAME AS country,
            ZLABEL AS label,
            ZISPRIMARY AS is_primary,
            ZORDERINGINDEX AS ordering_index
        FROM ZABCDPOSTALADDRESS
        WHERE ZOWNER IN (SELECT value FROM json_each(?1))
        ORDER BY ZOWNER, ZORDERINGINDEX
        "#,
        row_ids_json,
    )
    .fetch_all(executor)
    .await
}

pub async fn fetch_urls_for_contact_ids<'e, E>(
    executor: E,
    row_ids_json: &str,
) -> Result<Vec<UrlOwnedRow>, sqlx::Error>
where
    E: SqliteExecutor<'e>,
{
    record_test_query();
    sqlx::query_as!(
        UrlOwnedRow,
        r#"
        SELECT
            ZOWNER AS "owner!",
            ZUNIQUEID AS "unique_id!",
            ZURL AS url,
            ZLABEL AS label,
            ZISPRIMARY AS is_primary,
            ZORDERINGINDEX AS ordering_index
        FROM ZABCDURLADDRESS
        WHERE ZOWNER IN (SELECT value FROM json_each(?1))
        ORDER BY ZOWNER, ZORDERINGINDEX
        "#,
        row_ids_json,
    )
    .fetch_all(executor)
    .await
}

pub async fn fetch_socials_for_contact_ids<'e, E>(
    executor: E,
    row_ids_json: &str,
) -> Result<Vec<SocialOwnedRow>, sqlx::Error>
where
    E: SqliteExecutor<'e>,
{
    record_test_query();
    sqlx::query_as!(
        SocialOwnedRow,
        r#"
        SELECT
            ZOWNER AS "owner!",
            ZUNIQUEID AS "unique_id!",
            ZSERVICENAME AS service,
            ZUSERNAME AS username,
            ZURLSTRING AS url,
            ZLABEL AS label,
            ZISPRIMARY AS is_primary,
            ZORDERINGINDEX AS ordering_index
        FROM ZABCDSOCIALPROFILE
        WHERE ZOWNER IN (SELECT value FROM json_each(?1))
        ORDER BY ZOWNER, ZORDERINGINDEX
        "#,
        row_ids_json,
    )
    .fetch_all(executor)
    .await
}

pub async fn fetch_group_ids_for_contact_ids<'e, E>(
    executor: E,
    row_ids_json: &str,
) -> Result<Vec<GroupOwnedRow>, sqlx::Error>
where
    E: SqliteExecutor<'e>,
{
    record_test_query();
    sqlx::query_as!(
        GroupOwnedRow,
        r#"
        SELECT
            pg.Z_22CONTACTS AS "owner!",
            g.ZUNIQUEID AS "unique_id!"
        FROM Z_22PARENTGROUPS pg
        JOIN ZABCDRECORD g ON g.Z_PK = pg.Z_19PARENTGROUPS1
        WHERE pg.Z_22CONTACTS IN (SELECT value FROM json_each(?1))
        "#,
        row_ids_json,
    )
    .fetch_all(executor)
    .await
}
