pub const DEFAULT_PAGE_LIMIT: u32 = 50;
pub const MAX_PAGE_LIMIT: u32 = 200;

use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PageMetaDto {
    /// Applied page size for this response.
    #[schema(minimum = 1, maximum = 200, example = 50)]
    pub limit: u32,

    /// Whether additional items exist beyond this page.
    pub has_more: bool,

    /// URL-safe versioned cursor for the next page when `has_more` is true.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "v1.eyJkYXRlIjoxNzA0MDk2MDAwfQ")]
    pub next_cursor: Option<String>,
}

impl PageMetaDto {
    pub fn empty(limit: u32) -> Self {
        Self {
            limit,
            has_more: false,
            next_cursor: None,
        }
    }
}
