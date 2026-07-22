use std::{
    collections::HashMap,
    env,
    path::{Path, PathBuf},
};

use super::{
    model::{Attachment, AttachmentBodyRef, AttachmentKind, BodyAttribute, MessageBody},
    row::AttachmentRow,
};

/// Apple `attachment.transfer_state` value for a finished transfer.
pub const TRANSFER_STATE_COMPLETE: i64 = 5;

pub fn assemble_attachments(rows: &[AttachmentRow], body: &MessageBody) -> Vec<Attachment> {
    let body_refs = body_file_transfer_refs(body);
    let mut attachments: Vec<Attachment> = rows
        .iter()
        .map(|row| assemble_attachment(row, &body_refs))
        .collect();

    attachments.sort_by(|left, right| {
        match (
            left.body_reference
                .as_ref()
                .and_then(|reference| reference.part),
            right
                .body_reference
                .as_ref()
                .and_then(|reference| reference.part),
        ) {
            (Some(left_part), Some(right_part)) => left_part
                .cmp(&right_part)
                .then_with(|| left.guid.cmp(&right.guid)),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => left.guid.cmp(&right.guid),
        }
    });

    attachments
}

pub(crate) fn assemble_attachment(
    row: &AttachmentRow,
    body_refs: &HashMap<&str, AttachmentBodyRef>,
) -> Attachment {
    let body_reference = body_refs
        .get(row.guid.as_str())
        .or_else(|| body_refs.get(row.original_guid.as_str()))
        .cloned();
    let resolved_path = row
        .filename
        .as_deref()
        .and_then(resolve_attachment_path)
        .map(|path| path.to_string_lossy().into_owned());
    let present_on_disk = resolved_path
        .as_deref()
        .is_some_and(|path| Path::new(path).is_file());
    let kind = classify_kind(row);

    Attachment {
        guid: row.guid.clone(),
        original_guid: row.original_guid.clone(),
        filename: row.filename.clone(),
        resolved_path,
        uti: row.uti.clone(),
        mime_type: row.mime_type.clone(),
        transfer_name: row.transfer_name.clone(),
        total_bytes: row.total_bytes,
        kind,
        transfer_state: row.transfer_state,
        transfer_complete: row.transfer_state == TRANSFER_STATE_COMPLETE,
        present_on_disk,
        hide_attachment: row.hide_attachment,
        emoji_description: row.emoji_description.clone(),
        body_reference,
    }
}

fn body_file_transfer_refs(body: &MessageBody) -> HashMap<&str, AttachmentBodyRef> {
    let mut refs = HashMap::new();
    for run in &body.runs {
        for attribute in &run.attributes {
            if let BodyAttribute::FileTransfer {
                guid,
                inline_sticker,
            } = attribute
            {
                refs.entry(guid.as_str()).or_insert(AttachmentBodyRef {
                    part: run.part,
                    inline_sticker: *inline_sticker,
                });
            }
        }
    }
    refs
}

pub fn classify_kind(row: &AttachmentRow) -> AttachmentKind {
    let media = media_category(row.mime_type.as_deref(), row.uti.as_deref());

    if row.is_sticker {
        let animated = matches!(
            media,
            MediaCategory::Video | MediaCategory::ImageAnimated | MediaCategory::ImageSequence
        );
        return AttachmentKind::Sticker { animated };
    }

    match media {
        MediaCategory::Image | MediaCategory::ImageAnimated | MediaCategory::ImageSequence => {
            AttachmentKind::Image
        }
        MediaCategory::Video => AttachmentKind::Video,
        MediaCategory::Audio => AttachmentKind::Audio,
        MediaCategory::File => AttachmentKind::File,
        MediaCategory::Unknown => AttachmentKind::Unknown,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MediaCategory {
    Image,
    ImageAnimated,
    ImageSequence,
    Video,
    Audio,
    File,
    Unknown,
}

fn media_category(mime_type: Option<&str>, uti: Option<&str>) -> MediaCategory {
    if let Some(mime) = mime_type {
        let mime = mime.to_ascii_lowercase();
        if let Some((major, subtype)) = mime.split_once('/') {
            match major {
                "image" => {
                    return match subtype {
                        "gif" | "webp" => MediaCategory::ImageAnimated,
                        "heics" | "heic-sequence" => MediaCategory::ImageSequence,
                        _ => MediaCategory::Image,
                    };
                }
                "video" => return MediaCategory::Video,
                "audio" => return MediaCategory::Audio,
                "text" | "application" => return MediaCategory::File,
                _ => {}
            }
        }
    }

    match uti.map(str::to_ascii_lowercase).as_deref() {
        Some("public.heic" | "public.jpeg" | "public.png" | "public.tiff" | "public.image") => {
            MediaCategory::Image
        }
        Some("com.compuserve.gif" | "public.webp") => MediaCategory::ImageAnimated,
        Some("public.heics" | "public.heic-sequence") => MediaCategory::ImageSequence,
        Some("public.mpeg-4" | "com.apple.quicktime-movie" | "public.movie" | "public.video") => {
            MediaCategory::Video
        }
        Some(
            "com.apple.coreaudio-format" | "public.audio" | "public.mp3" | "public.mpeg-4-audio",
        ) => MediaCategory::Audio,
        Some(_) => MediaCategory::File,
        None => MediaCategory::Unknown,
    }
}

pub fn resolve_attachment_path(filename: &str) -> Option<PathBuf> {
    let trimmed = filename.trim();
    if trimmed.is_empty() {
        return None;
    }

    if let Some(rest) = trimmed.strip_prefix("~/") {
        let home = env::var_os("HOME")?;
        return Some(PathBuf::from(home).join(rest));
    }

    if trimmed == "~" {
        return env::var_os("HOME").map(PathBuf::from);
    }

    Some(PathBuf::from(trimmed))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{
        TRANSFER_STATE_COMPLETE, assemble_attachments, classify_kind, resolve_attachment_path,
    };
    use crate::messages::{
        model::{AttachmentBodyRef, AttachmentKind, AttributedRun, BodyAttribute, MessageBody},
        row::AttachmentRow,
    };

    fn row(guid: &str) -> AttachmentRow {
        AttachmentRow {
            message_id: 1,
            guid: guid.to_owned(),
            original_guid: guid.to_owned(),
            filename: None,
            uti: None,
            mime_type: None,
            transfer_name: None,
            total_bytes: 0,
            is_sticker: false,
            transfer_state: 0,
            hide_attachment: false,
            emoji_description: None,
        }
    }

    #[test]
    fn classifies_sticker_image_video_audio() {
        let mut sticker = row("s");
        sticker.is_sticker = true;
        sticker.mime_type = Some("image/heic".to_owned());
        assert_eq!(
            classify_kind(&sticker),
            AttachmentKind::Sticker { animated: false }
        );

        let mut animated = row("a");
        animated.is_sticker = true;
        animated.mime_type = Some("video/mp4".to_owned());
        assert_eq!(
            classify_kind(&animated),
            AttachmentKind::Sticker { animated: true }
        );

        let mut image = row("i");
        image.mime_type = Some("image/jpeg".to_owned());
        assert_eq!(classify_kind(&image), AttachmentKind::Image);

        let mut video = row("v");
        video.uti = Some("com.apple.quicktime-movie".to_owned());
        assert_eq!(classify_kind(&video), AttachmentKind::Video);

        let mut audio = row("au");
        audio.uti = Some("com.apple.coreaudio-format".to_owned());
        assert_eq!(classify_kind(&audio), AttachmentKind::Audio);
    }

    #[test]
    fn links_attachment_guid_to_attributed_body_file_transfer() {
        let body = MessageBody {
            text: Some("\u{fffc}caption".to_owned()),
            runs: vec![
                AttributedRun {
                    start: 0,
                    end: 3,
                    part: Some(0),
                    attributes: vec![BodyAttribute::FileTransfer {
                        guid: "at_0_ABC".to_owned(),
                        inline_sticker: false,
                    }],
                },
                AttributedRun {
                    start: 3,
                    end: 10,
                    part: Some(1),
                    attributes: Vec::new(),
                },
            ],
            attributed_body_error: None,
        };
        let mut attachment = row("at_0_ABC");
        attachment.transfer_state = TRANSFER_STATE_COMPLETE;
        attachment.mime_type = Some("image/heic".to_owned());

        let assembled = assemble_attachments(&[attachment], &body);
        assert_eq!(assembled.len(), 1);
        assert_eq!(
            assembled[0].body_reference,
            Some(AttachmentBodyRef {
                part: Some(0),
                inline_sticker: false,
            })
        );
        assert!(assembled[0].transfer_complete);
        assert!(!assembled[0].present_on_disk);
        assert_eq!(assembled[0].kind, AttachmentKind::Image);
        assert_eq!(body.text.as_deref(), Some("\u{fffc}caption"));
    }

    #[test]
    fn matches_original_guid_when_body_uses_it() {
        let body = MessageBody {
            text: Some("\u{fffc}".to_owned()),
            runs: vec![AttributedRun {
                start: 0,
                end: 3,
                part: Some(0),
                attributes: vec![BodyAttribute::FileTransfer {
                    guid: "orig-guid".to_owned(),
                    inline_sticker: true,
                }],
            }],
            attributed_body_error: None,
        };
        let mut attachment = row("display-guid");
        attachment.original_guid = "orig-guid".to_owned();
        attachment.is_sticker = true;
        attachment.mime_type = Some("image/heic".to_owned());

        let assembled = assemble_attachments(&[attachment], &body);
        assert_eq!(
            assembled[0].body_reference,
            Some(AttachmentBodyRef {
                part: Some(0),
                inline_sticker: true,
            })
        );
        assert_eq!(
            assembled[0].kind,
            AttachmentKind::Sticker { animated: false }
        );
    }

    #[test]
    fn expands_tilde_paths() {
        let home = std::env::var("HOME").expect("HOME");
        let resolved = resolve_attachment_path("~/Library/Messages/Attachments/x.bin").unwrap();
        assert_eq!(
            resolved,
            PathBuf::from(home).join("Library/Messages/Attachments/x.bin")
        );
        assert!(resolve_attachment_path("").is_none());
    }
}
