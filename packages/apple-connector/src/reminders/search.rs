//! Metadata filters and bounded text search for reminder listing.

use sqlx::{QueryBuilder, Sqlite};

use super::row::ReminderRow;

#[allow(dead_code)]
pub const REMINDER_SCAN_BUDGET: u32 = 500;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ReminderFilters {
    pub q: Option<String>,
    pub completed: Option<bool>,
    pub flagged: Option<bool>,
    pub list_id: Option<ListIdFilter>,
    pub section_id: Option<String>,
    pub has_due_date: Option<bool>,
    pub due_before: Option<i64>,
    pub due_after: Option<i64>,
    pub priority_min: Option<i32>,
    pub has_notes: Option<bool>,
    pub top_level_only: Option<bool>,
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
    pub section_id: Option<String>,
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
            || self.section_id.is_some()
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
            section_id: self.section_id.clone(),
            has_due_date: self.has_due_date,
            due_before: self.due_before,
            due_after: self.due_after,
            priority_min: self.priority_min,
            has_notes: self.has_notes,
            top_level_only: self.top_level_only,
        }
    }
}

fn list_id_filter_key(filter: &ListIdFilter) -> String {
    match filter {
        ListIdFilter::RowId(id) => format!("row:{id}"),
        ListIdFilter::Uuid(id) => format!("uuid:{id}"),
    }
}

pub fn apply_filters(builder: &mut QueryBuilder<Sqlite>, filters: &ReminderFilters) {
    if let Some(completed) = filters.completed {
        builder.push(" AND r.ZCOMPLETED = ");
        builder.push_bind(completed as i64);
    }
    if let Some(flagged) = filters.flagged {
        builder.push(" AND r.ZFLAGGED = ");
        builder.push_bind(flagged as i64);
    }
    if let Some(list_id) = &filters.list_id {
        match list_id {
            ListIdFilter::RowId(id) => {
                builder.push(" AND r.ZLIST = ");
                builder.push_bind(*id);
            }
            ListIdFilter::Uuid(id) => {
                builder.push(" AND lower(substr(hex(l.ZIDENTIFIER), 1, 8) || '-' || substr(hex(l.ZIDENTIFIER), 9, 4) || '-' || substr(hex(l.ZIDENTIFIER), 13, 4) || '-' || substr(hex(l.ZIDENTIFIER), 17, 4) || '-' || substr(hex(l.ZIDENTIFIER), 21, 12)) = ");
                builder.push_bind(id.to_lowercase());
            }
        }
    }
    if let Some(has_due_date) = filters.has_due_date {
        if has_due_date {
            builder.push(" AND r.ZDUEDATE IS NOT NULL");
        } else {
            builder.push(" AND r.ZDUEDATE IS NULL");
        }
    }
    if let Some(due_before) = filters.due_before {
        builder.push(" AND r.ZDUEDATE <= ");
        builder.push_bind(due_before);
    }
    if let Some(due_after) = filters.due_after {
        builder.push(" AND r.ZDUEDATE >= ");
        builder.push_bind(due_after);
    }
    if let Some(priority_min) = filters.priority_min {
        builder.push(" AND r.ZPRIORITY >= ");
        builder.push_bind(i64::from(priority_min));
    }
    if let Some(has_notes) = filters.has_notes {
        if has_notes {
            builder.push(" AND r.ZNOTES IS NOT NULL AND trim(r.ZNOTES) != ''");
        } else {
            builder.push(" AND (r.ZNOTES IS NULL OR trim(r.ZNOTES) = '')");
        }
    }
    if filters.top_level_only.unwrap_or(false) {
        builder.push(" AND r.ZPARENTREMINDER IS NULL");
    }
    if let Some(q) = &filters.q {
        builder.push(" AND (lower(r.ZTITLE) LIKE '%' || lower(");
        builder.push_bind(q);
        builder.push(") || '%' OR lower(coalesce(r.ZNOTES, '')) LIKE '%' || lower(");
        builder.push_bind(q);
        builder.push(") || '%')");
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
