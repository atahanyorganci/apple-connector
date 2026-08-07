//! Metadata filters and bounded text search for reminder listing.

use super::row::ReminderRow;
use crate::sqlx_util::optional_bool_filter;

#[allow(dead_code)]
pub const REMINDER_SCAN_BUDGET: u32 = 500;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ReminderFilters {
    pub q: Option<String>,
    pub completed: Option<bool>,
    pub flagged: Option<bool>,
    pub list_id: Option<ListIdFilter>,
    pub has_due_date: Option<bool>,
    pub due_before: Option<i64>,
    pub due_after: Option<i64>,
    pub priority_min: Option<i32>,
    pub has_notes: Option<bool>,
    pub top_level_only: Option<bool>,
}

/// Bind parameters for the compile-time filtered reminder listing query.
#[derive(Debug, Clone)]
pub struct ReminderFilterBinds {
    pub completed: Option<i64>,
    pub flagged: Option<i64>,
    pub list_row_id: Option<i64>,
    pub list_uuid: Option<String>,
    pub has_due_date: Option<i64>,
    pub due_before: Option<f64>,
    pub due_after: Option<f64>,
    pub priority_min: Option<i64>,
    pub has_notes: Option<i64>,
    pub top_level_only: Option<i64>,
    pub q: Option<String>,
    pub cursor_modified_at: Option<f64>,
    pub cursor_row_id: Option<i64>,
    pub limit: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ListIdFilter {
    RowId(i64),
    Uuid(String),
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ReminderFiltersSnapshot {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub q: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flagged: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_due_date: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due_before: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due_after: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority_min: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_notes: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_level_only: Option<bool>,
}

impl ReminderFilters {
    pub fn is_active(&self) -> bool {
        self.q.is_some()
            || self.completed.is_some()
            || self.flagged.is_some()
            || self.list_id.is_some()
            || self.has_due_date.is_some()
            || self.due_before.is_some()
            || self.due_after.is_some()
            || self.priority_min.is_some()
            || self.has_notes.is_some()
            || self.top_level_only.is_some()
    }

    pub fn requires_text_scan(&self) -> bool {
        self.q.is_some()
    }

    pub fn snapshot(&self) -> ReminderFiltersSnapshot {
        ReminderFiltersSnapshot {
            q: self.q.clone(),
            completed: self.completed,
            flagged: self.flagged,
            list_id: self.list_id.as_ref().map(list_id_filter_key),
            has_due_date: self.has_due_date,
            due_before: self.due_before,
            due_after: self.due_after,
            priority_min: self.priority_min,
            has_notes: self.has_notes,
            top_level_only: self.top_level_only,
        }
    }

    pub fn bind_values(
        &self,
        cursor_modified_at: Option<f64>,
        cursor_row_id: Option<i64>,
        limit: i64,
    ) -> ReminderFilterBinds {
        let (list_row_id, list_uuid) = match &self.list_id {
            Some(ListIdFilter::RowId(id)) => (Some(*id), None),
            Some(ListIdFilter::Uuid(id)) => (None, Some(id.to_lowercase())),
            None => (None, None),
        };

        ReminderFilterBinds {
            completed: optional_bool_filter(self.completed),
            flagged: optional_bool_filter(self.flagged),
            list_row_id,
            list_uuid,
            has_due_date: optional_bool_filter(self.has_due_date),
            due_before: self.due_before.map(|value| value as f64),
            due_after: self.due_after.map(|value| value as f64),
            priority_min: self.priority_min.map(i64::from),
            has_notes: optional_bool_filter(self.has_notes),
            top_level_only: self.top_level_only.filter(|&value| value).map(|_| 1),
            q: self.q.clone(),
            cursor_modified_at,
            cursor_row_id,
            limit,
        }
    }
}

fn list_id_filter_key(filter: &ListIdFilter) -> String {
    match filter {
        ListIdFilter::RowId(id) => format!("row:{id}"),
        ListIdFilter::Uuid(id) => format!("uuid:{id}"),
    }
}

#[allow(dead_code)]
pub fn searchable_text(row: &ReminderRow) -> Option<String> {
    let title = row.title.as_deref().unwrap_or("").trim();
    let notes = row.notes.as_deref().unwrap_or("").trim();
    if title.is_empty() && notes.is_empty() {
        None
    } else {
        Some(format!("{title} {notes}").trim().to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::{ReminderFilters, searchable_text};
    use crate::reminders::row::ReminderRow;

    #[test]
    fn searchable_text_joins_title_and_notes() {
        let row = ReminderRow {
            row_id: 1,
            id: "id".to_owned(),
            title: Some("Buy milk".to_owned()),
            notes: Some("2%".to_owned()),
            completed: false,
            flagged: false,
            priority: 0,
            all_day: false,
            list_row_id: 1,
            list_id: "list".to_owned(),
            list_name: Some("Groceries".to_owned()),
            parent_row_id: None,
            parent_id: None,
            display_order: 0,
            due_date: None,
            completion_date: None,
            creation_date: None,
            last_modified_date: None,
            list_ent: 0,
            list_smart_type: None,
            list_sharing_status: None,
            list_shared_owner_name: None,
            list_shared_owner_address: None,
            list_filter_data: None,
            list_membership_data: None,
        };
        assert_eq!(searchable_text(&row).as_deref(), Some("Buy milk 2%"));
    }

    #[test]
    fn filters_active_when_any_constraint_set() {
        assert!(!ReminderFilters::default().is_active());
        assert!(
            ReminderFilters {
                completed: Some(true),
                ..ReminderFilters::default()
            }
            .is_active()
        );
    }
}
