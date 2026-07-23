pub const NOTE_SELECT_CORE: &str = r#"
SELECT
  n.Z_PK AS row_id,
  n.ZIDENTIFIER AS id,
  n.ZTITLE1 AS title,
  n.ZSNIPPET AS snippet,
  CAST(n.ZCREATIONDATE1 AS REAL) AS created_at,
  CAST(n.ZMODIFICATIONDATE1 AS REAL) AS modified_at,
  n.ZFOLDER AS folder_row_id,
  f.ZIDENTIFIER AS folder_id,
  f.ZTITLE2 AS folder_name,
  f.ZFOLDERTYPE AS folder_type,
  n.ZISPINNED AS is_pinned,
  n.ZHASCHECKLIST AS has_checklist,
  n.ZISPASSWORDPROTECTED AS is_locked,
  n.ZMARKEDFORDELETION AS marked_for_deletion
"#;

pub const NOTE_DETAIL_SELECT: &str = r#"
SELECT
  n.Z_PK AS row_id,
  n.ZIDENTIFIER AS id,
  n.ZTITLE1 AS title,
  n.ZSNIPPET AS snippet,
  CAST(n.ZCREATIONDATE1 AS REAL) AS created_at,
  CAST(n.ZMODIFICATIONDATE1 AS REAL) AS modified_at,
  n.ZFOLDER AS folder_row_id,
  f.ZIDENTIFIER AS folder_id,
  f.ZTITLE2 AS folder_name,
  f.ZFOLDERTYPE AS folder_type,
  n.ZISPINNED AS is_pinned,
  n.ZHASCHECKLIST AS has_checklist,
  n.ZISPASSWORDPROTECTED AS is_locked,
  n.ZMARKEDFORDELETION AS marked_for_deletion,
  nd.ZDATA AS note_data
"#;

pub const FOLDER_SELECT_CORE: &str = r#"
SELECT
  f.Z_PK AS row_id,
  f.ZIDENTIFIER AS id,
  f.ZTITLE2 AS title,
  f.ZFOLDERTYPE AS folder_type,
  f.ZPARENT AS parent_row_id,
  p.ZIDENTIFIER AS parent_id,
  f.ZACCOUNT8 AS account_row_id,
  a.ZIDENTIFIER AS account_id,
  CAST(f.ZFOLDERMODIFICATIONDATE AS REAL) AS modified_at
"#;

pub const ATTACHMENT_SELECT_CORE: &str = r#"
SELECT
  a.Z_PK AS row_id,
  a.ZIDENTIFIER AS id,
  a.ZFILENAME AS filename,
  a.ZTYPEUTI AS uti,
  a.ZNOTE AS note_row_id,
  n.ZIDENTIFIER AS note_id,
  a.ZFILESIZE AS file_size,
  CAST(a.ZMODIFICATIONDATE1 AS REAL) AS modified_at,
  acc.ZIDENTIFIER AS account_id
"#;

pub const NOTE_FROM_JOIN: &str = "
FROM ZICCLOUDSYNCINGOBJECT n
LEFT JOIN ZICCLOUDSYNCINGOBJECT f ON n.ZFOLDER = f.Z_PK
";

pub const NOTE_DETAIL_FROM_JOIN: &str = "
FROM ZICCLOUDSYNCINGOBJECT n
LEFT JOIN ZICCLOUDSYNCINGOBJECT f ON n.ZFOLDER = f.Z_PK
LEFT JOIN ZICNOTEDATA nd ON n.ZNOTEDATA = nd.Z_PK
";

pub const FOLDER_FROM: &str = "
FROM ZICCLOUDSYNCINGOBJECT f
LEFT JOIN ZICCLOUDSYNCINGOBJECT p ON f.ZPARENT = p.Z_PK
LEFT JOIN ZICCLOUDSYNCINGOBJECT a ON f.ZACCOUNT8 = a.Z_PK
";

pub const ATTACHMENT_FROM_JOIN: &str = "
FROM ZICCLOUDSYNCINGOBJECT a
JOIN ZICCLOUDSYNCINGOBJECT n ON a.ZNOTE = n.Z_PK
LEFT JOIN ZICCLOUDSYNCINGOBJECT acc ON a.ZACCOUNT6 = acc.Z_PK
";
