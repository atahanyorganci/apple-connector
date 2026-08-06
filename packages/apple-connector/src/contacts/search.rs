#[derive(Debug, Clone, Default)]
pub struct ContactFilters {
    pub q: Option<String>,
    pub container_id: Option<String>,
    pub group_id: Option<String>,
}

/// Bind parameters for the compile-time filtered contact listing query.
#[derive(Debug, Clone)]
pub struct ContactFilterBinds {
    pub q_pattern: Option<String>,
    pub container_id: Option<String>,
    pub group_id: Option<String>,
    pub cursor_row_id: Option<i64>,
    pub limit: i64,
}

impl ContactFilters {
    pub fn bind_values(&self, cursor_row_id: Option<i64>, limit: i64) -> ContactFilterBinds {
        ContactFilterBinds {
            q_pattern: self.q.as_ref().map(|q| format!("%{q}%")),
            container_id: self.container_id.clone(),
            group_id: self.group_id.clone(),
            cursor_row_id,
            limit,
        }
    }
}
