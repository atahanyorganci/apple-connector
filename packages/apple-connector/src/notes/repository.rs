use std::collections::{HashMap, HashSet};

use sqlx::{QueryBuilder, Sqlite, SqlitePool};

use super::{
    assembly::{attachment_from_row, folder_from_row, note_detail_from_row, note_summary_from_row},
    entities::{EntityIds, load_entity_ids},
    model::{NoteAttachment, NoteDetail, NoteFolder, NoteSummary},
    row::{AttachmentRow, FolderRow, NoteDetailRow, NoteRow},
    search::{FolderIdFilter, NoteFilters, apply_filters},
    sql::{
        ATTACHMENT_FROM_JOIN, ATTACHMENT_SELECT_CORE, FOLDER_FROM, FOLDER_SELECT_CORE,
        NOTE_DETAIL_FROM_JOIN, NOTE_DETAIL_SELECT, NOTE_FROM_JOIN, NOTE_SELECT_CORE,
    },
};
use crate::api::cursor::{
    FolderListCursor, FolderNoteCursor, GlobalNoteCursor, NoteSearchCursor, decode, encode,
};

#[derive(Debug, Clone)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub has_more: bool,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FolderLookupError {
    NotFound,
}

pub struct NoteRepository<'a> {
    pool: &'a SqlitePool,
}

impl<'a> NoteRepository<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn list_folders(
        &self,
        limit: u32,
        cursor: Option<FolderListCursor>,
        include_deleted: bool,
    ) -> Result<Page<NoteFolder>, sqlx::Error> {
        crate::db::run_timed_query(|| self.list_folders_inner(limit, cursor, include_deleted)).await
    }

    async fn list_folders_inner(
        &self,
        limit: u32,
        cursor: Option<FolderListCursor>,
        include_deleted: bool,
    ) -> Result<Page<NoteFolder>, sqlx::Error> {
        let entity_ids = load_entity_ids(self.pool).await?;
        let fetch_limit = i64::from(limit) + 1;

        let mut builder = QueryBuilder::<Sqlite>::new(FOLDER_SELECT_CORE);
        builder.push(FOLDER_FROM);
        builder.push(" WHERE f.Z_ENT = ");
        builder.push_bind(entity_ids.folder);
        builder.push(" AND f.ZMARKEDFORDELETION = 0");
        if !include_deleted {
            builder.push(" AND f.ZFOLDERTYPE != 1");
        }
        if let Some(cursor) = cursor {
            builder.push(" AND f.Z_PK < ");
            builder.push_bind(cursor.row_id);
        }
        builder.push(" ORDER BY f.Z_PK DESC LIMIT ");
        builder.push_bind(fetch_limit);

        let rows: Vec<FolderRow> = builder.build_query_as().fetch_all(self.pool).await?;
        let (rows, has_more) = split_page(rows, limit);
        let next_cursor = has_more
            .then(|| {
                rows.last()
                    .map(|row| encode(&FolderListCursor { row_id: row.row_id }).ok())
            })
            .flatten()
            .flatten();

        let items = rows.into_iter().map(folder_from_row).collect();
        Ok(Page {
            items,
            has_more,
            next_cursor,
        })
    }

    pub async fn get_folder(&self, folder_row_id: i64) -> Result<Option<NoteFolder>, sqlx::Error> {
        let entity_ids = load_entity_ids(self.pool).await?;
        let mut builder = QueryBuilder::<Sqlite>::new(FOLDER_SELECT_CORE);
        builder.push(FOLDER_FROM);
        builder.push(" WHERE f.Z_ENT = ");
        builder.push_bind(entity_ids.folder);
        builder.push(" AND f.ZMARKEDFORDELETION = 0 AND f.ZFOLDERTYPE != 1 AND f.Z_PK = ");
        builder.push_bind(folder_row_id);

        let row: Option<FolderRow> = builder.build_query_as().fetch_optional(self.pool).await?;
        Ok(row.map(folder_from_row))
    }

    pub async fn get_folder_by_id(&self, id: &str) -> Result<Option<NoteFolder>, sqlx::Error> {
        let entity_ids = load_entity_ids(self.pool).await?;
        let mut builder = QueryBuilder::<Sqlite>::new(FOLDER_SELECT_CORE);
        builder.push(FOLDER_FROM);
        builder.push(" WHERE f.Z_ENT = ");
        builder.push_bind(entity_ids.folder);
        builder.push(
            " AND f.ZMARKEDFORDELETION = 0 AND f.ZFOLDERTYPE != 1 AND lower(f.ZIDENTIFIER) = ",
        );
        builder.push_bind(id.to_lowercase());

        let row: Option<FolderRow> = builder.build_query_as().fetch_optional(self.pool).await?;
        Ok(row.map(folder_from_row))
    }

    pub async fn list_notes_in_folder(
        &self,
        folder_row_id: i64,
        filters: &NoteFilters,
        limit: u32,
        cursor: Option<FolderNoteCursor>,
    ) -> Result<Result<Page<NoteSummary>, FolderLookupError>, sqlx::Error> {
        if self.get_folder(folder_row_id).await?.is_none() {
            return Ok(Err(FolderLookupError::NotFound));
        }

        let mut scoped_filters = filters.clone();
        scoped_filters.folder_id = Some(FolderIdFilter::RowId(folder_row_id));
        let global_cursor = cursor.map(|value| GlobalNoteCursor {
            modified_at: value.modified_at,
            row_id: value.row_id,
        });
        let page = self
            .list_notes_inner(&scoped_filters, limit, global_cursor)
            .await?;

        let next_cursor = if scoped_filters.is_active() {
            page.next_cursor
        } else {
            page.next_cursor.and_then(|cursor| {
                decode::<GlobalNoteCursor>(&cursor)
                    .ok()
                    .and_then(|decoded| {
                        encode(&FolderNoteCursor {
                            modified_at: decoded.modified_at,
                            row_id: decoded.row_id,
                        })
                        .ok()
                    })
            })
        };

        Ok(Ok(Page {
            items: page.items,
            has_more: page.has_more,
            next_cursor,
        }))
    }

    pub async fn list_notes(
        &self,
        filters: &NoteFilters,
        limit: u32,
        cursor: Option<GlobalNoteCursor>,
    ) -> Result<Page<NoteSummary>, sqlx::Error> {
        crate::db::run_timed_query(|| self.list_notes_inner(filters, limit, cursor)).await
    }

    async fn list_notes_inner(
        &self,
        filters: &NoteFilters,
        limit: u32,
        cursor: Option<GlobalNoteCursor>,
    ) -> Result<Page<NoteSummary>, sqlx::Error> {
        let entity_ids = load_entity_ids(self.pool).await?;
        let fetch_limit = i64::from(limit) + 1;

        let mut builder = QueryBuilder::<Sqlite>::new(NOTE_SELECT_CORE);
        builder.push(NOTE_FROM_JOIN);
        builder.push(" WHERE n.Z_ENT = ");
        builder.push_bind(entity_ids.note);
        apply_filters(&mut builder, filters, &entity_ids);
        if let Some(cursor) = cursor {
            builder.push(
                " AND (n.ZMODIFICATIONDATE1 < ? OR (n.ZMODIFICATIONDATE1 = ? AND n.Z_PK < ?))",
            );
            builder.push_bind(cursor.modified_at);
            builder.push_bind(cursor.modified_at);
            builder.push_bind(cursor.row_id);
        }
        builder.push(" ORDER BY n.ZMODIFICATIONDATE1 DESC, n.Z_PK DESC LIMIT ");
        builder.push_bind(fetch_limit);

        let rows: Vec<NoteRow> = builder.build_query_as().fetch_all(self.pool).await?;
        let (rows, has_more) = split_page(rows, limit);
        let attachment_flags = self
            .attachment_flags_for_notes(rows.iter().map(|row| row.row_id).collect(), &entity_ids)
            .await?;

        let use_search_cursor = filters.is_active();
        let next_cursor = has_more
            .then(|| {
                rows.last().map(|row| {
                    let modified_at = row.modified_at.unwrap_or(0.0);
                    if use_search_cursor {
                        encode(&NoteSearchCursor {
                            modified_at,
                            row_id: row.row_id,
                            filters: filters.snapshot(),
                        })
                        .ok()
                    } else {
                        encode(&GlobalNoteCursor {
                            modified_at,
                            row_id: row.row_id,
                        })
                        .ok()
                    }
                })
            })
            .flatten()
            .flatten();

        let items = rows
            .into_iter()
            .map(|row| {
                let has_attachments = attachment_flags.get(&row.row_id).copied().unwrap_or(false);
                note_summary_from_row(row, has_attachments)
            })
            .collect();

        Ok(Page {
            items,
            has_more,
            next_cursor,
        })
    }

    pub async fn get_note(&self, id: &str) -> Result<Option<NoteDetail>, sqlx::Error> {
        let entity_ids = load_entity_ids(self.pool).await?;
        let mut builder = QueryBuilder::<Sqlite>::new(NOTE_DETAIL_SELECT);
        builder.push(NOTE_DETAIL_FROM_JOIN);
        builder.push(" WHERE n.Z_ENT = ");
        builder.push_bind(entity_ids.note);
        builder.push(" AND n.ZMARKEDFORDELETION = 0");
        builder.push(" AND (f.Z_PK IS NULL OR (f.ZMARKEDFORDELETION = 0 AND f.ZFOLDERTYPE != 1))");
        builder.push(" AND lower(n.ZIDENTIFIER) = ");
        builder.push_bind(id.to_lowercase());

        let row: Option<NoteDetailRow> = builder.build_query_as().fetch_optional(self.pool).await?;
        let Some(row) = row else {
            return Ok(None);
        };

        let has_attachments = self
            .attachment_flags_for_notes(vec![row.row_id], &entity_ids)
            .await?
            .get(&row.row_id)
            .copied()
            .unwrap_or(false);

        Ok(Some(note_detail_from_row(row, has_attachments)))
    }

    pub async fn search_notes(
        &self,
        filters: &NoteFilters,
        limit: u32,
        cursor: Option<NoteSearchCursor>,
    ) -> Result<Page<NoteSummary>, sqlx::Error> {
        use super::search::{
            CANDIDATE_CHUNK_SIZE, NOTE_SCAN_BUDGET, metadata_filters, text_matches,
        };

        let query = filters.q.as_deref().expect("search requires q");
        let sql_filters = metadata_filters(filters);
        let entity_ids = load_entity_ids(self.pool).await?;
        let mut matching_rows = Vec::new();
        let mut scanned = 0_u32;
        let mut scan_position = cursor.map(|value| (value.modified_at, value.row_id));
        let mut reached_end = false;

        'search: while scanned < NOTE_SCAN_BUDGET {
            let mut builder = QueryBuilder::<Sqlite>::new(NOTE_DETAIL_SELECT);
            builder.push(NOTE_DETAIL_FROM_JOIN);
            builder.push(" WHERE n.Z_ENT = ");
            builder.push_bind(entity_ids.note);
            apply_filters(&mut builder, &sql_filters, &entity_ids);
            if let Some((modified_at, row_id)) = scan_position {
                builder.push(
                    " AND (n.ZMODIFICATIONDATE1 < ? OR (n.ZMODIFICATIONDATE1 = ? AND n.Z_PK < ?))",
                );
                builder.push_bind(modified_at);
                builder.push_bind(modified_at);
                builder.push_bind(row_id);
            }
            builder.push(" ORDER BY n.ZMODIFICATIONDATE1 DESC, n.Z_PK DESC LIMIT ");
            builder.push_bind(i64::from(CANDIDATE_CHUNK_SIZE));

            let chunk: Vec<NoteDetailRow> = builder.build_query_as().fetch_all(self.pool).await?;
            if chunk.is_empty() {
                reached_end = true;
                break;
            }

            let chunk_len = chunk.len();
            for row in chunk {
                scanned += 1;
                scan_position = Some((row.modified_at.unwrap_or(0.0), row.row_id));

                let note_row = NoteRow {
                    row_id: row.row_id,
                    id: row.id.clone(),
                    title: row.title.clone(),
                    snippet: row.snippet.clone(),
                    created_at: row.created_at,
                    modified_at: row.modified_at,
                    folder_row_id: row.folder_row_id,
                    folder_id: row.folder_id.clone(),
                    folder_name: row.folder_name.clone(),
                    folder_type: row.folder_type,
                    is_pinned: row.is_pinned,
                    has_checklist: row.has_checklist,
                    is_locked: row.is_locked,
                    marked_for_deletion: row.marked_for_deletion,
                };

                if text_matches(&note_row, row.note_data.as_deref(), query) {
                    matching_rows.push(row);
                    if matching_rows.len() > limit as usize {
                        break 'search;
                    }
                }

                if scanned >= NOTE_SCAN_BUDGET {
                    break;
                }
            }

            if chunk_len < CANDIDATE_CHUNK_SIZE as usize {
                reached_end = true;
                break;
            }
        }

        let has_more =
            matching_rows.len() > limit as usize || (!reached_end && scanned >= NOTE_SCAN_BUDGET);
        if matching_rows.len() > limit as usize {
            matching_rows.truncate(limit as usize);
        }

        let note_row_ids: Vec<i64> = matching_rows.iter().map(|row| row.row_id).collect();
        let attachment_flags = self
            .attachment_flags_for_notes(note_row_ids.clone(), &entity_ids)
            .await?;

        let items = matching_rows
            .into_iter()
            .map(|row| {
                let summary = NoteRow {
                    row_id: row.row_id,
                    id: row.id,
                    title: row.title,
                    snippet: row.snippet,
                    created_at: row.created_at,
                    modified_at: row.modified_at,
                    folder_row_id: row.folder_row_id,
                    folder_id: row.folder_id,
                    folder_name: row.folder_name,
                    folder_type: row.folder_type,
                    is_pinned: row.is_pinned,
                    has_checklist: row.has_checklist,
                    is_locked: row.is_locked,
                    marked_for_deletion: row.marked_for_deletion,
                };
                let has_attachments = attachment_flags
                    .get(&summary.row_id)
                    .copied()
                    .unwrap_or(false);
                note_summary_from_row(summary, has_attachments)
            })
            .collect();

        let next_cursor = has_more
            .then(|| {
                scan_position.and_then(|(modified_at, row_id)| {
                    encode(&NoteSearchCursor {
                        modified_at,
                        row_id,
                        filters: filters.snapshot(),
                    })
                    .ok()
                })
            })
            .flatten();

        Ok(Page {
            items,
            has_more,
            next_cursor,
        })
    }

    pub async fn list_attachments_for_note(
        &self,
        note_row_id: i64,
    ) -> Result<Vec<NoteAttachment>, sqlx::Error> {
        let entity_ids = load_entity_ids(self.pool).await?;
        let mut builder = QueryBuilder::<Sqlite>::new(ATTACHMENT_SELECT_CORE);
        builder.push(ATTACHMENT_FROM_JOIN);
        builder.push(" WHERE a.Z_ENT = ");
        builder.push_bind(entity_ids.attachment);
        builder.push(" AND a.ZMARKEDFORDELETION = 0 AND a.ZNOTE = ");
        builder.push_bind(note_row_id);
        builder.push(" ORDER BY a.Z_PK ASC");

        let rows: Vec<AttachmentRow> = builder.build_query_as().fetch_all(self.pool).await?;
        Ok(rows.into_iter().map(attachment_from_row).collect())
    }

    pub async fn get_attachment_by_id(
        &self,
        id: &str,
    ) -> Result<Option<NoteAttachment>, sqlx::Error> {
        let entity_ids = load_entity_ids(self.pool).await?;
        let mut builder = QueryBuilder::<Sqlite>::new(ATTACHMENT_SELECT_CORE);
        builder.push(ATTACHMENT_FROM_JOIN);
        builder.push(" WHERE a.Z_ENT = ");
        builder.push_bind(entity_ids.attachment);
        builder.push(" AND a.ZMARKEDFORDELETION = 0 AND lower(a.ZIDENTIFIER) = ");
        builder.push_bind(id.to_lowercase());

        let row: Option<AttachmentRow> = builder.build_query_as().fetch_optional(self.pool).await?;
        Ok(row.map(attachment_from_row))
    }

    async fn attachment_flags_for_notes(
        &self,
        note_row_ids: Vec<i64>,
        entity_ids: &EntityIds,
    ) -> Result<HashMap<i64, bool>, sqlx::Error> {
        if note_row_ids.is_empty() || entity_ids.attachment == 0 {
            return Ok(HashMap::new());
        }

        let mut builder = QueryBuilder::<Sqlite>::new(
            "SELECT a.ZNOTE AS note_row_id FROM ZICCLOUDSYNCINGOBJECT a \
             WHERE a.Z_ENT = ",
        );
        builder.push_bind(entity_ids.attachment);
        builder.push(" AND a.ZMARKEDFORDELETION = 0 AND a.ZNOTE IN (");
        {
            let mut separated = builder.separated(", ");
            for id in &note_row_ids {
                separated.push_bind(id);
            }
        }
        builder.push(")");

        let rows: Vec<(i64,)> = builder.build_query_as().fetch_all(self.pool).await?;
        let attached: HashSet<i64> = rows.into_iter().map(|row| row.0).collect();
        Ok(note_row_ids
            .into_iter()
            .map(|row_id| (row_id, attached.contains(&row_id)))
            .collect())
    }
}

fn split_page<T>(mut rows: Vec<T>, limit: u32) -> (Vec<T>, bool) {
    let has_more = rows.len() > limit as usize;
    if has_more {
        rows.truncate(limit as usize);
    }
    (rows, has_more)
}
