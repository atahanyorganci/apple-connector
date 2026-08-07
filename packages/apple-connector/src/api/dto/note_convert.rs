use super::{
    common::timestamp_to_unix,
    note::{
        ChecklistItemDto, EmbeddedObjectDto, FolderKindDto, NoteAttachmentDetailDto,
        NoteAttachmentSummaryDto, NoteBodyDto, NoteDetailDto, NoteFolderDetailDto,
        NoteFolderPageDto, NoteFolderSummaryDto, NotePageDto, NoteRunDto, NoteSummaryDto,
        ParagraphStyleDto, ParagraphStyleKindDto,
    },
    pagination::PageMetaDto,
};
use crate::notes::{
    ChecklistItem, EmbeddedObject, FolderKind, NoteAttachment, NoteBody, NoteDetail, NoteFolder,
    NoteRun, NoteSummary, ParagraphStyle, ParagraphStyleKind,
};

fn display_title(title: &Option<String>) -> String {
    title.as_deref().unwrap_or("Untitled").to_owned()
}

pub fn note_folder_page_to_dto(
    items: Vec<NoteFolder>,
    has_more: bool,
    next_cursor: Option<String>,
    limit: u32,
) -> NoteFolderPageDto {
    NoteFolderPageDto {
        items: items.iter().map(note_folder_summary_to_dto).collect(),
        page: PageMetaDto {
            limit,
            has_more,
            next_cursor,
        },
    }
}

pub fn note_page_to_dto(
    items: Vec<NoteSummary>,
    has_more: bool,
    next_cursor: Option<String>,
    limit: u32,
) -> NotePageDto {
    NotePageDto {
        items: items.iter().map(note_summary_to_dto).collect(),
        page: PageMetaDto {
            limit,
            has_more,
            next_cursor,
        },
    }
}

pub fn note_folder_summary_to_dto(folder: &NoteFolder) -> NoteFolderSummaryDto {
    NoteFolderSummaryDto {
        id: folder.id.clone(),
        row_id: folder.row_id.get(),
        title: display_title(&folder.title),
        kind: folder_kind_to_dto(&folder.kind),
        parent_id: folder.parent_id.clone(),
        modified_at: folder.modified_at.map(timestamp_to_unix),
    }
}

pub fn note_folder_detail_to_dto(folder: &NoteFolder) -> NoteFolderDetailDto {
    NoteFolderDetailDto {
        id: folder.id.clone(),
        row_id: folder.row_id.get(),
        title: display_title(&folder.title),
        kind: folder_kind_to_dto(&folder.kind),
        parent_id: folder.parent_id.clone(),
        parent_row_id: folder.parent_row_id.map(|id| id.get()),
        account_id: folder.account_id.clone(),
        modified_at: folder.modified_at.map(timestamp_to_unix),
    }
}

pub fn note_summary_to_dto(note: &NoteSummary) -> NoteSummaryDto {
    NoteSummaryDto {
        id: note.id.clone(),
        row_id: note.row_id.get(),
        title: display_title(&note.title),
        snippet: note.snippet.clone(),
        folder_id: note.folder_id.clone(),
        folder_row_id: note.folder_row_id.map(|id| id.get()),
        folder_name: note.folder_name.clone(),
        folder_kind: folder_kind_to_dto(&note.folder_kind),
        is_pinned: note.is_pinned,
        has_checklist: note.has_checklist,
        is_locked: note.is_locked,
        marked_for_deletion: note.marked_for_deletion,
        has_attachments: note.has_attachments,
        created_at: note.created_at.map(timestamp_to_unix),
        modified_at: note.modified_at.map(timestamp_to_unix),
    }
}

pub fn note_detail_to_dto(note: &NoteDetail, attachments: &[NoteAttachment]) -> NoteDetailDto {
    let summary = &note.summary;
    NoteDetailDto {
        id: summary.id.clone(),
        row_id: summary.row_id.get(),
        title: display_title(&summary.title),
        snippet: summary.snippet.clone(),
        folder_id: summary.folder_id.clone(),
        folder_row_id: summary.folder_row_id.map(|id| id.get()),
        folder_name: summary.folder_name.clone(),
        folder_kind: folder_kind_to_dto(&summary.folder_kind),
        is_pinned: summary.is_pinned,
        has_checklist: summary.has_checklist,
        is_locked: summary.is_locked,
        marked_for_deletion: summary.marked_for_deletion,
        has_attachments: summary.has_attachments,
        created_at: summary.created_at.map(timestamp_to_unix),
        modified_at: summary.modified_at.map(timestamp_to_unix),
        body: note_body_to_dto(&note.body),
        attachments: attachments
            .iter()
            .map(note_attachment_summary_to_dto)
            .collect(),
    }
}

pub fn note_attachment_detail_to_dto(attachment: &NoteAttachment) -> NoteAttachmentDetailDto {
    NoteAttachmentDetailDto {
        id: attachment.id.clone(),
        row_id: attachment.row_id.get(),
        filename: attachment.filename.clone(),
        uti: attachment.uti.clone(),
        file_size: attachment.file_size,
        note_id: attachment.note_id.clone(),
        modified_at: attachment.modified_at.map(timestamp_to_unix),
    }
}

fn note_body_to_dto(body: &NoteBody) -> NoteBodyDto {
    NoteBodyDto {
        text: body.text.clone(),
        runs: body.runs.iter().map(note_run_to_dto).collect(),
        checklist_items: body
            .checklist_items
            .iter()
            .map(checklist_item_to_dto)
            .collect(),
        embedded: body.embedded.iter().map(embedded_object_to_dto).collect(),
        decode_error: body.decode_error.clone(),
    }
}

fn note_run_to_dto(run: &NoteRun) -> NoteRunDto {
    NoteRunDto {
        start: run.start,
        length: run.length,
        paragraph_style: run.paragraph_style.as_ref().map(paragraph_style_to_dto),
        font_hints: run.font_hints,
        link: run.link.clone(),
    }
}

fn paragraph_style_to_dto(style: &ParagraphStyle) -> ParagraphStyleDto {
    ParagraphStyleDto {
        style: paragraph_style_kind_to_dto(&style.style),
        todo_uuid: style.todo_uuid.clone(),
        done: style.done,
    }
}

fn paragraph_style_kind_to_dto(kind: &ParagraphStyleKind) -> ParagraphStyleKindDto {
    match kind {
        ParagraphStyleKind::Title => ParagraphStyleKindDto::Title,
        ParagraphStyleKind::Heading => ParagraphStyleKindDto::Heading,
        ParagraphStyleKind::Monospace => ParagraphStyleKindDto::Monospace,
        ParagraphStyleKind::BulletList => ParagraphStyleKindDto::BulletList,
        ParagraphStyleKind::DashList => ParagraphStyleKindDto::DashList,
        ParagraphStyleKind::NumberedList => ParagraphStyleKindDto::NumberedList,
        ParagraphStyleKind::Checklist => ParagraphStyleKindDto::Checklist,
        ParagraphStyleKind::Unknown(_) => ParagraphStyleKindDto::Unknown,
    }
}

fn checklist_item_to_dto(item: &ChecklistItem) -> ChecklistItemDto {
    ChecklistItemDto {
        id: item.id.clone(),
        text: item.text.clone(),
        done: item.done,
    }
}

fn embedded_object_to_dto(object: &EmbeddedObject) -> EmbeddedObjectDto {
    EmbeddedObjectDto {
        attachment_identifier: object.attachment_identifier.clone(),
        type_uti: object.type_uti.clone(),
    }
}

fn note_attachment_summary_to_dto(attachment: &NoteAttachment) -> NoteAttachmentSummaryDto {
    NoteAttachmentSummaryDto {
        id: attachment.id.clone(),
        row_id: attachment.row_id.get(),
        filename: attachment.filename.clone(),
        uti: attachment.uti.clone(),
        file_size: attachment.file_size,
    }
}

fn folder_kind_to_dto(kind: &FolderKind) -> FolderKindDto {
    match kind {
        FolderKind::Standard => FolderKindDto::Standard,
        FolderKind::Smart => FolderKindDto::Smart,
        FolderKind::Deleted => FolderKindDto::Deleted,
    }
}
