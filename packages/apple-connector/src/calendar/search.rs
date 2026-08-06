//! Event search filters for calendar listing endpoints.

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

/// Bind parameters for compile-time checked event listing queries.
#[derive(Debug, Clone)]
pub struct EventFilterBinds {
    pub include_hidden: i64,
    pub include_cancelled: i64,
    pub calendar_id: Option<String>,
    pub account_id: Option<String>,
    pub q_pattern: Option<String>,
    pub start_after: Option<f64>,
    pub start_before: Option<f64>,
    pub cursor_at: Option<f64>,
    pub cursor_row_id: Option<i64>,
    pub limit: i64,
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

    pub fn bind_values(
        &self,
        cursor_at: Option<f64>,
        cursor_row_id: Option<i64>,
        limit: i64,
    ) -> EventFilterBinds {
        EventFilterBinds {
            include_hidden: i64::from(self.include_hidden),
            include_cancelled: i64::from(self.include_cancelled),
            calendar_id: self.calendar_id.clone(),
            account_id: self.account_id.clone(),
            q_pattern: self.q.as_ref().map(|q| format!("%{q}%")),
            start_after: self.start_after,
            start_before: self.start_before,
            cursor_at,
            cursor_row_id,
            limit,
        }
    }
}
