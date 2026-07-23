use super::row::{ATTACHMENT_UUID_SQL, LIST_UUID_SQL, SECTION_UUID_SQL, UUID_SQL};

pub const REMINDER_SELECT_CORE: &str = r#"
SELECT
  r.Z_PK AS row_id,
  lower(
    substr(hex(r.ZIDENTIFIER), 1, 8) || '-' ||
    substr(hex(r.ZIDENTIFIER), 9, 4) || '-' ||
    substr(hex(r.ZIDENTIFIER), 13, 4) || '-' ||
    substr(hex(r.ZIDENTIFIER), 17, 4) || '-' ||
    substr(hex(r.ZIDENTIFIER), 21, 12)
  ) AS id,
  r.ZTITLE AS title,
  r.ZNOTES AS notes,
  r.ZCOMPLETED AS completed,
  r.ZFLAGGED AS flagged,
  r.ZPRIORITY AS priority,
  r.ZALLDAY AS all_day,
  r.ZLIST AS list_row_id,
  lower(
    substr(hex(l.ZIDENTIFIER), 1, 8) || '-' ||
    substr(hex(l.ZIDENTIFIER), 9, 4) || '-' ||
    substr(hex(l.ZIDENTIFIER), 13, 4) || '-' ||
    substr(hex(l.ZIDENTIFIER), 17, 4) || '-' ||
    substr(hex(l.ZIDENTIFIER), 21, 12)
  ) AS list_id,
  l.ZNAME AS list_name,
  r.ZPARENTREMINDER AS parent_row_id,
  CASE
    WHEN p.ZIDENTIFIER IS NULL THEN NULL
    ELSE lower(
      substr(hex(p.ZIDENTIFIER), 1, 8) || '-' ||
      substr(hex(p.ZIDENTIFIER), 9, 4) || '-' ||
      substr(hex(p.ZIDENTIFIER), 13, 4) || '-' ||
      substr(hex(p.ZIDENTIFIER), 17, 4) || '-' ||
      substr(hex(p.ZIDENTIFIER), 21, 12)
    )
  END AS parent_id,
  r.ZICSDISPLAYORDER AS display_order,
  CAST(r.ZDUEDATE AS REAL) AS due_date,
  CAST(r.ZCOMPLETIONDATE AS REAL) AS completion_date,
  CAST(r.ZCREATIONDATE AS REAL) AS creation_date,
  CAST(r.ZLASTMODIFIEDDATE AS REAL) AS last_modified_date,
  l.Z_ENT AS list_ent,
  l.ZSMARTLISTTYPE AS list_smart_type,
  l.ZSHARINGSTATUS AS list_sharing_status,
  l.ZSHAREDOWNERNAME AS list_shared_owner_name,
  l.ZSHAREDOWNERADDRESS AS list_shared_owner_address,
  l.ZFILTERDATA AS list_filter_data,
  l.ZMEMBERSHIPSOFREMINDERSINSECTIONSASDATA AS list_membership_data
"#;

pub const LIST_SELECT_CORE: &str = r#"
SELECT
  l.Z_PK AS row_id,
  lower(
    substr(hex(l.ZIDENTIFIER), 1, 8) || '-' ||
    substr(hex(l.ZIDENTIFIER), 9, 4) || '-' ||
    substr(hex(l.ZIDENTIFIER), 13, 4) || '-' ||
    substr(hex(l.ZIDENTIFIER), 17, 4) || '-' ||
    substr(hex(l.ZIDENTIFIER), 21, 12)
  ) AS id,
  l.ZNAME AS name,
  l.Z_ENT AS ent,
  l.ZSMARTLISTTYPE AS smart_list_type,
  l.ZSHARINGSTATUS AS sharing_status,
  l.ZSHAREDOWNERNAME AS shared_owner_name,
  l.ZSHAREDOWNERADDRESS AS shared_owner_address,
  l.ZFILTERDATA AS filter_data,
  l.ZMEMBERSHIPSOFREMINDERSINSECTIONSASDATA AS membership_data,
  NULL AS last_modified_date
"#;

pub const SECTION_SELECT_CORE: &str = r#"
SELECT
  s.Z_PK AS row_id,
  lower(
    substr(hex(s.ZIDENTIFIER), 1, 8) || '-' ||
    substr(hex(s.ZIDENTIFIER), 9, 4) || '-' ||
    substr(hex(s.ZIDENTIFIER), 13, 4) || '-' ||
    substr(hex(s.ZIDENTIFIER), 17, 4) || '-' ||
    substr(hex(s.ZIDENTIFIER), 21, 12)
  ) AS id,
  s.ZDISPLAYNAME AS display_name,
  s.ZCANONICALNAME AS canonical_name,
  s.ZLIST AS list_row_id
"#;

pub const ATTACHMENT_SELECT_CORE: &str = r#"
SELECT
  sa.Z_PK AS row_id,
  lower(
    substr(hex(sa.ZIDENTIFIER), 1, 8) || '-' ||
    substr(hex(sa.ZIDENTIFIER), 9, 4) || '-' ||
    substr(hex(sa.ZIDENTIFIER), 13, 4) || '-' ||
    substr(hex(sa.ZIDENTIFIER), 17, 4) || '-' ||
    substr(hex(sa.ZIDENTIFIER), 21, 12)
  ) AS id,
  sa.ZFILENAME AS filename,
  sa.ZUTI AS uti,
  sa.ZSHA512SUM AS sha512,
  sa.ZATTACHMENTTYPERAWVALUE AS kind_raw,
  sa.ZREMINDER AS reminder_row_id,
  CAST(sa.ZLASTMODIFIEDDATE AS REAL) AS modified_at
"#;

pub const REMINDER_FROM_JOIN: &str = "
FROM ZREMCDREMINDER r
JOIN ZREMCDBASELIST l ON r.ZLIST = l.Z_PK
LEFT JOIN ZREMCDREMINDER p ON r.ZPARENTREMINDER = p.Z_PK
WHERE r.ZMARKEDFORDELETION = 0
  AND l.ZMARKEDFORDELETION = 0
";

pub const LIST_FROM: &str = "
FROM ZREMCDBASELIST l
WHERE l.ZMARKEDFORDELETION = 0
";

// Silence unused import warnings for exported SQL fragments used elsewhere.
const _: &str = UUID_SQL;
const _: &str = LIST_UUID_SQL;
const _: &str = SECTION_UUID_SQL;
const _: &str = ATTACHMENT_UUID_SQL;
