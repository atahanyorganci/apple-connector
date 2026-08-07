use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use sqlx::SqlitePool;

use super::{
    assembly::{attachment_from_row, folder_from_row, note_detail_from_row, note_summary_from_row},
    entities::{EntityIds, load_entity_ids},
    model::{NoteAttachment, NoteDetail, NoteFolder, NoteSummary},
    queries::{
        fetch_filtered_note_details, fetch_filtered_notes, fetch_note_row_ids_with_attachments,
        fetch_tags_for_note, get_attachment_by_identifier, get_folder_by_identifier,
        get_folder_by_row_id, get_note_by_identifier, list_attachments_for_note, list_folders,
    },
    row::{NoteDetailRow, NoteRow},
    search::{FolderIdFilter, NoteFilters},
};
use crate::{
    api::cursor::{
        FolderListCursor, FolderNoteCursor, GlobalNoteCursor, NoteSearchCursor, decode, encode,
    },
    sqlx_util::json_ids,
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
    entity_ids: Option<Arc<EntityIds>>,
}

impl<'a> NoteRepository<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self {
            pool,
            entity_ids: None,
        }
    }

    pub fn with_entity_ids(pool: &'a SqlitePool, entity_ids: Arc<EntityIds>) -> Self {
        Self {
            pool,
            entity_ids: Some(entity_ids),
        }
    }

    async fn entity_ids(&self) -> Result<Arc<EntityIds>, sqlx::Error> {
        if let Some(entity_ids) = &self.entity_ids {
            return Ok(Arc::clone(entity_ids));
        }
        load_entity_ids(self.pool).await.map(Arc::new)
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
        let entity_ids = self.entity_ids().await?;
        let fetch_limit = i64::from(limit) + 1;

        let rows = list_folders(
            self.pool,
            entity_ids.folder,
            i64::from(include_deleted),
            cursor.map(|value| value.row_id),
            fetch_limit,
        )
        .await?;

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
        crate::db::run_timed_query(|| self.get_folder_inner(folder_row_id)).await
    }

    async fn get_folder_inner(
        &self,
        folder_row_id: i64,
    ) -> Result<Option<NoteFolder>, sqlx::Error> {
        let entity_ids = self.entity_ids().await?;
        let row = get_folder_by_row_id(self.pool, entity_ids.folder, folder_row_id).await?;
        Ok(row.map(folder_from_row))
    }

    pub async fn get_folder_by_id(&self, id: &str) -> Result<Option<NoteFolder>, sqlx::Error> {
        crate::db::run_timed_query(|| self.get_folder_by_id_inner(id)).await
    }

    async fn get_folder_by_id_inner(&self, id: &str) -> Result<Option<NoteFolder>, sqlx::Error> {
        let entity_ids = self.entity_ids().await?;
        let row =
            get_folder_by_identifier(self.pool, entity_ids.folder, &id.to_lowercase()).await?;
        Ok(row.map(folder_from_row))
    }

    pub async fn list_notes_in_folder(
        &self,
        folder_row_id: i64,
        filters: &NoteFilters,
        limit: u32,
        cursor: Option<FolderNoteCursor>,
    ) -> Result<Result<Page<NoteSummary>, FolderLookupError>, sqlx::Error> {
        crate::db::run_timed_query(|| {
            self.list_notes_in_folder_inner(folder_row_id, filters, limit, cursor)
        })
        .await
    }

    async fn list_notes_in_folder_inner(
        &self,
        folder_row_id: i64,
        filters: &NoteFilters,
        limit: u32,
        cursor: Option<FolderNoteCursor>,
    ) -> Result<Result<Page<NoteSummary>, FolderLookupError>, sqlx::Error> {
        if self.get_folder_inner(folder_row_id).await?.is_none() {
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
        let entity_ids = self.entity_ids().await?;
        let fetch_limit = i64::from(limit) + 1;
        let binds = filters.bind_values(
            &entity_ids,
            cursor.as_ref().map(|value| value.modified_at),
            cursor.as_ref().map(|value| value.row_id),
            fetch_limit,
            true,
        );

        let rows = fetch_filtered_notes(self.pool, &binds).await?;
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
        crate::db::run_timed_query(|| self.get_note_inner(id)).await
    }

    async fn get_note_inner(&self, id: &str) -> Result<Option<NoteDetail>, sqlx::Error> {
        let entity_ids = self.entity_ids().await?;
        let row = get_note_by_identifier(self.pool, entity_ids.note, &id.to_lowercase()).await?;
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

        let Some(query) = filters.q.as_deref() else {
            return Ok(Page {
                items: Vec::new(),
                has_more: false,
                next_cursor: None,
            });
        };
        let sql_filters = metadata_filters(filters);
        let entity_ids = self.entity_ids().await?;
        let mut matching_rows = Vec::new();
        let mut scanned = 0_u32;
        let mut scan_position = cursor.map(|value| (value.modified_at, value.row_id));
        let mut reached_end = false;

        'search: while scanned < NOTE_SCAN_BUDGET {
            let binds = sql_filters.bind_values(
                &entity_ids,
                scan_position.map(|value| value.0),
                scan_position.map(|value| value.1),
                i64::from(CANDIDATE_CHUNK_SIZE),
                false,
            );

            let chunk: Vec<NoteDetailRow> = fetch_filtered_note_details(self.pool, &binds).await?;
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

    pub async fn fetch_tags_for_note(&self, note_row_id: i64) -> Result<Vec<String>, sqlx::Error> {
        let rows = fetch_tags_for_note(self.pool, note_row_id).await?;

        let mut tags = Vec::new();
        let mut seen = HashSet::new();
        for (alt_text,) in rows {
            let Some(alt_text) = alt_text else {
                continue;
            };
            let tag = normalize_hashtag_label(&alt_text);
            if tag.is_empty() {
                continue;
            }
            if seen.insert(tag.to_lowercase()) {
                tags.push(tag);
            }
        }
        Ok(tags)
    }

    pub async fn list_attachments_for_note(
        &self,
        note_row_id: i64,
    ) -> Result<Vec<NoteAttachment>, sqlx::Error> {
        let entity_ids = self.entity_ids().await?;
        let rows = list_attachments_for_note(self.pool, entity_ids.attachment, note_row_id).await?;
        Ok(rows.into_iter().map(attachment_from_row).collect())
    }

    pub async fn get_attachment_by_id(
        &self,
        id: &str,
    ) -> Result<Option<NoteAttachment>, sqlx::Error> {
        let entity_ids = self.entity_ids().await?;
        let row =
            get_attachment_by_identifier(self.pool, entity_ids.attachment, &id.to_lowercase())
                .await?;
        Ok(row.map(attachment_from_row))
    }

    async fn attachment_flags_for_notes(
        &self,
        note_row_ids: Vec<i64>,
        entity_ids: &EntityIds,
    ) -> Result<HashMap<i64, bool>, sqlx::Error> {
        if note_row_ids.is_empty() {
            return Ok(HashMap::new());
        }

        let rows = fetch_note_row_ids_with_attachments(
            self.pool,
            entity_ids.attachment,
            &json_ids(&note_row_ids),
        )
        .await?;
        let attached: HashSet<i64> = rows.into_iter().map(|row| row.note_row_id).collect();
        Ok(note_row_ids
            .into_iter()
            .map(|row_id| (row_id, attached.contains(&row_id)))
            .collect())
    }
}

fn normalize_hashtag_label(alt_text: &str) -> String {
    alt_text
        .strip_prefix('#')
        .unwrap_or(alt_text)
        .trim()
        .to_owned()
}

fn split_page<T>(mut rows: Vec<T>, limit: u32) -> (Vec<T>, bool) {
    let has_more = rows.len() > limit as usize;
    if has_more {
        rows.truncate(limit as usize);
    }
    (rows, has_more)
}
