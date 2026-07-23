use serde::Serialize;
use utoipa::ToSchema;

use super::pagination::PageMetaDto;
use crate::apple_types::{NoteAttachmentId, NoteFolderId, NoteId, UnixTimestamp};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum FolderKindDto {
    Standard,
    Smart,
    Deleted,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct NoteFolderSummaryDto {
    pub id: NoteFolderId,
    pub row_id: i64,
    pub title: String,
    pub kind: FolderKindDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<NoteFolderId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified_at: Option<UnixTimestamp>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct NoteFolderDetailDto {
    pub id: NoteFolderId,
    pub row_id: i64,
    pub title: String,
    pub kind: FolderKindDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<NoteFolderId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_row_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified_at: Option<UnixTimestamp>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct NoteFolderPageDto {
    pub items: Vec<NoteFolderSummaryDto>,
    pub page: PageMetaDto,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct NoteSummaryDto {
    pub id: NoteId,
    pub row_id: i64,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snippet: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folder_id: Option<NoteFolderId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folder_row_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folder_name: Option<String>,
    pub folder_kind: FolderKindDto,
    pub is_pinned: bool,
    pub has_checklist: bool,
    pub is_locked: bool,
    pub marked_for_deletion: bool,
    pub has_attachments: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<UnixTimestamp>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified_at: Option<UnixTimestamp>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ParagraphStyleKindDto {
    Title,
    Heading,
    Monospace,
    BulletList,
    DashList,
    NumberedList,
    Checklist,
    Unknown,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ParagraphStyleDto {
    pub style: ParagraphStyleKindDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub todo_uuid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub done: Option<bool>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct NoteRunDto {
    pub start: usize,
    pub length: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paragraph_style: Option<ParagraphStyleDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_hints: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ChecklistItemDto {
    pub id: String,
    pub text: String,
    pub done: bool,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct EmbeddedObjectDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attachment_identifier: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_uti: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct NoteBodyDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub runs: Vec<NoteRunDto>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub checklist_items: Vec<ChecklistItemDto>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub embedded: Vec<EmbeddedObjectDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decode_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct NoteAttachmentSummaryDto {
    pub id: NoteAttachmentId,
    pub row_id: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uti: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_size: Option<i64>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct NoteDetailDto {
    pub id: NoteId,
    pub row_id: i64,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snippet: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folder_id: Option<NoteFolderId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folder_row_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folder_name: Option<String>,
    pub folder_kind: FolderKindDto,
    pub is_pinned: bool,
    pub has_checklist: bool,
    pub is_locked: bool,
    pub marked_for_deletion: bool,
    pub has_attachments: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<UnixTimestamp>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified_at: Option<UnixTimestamp>,
    pub body: NoteBodyDto,
    pub attachments: Vec<NoteAttachmentSummaryDto>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct NotePageDto {
    pub items: Vec<NoteSummaryDto>,
    pub page: PageMetaDto,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct NoteAttachmentDetailDto {
    pub id: NoteAttachmentId,
    pub row_id: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uti: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_size: Option<i64>,
    pub note_id: NoteId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified_at: Option<UnixTimestamp>,
}

/// YAML front matter schema for `GET /v1/notes/{note_id}/contents`.
///
/// The HTTP response is a single `text/markdown` document; this schema documents
/// the fields serialized between the opening and closing `---` delimiters.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct NoteContentsPreambleDto {
    pub schema_version: u32,
    pub id: NoteId,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folder: Option<NoteContentsFolderDto>,
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<UnixTimestamp>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified_at: Option<UnixTimestamp>,
    pub is_pinned: bool,
    pub has_checklist: bool,
    pub is_locked: bool,
    pub marked_for_deletion: bool,
    pub has_attachments: bool,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct NoteContentsFolderDto {
    pub id: NoteFolderId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub kind: FolderKindDto,
}
