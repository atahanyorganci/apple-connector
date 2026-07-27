//! Event search filters for calendar listing endpoints.

use sqlx::{QueryBuilder, Sqlite};

#[derive(Debug, Clone, PartialEq, Default)]
pub struct EventFilters {
    pub q: Option<String>,
    pub calendar_id: Option<String>,
    pub account_id: Option<String>,
    pub start_after: Option<f64>,
    pub start_before: Option<f64>,
    pub include_hidden: bool,
    pub include_cancelled: bool,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EventFiltersSnapshot {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub q: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub calendar_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_after: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_before: Option<f64>,
    pub include_hidden: bool,
    pub include_cancelled: bool,
}

impl EventFilters {
    pub fn is_active(&self) -> bool {
        self.q.is_some()
            || self.calendar_id.is_some()
            || self.account_id.is_some()
            || self.start_after.is_some()
            || self.start_before.is_some()
            || self.include_hidden
            || self.include_cancelled
    }

    pub fn snapshot(&self) -> EventFiltersSnapshot {
        EventFiltersSnapshot {
            q: self.q.clone(),
            calendar_id: self.calendar_id.clone(),
            account_id: self.account_id.clone(),
            start_after: self.start_after,
            start_before: self.start_before,
            include_hidden: self.include_hidden,
            include_cancelled: self.include_cancelled,
        }
    }
}

pub fn apply_event_filters(
    builder: &mut QueryBuilder<Sqlite>,
    filters: &EventFilters,
    alias: &str,
) {
    if !filters.include_hidden {
        builder.push(format!(" AND {alias}.hidden = 0"));
    }
    if !filters.include_cancelled {
        builder.push(format!(" AND COALESCE({alias}.status, 0) != 2"));
    }
    if let Some(calendar_id) = &filters.calendar_id {
        builder.push(" AND lower(c.UUID) = lower(");
        builder.push_bind(calendar_id.clone());
        builder.push(")");
    }
    if let Some(account_id) = &filters.account_id {
        builder.push(" AND lower(s.external_id) = lower(");
        builder.push_bind(account_id.clone());
        builder.push(")");
    }
    if let Some(q) = &filters.q {
        builder.push(format!(" AND {alias}.summary LIKE "));
        builder.push_bind(format!("%{q}%"));
    }
}

pub fn apply_occurrence_date_range(
    builder: &mut QueryBuilder<Sqlite>,
    start_after: Option<f64>,
    start_before: Option<f64>,
) {
    if let Some(start) = start_after {
        builder.push(" AND oc.occurrence_end_date >= ");
        builder.push_bind(start);
    }
    if let Some(end) = start_before {
        builder.push(" AND oc.occurrence_start_date <= ");
        builder.push_bind(end);
    }
}

pub fn apply_direct_date_range(
    builder: &mut QueryBuilder<Sqlite>,
    start_after: Option<f64>,
    start_before: Option<f64>,
    alias: &str,
) {
    if let Some(start) = start_after {
        builder.push(format!(" AND {alias}.end_date >= "));
        builder.push_bind(start);
    }
    if let Some(end) = start_before {
        builder.push(format!(" AND {alias}.start_date <= "));
        builder.push_bind(end);
    }
}
