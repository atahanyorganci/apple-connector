use super::model::{
    ChecklistItem, EmbeddedObject, NoteBody, NoteRun, ParagraphStyle, ParagraphStyleKind,
};

pub fn decode_notedata(data: Option<&[u8]>, is_locked: bool) -> NoteBody {
    if is_locked {
        return empty_body();
    }

    let Some(data) = data.filter(|bytes| !bytes.is_empty()) else {
        return empty_body();
    };

    let decoded = apple_notes_protobuf::decode_note_body(data);
    if let Some(error) = decoded.decode_error {
        if error == apple_notes_protobuf::DecodeError::Empty.to_string() {
            return empty_body();
        }
        return NoteBody {
            decode_error: Some(error),
            ..Default::default()
        };
    }

    NoteBody {
        text: decoded.text,
        runs: decoded.runs.into_iter().map(note_run_from_protobuf).collect(),
        checklist_items: decoded
            .checklist_items
            .into_iter()
            .map(checklist_item_from_protobuf)
            .collect(),
        embedded: decoded
            .embedded
            .into_iter()
            .map(embedded_object_from_protobuf)
            .collect(),
        decode_error: None,
    }
}

fn note_run_from_protobuf(run: apple_notes_protobuf::NoteRun) -> NoteRun {
    NoteRun {
        start: run.start,
        length: run.length,
        paragraph_style: run.paragraph_style.map(paragraph_style_from_protobuf),
        font_hints: run.font_hints,
        link: run.link,
    }
}

fn paragraph_style_from_protobuf(style: apple_notes_protobuf::ParagraphStyle) -> ParagraphStyle {
    ParagraphStyle {
        style: paragraph_style_kind_from_protobuf(style.style),
        todo_uuid: style.todo_uuid,
        done: style.done,
    }
}

fn paragraph_style_kind_from_protobuf(
    kind: apple_notes_protobuf::ParagraphStyleKind,
) -> ParagraphStyleKind {
    match kind {
        apple_notes_protobuf::ParagraphStyleKind::Title => ParagraphStyleKind::Title,
        apple_notes_protobuf::ParagraphStyleKind::Heading => ParagraphStyleKind::Heading,
        apple_notes_protobuf::ParagraphStyleKind::Monospace => ParagraphStyleKind::Monospace,
        apple_notes_protobuf::ParagraphStyleKind::BulletList => ParagraphStyleKind::BulletList,
        apple_notes_protobuf::ParagraphStyleKind::DashList => ParagraphStyleKind::DashList,
        apple_notes_protobuf::ParagraphStyleKind::NumberedList => ParagraphStyleKind::NumberedList,
        apple_notes_protobuf::ParagraphStyleKind::Checklist => ParagraphStyleKind::Checklist,
        apple_notes_protobuf::ParagraphStyleKind::Unknown(value) => {
            ParagraphStyleKind::Unknown(value)
        }
    }
}

fn checklist_item_from_protobuf(item: apple_notes_protobuf::ChecklistItem) -> ChecklistItem {
    ChecklistItem {
        id: item.id,
        text: item.text,
        done: item.done,
    }
}

fn embedded_object_from_protobuf(object: apple_notes_protobuf::EmbeddedObject) -> EmbeddedObject {
    EmbeddedObject {
        attachment_identifier: object.attachment_identifier,
        type_uti: object.type_uti,
    }
}

fn empty_body() -> NoteBody {
    NoteBody::default()
}

#[cfg(test)]
mod tests {
    use super::decode_notedata;

    #[test]
    fn locked_notes_never_decode_body() {
        let body = decode_notedata(Some(b"secret-bytes"), true);
        assert!(body.text.is_none());
        assert!(body.decode_error.is_none());
    }

    #[test]
    fn empty_body_is_none_without_error() {
        let body = decode_notedata(None, false);
        assert!(body.text.is_none());
        assert!(body.decode_error.is_none());
    }
}
