//! Metadata filters for note listing.

use sqlx::{QueryBuilder, Sqlite};

use super::{entities::EntityIds, row::NoteRow};

#[allow(dead_code)]
pub const NOTE_SCAN_BUDGET: u32 = 500;

pub const CANDIDATE_CHUNK_SIZE: u32 = 100;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct NoteFilters {
    pub q: Option<String>,
    pub folder_id: Option<FolderIdFilter>,
    pub is_pinned: Option<bool>,
    pub is_locked: Option<bool>,
    pub has_checklist: Option<bool>,
    pub has_attachments: Option<bool>,
    pub include_deleted: Option<bool>,
    pub modified_before: Option<f64>,
    pub modified_after: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FolderIdFilter {
    RowId(i64),
    Uuid(String),
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct NoteFiltersSnapshot {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub q: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folder_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_pinned: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_locked: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_checklist: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_attachments: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_deleted: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified_before: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified_after: Option<f64>,
}

impl NoteFilters {
    pub fn is_active(&self) -> bool {
        self.q.is_some()
            || self.folder_id.is_some()
            || self.is_pinned.is_some()
            || self.is_locked.is_some()
            || self.has_checklist.is_some()
            || self.has_attachments.is_some()
            || self.include_deleted.is_some()
            || self.modified_before.is_some()
            || self.modified_after.is_some()
    }

    pub fn requires_text_scan(&self) -> bool {
        self.q.is_some()
    }

    pub fn snapshot(&self) -> NoteFiltersSnapshot {
        NoteFiltersSnapshot {
            q: self.q.clone(),
            folder_id: self.folder_id.as_ref().map(folder_id_filter_key),
            is_pinned: self.is_pinned,
            is_locked: self.is_locked,
            has_checklist: self.has_checklist,
            has_attachments: self.has_attachments,
            include_deleted: self.include_deleted,
            modified_before: self.modified_before,
            modified_after: self.modified_after,
        }
    }
}

fn folder_id_filter_key(filter: &FolderIdFilter) -> String {
    match filter {
        FolderIdFilter::RowId(id) => format!("row:{id}"),
        FolderIdFilter::Uuid(id) => format!("uuid:{id}"),
    }
}

pub fn apply_filters(
    builder: &mut QueryBuilder<Sqlite>,
    filters: &NoteFilters,
    entity_ids: &EntityIds,
) {
    if !filters.include_deleted.unwrap_or(false) {
        builder.push(" AND n.ZMARKEDFORDELETION = 0");
        builder.push(" AND (f.Z_PK IS NULL OR (f.ZMARKEDFORDELETION = 0 AND f.ZFOLDERTYPE != 1))");
    }

    if let Some(is_pinned) = filters.is_pinned {
        builder.push(" AND n.ZISPINNED = ");
        builder.push_bind(is_pinned as i64);
    }
    if let Some(is_locked) = filters.is_locked {
        builder.push(" AND n.ZISPASSWORDPROTECTED = ");
        builder.push_bind(is_locked as i64);
    }
    if let Some(has_checklist) = filters.has_checklist {
        builder.push(" AND n.ZHASCHECKLIST = ");
        builder.push_bind(has_checklist as i64);
    }
    if let Some(folder_id) = &filters.folder_id {
        match folder_id {
            FolderIdFilter::RowId(id) => {
                builder.push(" AND n.ZFOLDER = ");
                builder.push_bind(*id);
            }
            FolderIdFilter::Uuid(id) => {
                builder.push(" AND lower(f.ZIDENTIFIER) = ");
                builder.push_bind(id.to_lowercase());
            }
        }
    }
    if let Some(modified_before) = filters.modified_before {
        builder.push(" AND n.ZMODIFICATIONDATE1 <= ");
        builder.push_bind(modified_before);
    }
    if let Some(modified_after) = filters.modified_after {
        builder.push(" AND n.ZMODIFICATIONDATE1 >= ");
        builder.push_bind(modified_after);
    }
    if let Some(has_attachments) = filters.has_attachments {
        if has_attachments {
            builder.push(
                " AND EXISTS (SELECT 1 FROM ZICCLOUDSYNCINGOBJECT a \
                 WHERE a.ZNOTE = n.Z_PK AND a.Z_ENT = ",
            );
            builder.push_bind(entity_ids.attachment);
            builder.push(" AND a.ZMARKEDFORDELETION = 0)");
        } else {
            builder.push(
                " AND NOT EXISTS (SELECT 1 FROM ZICCLOUDSYNCINGOBJECT a \
                 WHERE a.ZNOTE = n.Z_PK AND a.Z_ENT = ",
            );
            builder.push_bind(entity_ids.attachment);
            builder.push(" AND a.ZMARKEDFORDELETION = 0)");
        }
    }
    if let Some(q) = &filters.q {
        builder.push(" AND (lower(coalesce(n.ZTITLE1, '')) LIKE '%' || lower(");
        builder.push_bind(q);
        builder.push(") || '%' OR lower(coalesce(n.ZSNIPPET, '')) LIKE '%' || lower(");
        builder.push_bind(q);
        builder.push(") || '%')");
    }
}

pub fn metadata_filters(filters: &NoteFilters) -> NoteFilters {
    NoteFilters {
        q: None,
        ..filters.clone()
    }
}

pub fn text_matches(row: &NoteRow, note_data: Option<&[u8]>, query: &str) -> bool {
    let needle = query.to_lowercase();
    if row
        .title
        .as_deref()
        .is_some_and(|value| value.to_lowercase().contains(&needle))
    {
        return true;
    }
    if row
        .snippet
        .as_deref()
        .is_some_and(|value| value.to_lowercase().contains(&needle))
    {
        return true;
    }
    if row.is_locked {
        return false;
    }
    if let Some(data) = note_data.filter(|bytes| !bytes.is_empty()) {
        let body = crate::notes::decode::decode_notedata(Some(data), false);
        if body
            .text
            .as_deref()
            .is_some_and(|text| text.to_lowercase().contains(&needle))
        {
            return true;
        }
    }
    false
}

#[allow(dead_code)]
pub fn searchable_text(row: &NoteRow) -> Option<String> {
    let title = row.title.as_deref().unwrap_or("").trim();
    let snippet = row.snippet.as_deref().unwrap_or("").trim();
    if title.is_empty() && snippet.is_empty() {
        None
    } else {
        Some(format!("{title} {snippet}").trim().to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::{NoteFilters, searchable_text};
    use crate::notes::row::NoteRow;

    #[test]
    fn searchable_text_joins_title_and_snippet() {
        let row = NoteRow {
            row_id: 1,
            id: "id".to_owned(),
            title: Some("Blog Ideas".to_owned()),
            snippet: Some("#project".to_owned()),
            created_at: None,
            modified_at: None,
            folder_row_id: Some(2),
            folder_id: Some("folder".to_owned()),
            folder_name: Some("Notes".to_owned()),
            folder_type: Some(0),
            is_pinned: false,
            has_checklist: false,
            is_locked: false,
            marked_for_deletion: false,
        };
        assert_eq!(
            searchable_text(&row).as_deref(),
            Some("Blog Ideas #project")
        );
    }

    #[test]
    fn filters_active_when_any_constraint_set() {
        assert!(!NoteFilters::default().is_active());
        assert!(
            NoteFilters {
                is_pinned: Some(true),
                ..NoteFilters::default()
            }
            .is_active()
        );
    }
}
