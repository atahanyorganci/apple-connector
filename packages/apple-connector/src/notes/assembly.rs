use super::{
    decode::decode_notedata,
    model::{FolderKind, NoteAttachment, NoteBody, NoteDetail, NoteFolder, NoteSummary},
    row::{AttachmentRow, FolderRow, NoteDetailRow, NoteRow, parse_core_data_timestamp},
};

pub fn folder_from_row(row: FolderRow) -> NoteFolder {
    NoteFolder {
        row_id: row.row_id,
        id: row.id,
        title: row.title.unwrap_or_else(|| "Untitled".to_owned()),
        kind: FolderKind::from_raw(row.folder_type),
        parent_row_id: row.parent_row_id,
        parent_id: row.parent_id,
        account_row_id: row.account_row_id,
        account_id: row.account_id,
        modified_at: parse_core_data_timestamp(row.modified_at),
    }
}

pub fn note_summary_from_row(row: NoteRow, has_attachments: bool) -> NoteSummary {
    NoteSummary {
        row_id: row.row_id,
        id: row.id,
        title: row.title.unwrap_or_else(|| "Untitled".to_owned()),
        snippet: row.snippet,
        folder_row_id: row.folder_row_id,
        folder_id: row.folder_id,
        folder_name: row.folder_name,
        folder_kind: FolderKind::from_raw(row.folder_type),
        is_pinned: row.is_pinned,
        has_checklist: row.has_checklist,
        is_locked: row.is_locked,
        marked_for_deletion: row.marked_for_deletion,
        has_attachments,
        created_at: parse_core_data_timestamp(row.created_at),
        modified_at: parse_core_data_timestamp(row.modified_at),
    }
}

pub fn note_detail_from_row(row: NoteDetailRow, has_attachments: bool) -> NoteDetail {
    let summary = note_summary_from_row(
        NoteRow {
            row_id: row.row_id,
            id: row.id,
            title: row.title,
            snippet: row.snippet,
            created_at: row.created_at,
            modified_at: row.modified_at,
            folder_row_id: row.folder_row_id,
            folder_id: row.folder_id,
            folder_name: row.folder_name,
            folder_type: row.folder_type,
            is_pinned: row.is_pinned,
            has_checklist: row.has_checklist,
            is_locked: row.is_locked,
            marked_for_deletion: row.marked_for_deletion,
        },
        has_attachments,
    );
    let body = decode_notedata(row.note_data.as_deref(), row.is_locked);
    NoteDetail { summary, body }
}

pub fn attachment_from_row(row: AttachmentRow) -> NoteAttachment {
    NoteAttachment {
        row_id: row.row_id,
        id: row.id,
        filename: row.filename,
        uti: row.uti,
        note_row_id: row.note_row_id,
        note_id: row.note_id,
        file_size: row.file_size,
        account_id: row.account_id,
        modified_at: parse_core_data_timestamp(row.modified_at),
    }
}

#[allow(dead_code)]
pub fn empty_note_body() -> NoteBody {
    NoteBody::default()
}
