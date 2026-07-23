use chrono::{DateTime, Utc};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FolderKind {
    Standard,
    Smart,
    Deleted,
}

impl FolderKind {
    pub fn from_raw(value: Option<i64>) -> Self {
        match value.unwrap_or(0) {
            1 => Self::Deleted,
            2 => Self::Smart,
            _ => Self::Standard,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct NoteBody {
    pub text: Option<String>,
    pub runs: Vec<NoteRun>,
    pub checklist_items: Vec<ChecklistItem>,
    pub embedded: Vec<EmbeddedObject>,
    pub decode_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteRun {
    pub start: usize,
    pub length: u32,
    pub paragraph_style: Option<ParagraphStyle>,
    pub font_hints: Option<u32>,
    pub link: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParagraphStyle {
    pub style: ParagraphStyleKind,
    pub todo_uuid: Option<String>,
    pub done: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParagraphStyleKind {
    Title,
    Heading,
    Monospace,
    BulletList,
    DashList,
    NumberedList,
    Checklist,
    Unknown(u32),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChecklistItem {
    pub id: String,
    pub text: String,
    pub done: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbeddedObject {
    pub attachment_identifier: Option<String>,
    pub type_uti: Option<String>,
}

#[derive(Debug, Clone)]
pub struct NoteFolder {
    pub row_id: i64,
    pub id: String,
    pub title: String,
    pub kind: FolderKind,
    pub parent_row_id: Option<i64>,
    pub parent_id: Option<String>,
    #[allow(dead_code)]
    pub account_row_id: Option<i64>,
    pub account_id: Option<String>,
    pub modified_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct NoteSummary {
    pub row_id: i64,
    pub id: String,
    pub title: String,
    pub snippet: Option<String>,
    pub folder_row_id: Option<i64>,
    pub folder_id: Option<String>,
    pub folder_name: Option<String>,
    pub folder_kind: FolderKind,
    pub is_pinned: bool,
    pub has_checklist: bool,
    pub is_locked: bool,
    pub marked_for_deletion: bool,
    pub has_attachments: bool,
    pub created_at: Option<DateTime<Utc>>,
    pub modified_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct NoteDetail {
    pub summary: NoteSummary,
    pub body: NoteBody,
}

#[derive(Debug, Clone)]
pub struct NoteAttachment {
    pub row_id: i64,
    pub id: String,
    pub filename: Option<String>,
    pub uti: Option<String>,
    #[allow(dead_code)]
    pub note_row_id: i64,
    pub note_id: String,
    pub file_size: Option<i64>,
    pub account_id: Option<String>,
    pub modified_at: Option<DateTime<Utc>>,
}
