//! Metadata filters for note listing.

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
    Identifier(String),
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

/// Bind parameters for compile-time filtered note listing queries.
#[derive(Debug, Clone)]
pub struct NoteFilterBinds {
    pub note_ent: i64,
    pub attachment_ent: i64,
    pub include_deleted: i64,
    pub is_pinned: Option<i64>,
    pub is_locked: Option<i64>,
    pub has_checklist: Option<i64>,
    pub folder_row_id: Option<i64>,
    pub folder_identifier: Option<String>,
    pub modified_before: Option<f64>,
    pub modified_after: Option<f64>,
    pub has_attachments: Option<i64>,
    pub q: Option<String>,
    pub cursor_modified_at: Option<f64>,
    pub cursor_row_id: Option<i64>,
    pub limit: i64,
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

    pub fn bind_values(
        &self,
        entity_ids: &EntityIds,
        cursor_modified_at: Option<f64>,
        cursor_row_id: Option<i64>,
        limit: i64,
        include_q: bool,
    ) -> NoteFilterBinds {
        let (folder_row_id, folder_identifier) = match &self.folder_id {
            Some(FolderIdFilter::RowId(id)) => (Some(*id), None),
            Some(FolderIdFilter::Identifier(id)) => (None, Some(id.to_lowercase())),
            None => (None, None),
        };

        NoteFilterBinds {
            note_ent: entity_ids.note,
            attachment_ent: entity_ids.attachment,
            include_deleted: i64::from(self.include_deleted.unwrap_or(false)),
            is_pinned: self.is_pinned.map(i64::from),
            is_locked: self.is_locked.map(i64::from),
            has_checklist: self.has_checklist.map(i64::from),
            folder_row_id,
            folder_identifier,
            modified_before: self.modified_before,
            modified_after: self.modified_after,
            has_attachments: self.has_attachments.map(i64::from),
            q: if include_q { self.q.clone() } else { None },
            cursor_modified_at,
            cursor_row_id,
            limit,
        }
    }
}

fn folder_id_filter_key(filter: &FolderIdFilter) -> String {
    match filter {
        FolderIdFilter::RowId(id) => format!("row:{id}"),
        FolderIdFilter::Identifier(id) => format!("id:{id}"),
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
