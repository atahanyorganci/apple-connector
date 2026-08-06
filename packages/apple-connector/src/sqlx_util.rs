//! Helpers for compile-time SQLx queries with dynamic bind shapes.

/// Serialize row ids for SQLite `json_each(?)` IN-clause lookups.
pub fn json_ids(ids: &[i64]) -> String {
    serde_json::to_string(ids).expect("serialize id list")
}

/// Map an optional bool filter to `None` (no filter) or `Some(0/1)`.
pub fn optional_bool_filter(value: Option<bool>) -> Option<i64> {
    value.map(i64::from)
}
