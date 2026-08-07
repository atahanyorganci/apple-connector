use serde::Serialize;

use super::model::{FolderKind, NoteBody, NoteSummary, ParagraphStyleKind};

pub const NOTE_CONTENTS_SCHEMA_VERSION: u32 = 1;

const OBJECT_REPLACEMENT: char = '\u{FFFC}';

#[derive(Debug, Clone, Serialize)]
pub struct NoteContentsPreamble {
    pub schema_version: u32,
    pub id: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folder: Option<NoteContentsFolder>,
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified_at: Option<i64>,
    pub is_pinned: bool,
    pub has_checklist: bool,
    pub is_locked: bool,
    pub marked_for_deletion: bool,
    pub has_attachments: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct NoteContentsFolder {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub kind: NoteContentsFolderKind,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NoteContentsFolderKind {
    Standard,
    Smart,
    Deleted,
}

impl From<&FolderKind> for NoteContentsFolderKind {
    fn from(kind: &FolderKind) -> Self {
        match kind {
            FolderKind::Standard => Self::Standard,
            FolderKind::Smart => Self::Smart,
            FolderKind::Deleted => Self::Deleted,
        }
    }
}

pub fn preamble_from_note(summary: &NoteSummary, tags: Vec<String>) -> NoteContentsPreamble {
    let folder = summary.folder_id.as_ref().map(|id| NoteContentsFolder {
        id: id.as_str().to_owned(),
        name: summary.folder_name.clone(),
        kind: NoteContentsFolderKind::from(&summary.folder_kind),
    });

    NoteContentsPreamble {
        schema_version: NOTE_CONTENTS_SCHEMA_VERSION,
        id: summary.id.as_str().to_owned(),
        title: summary.title.as_deref().unwrap_or("Untitled").to_owned(),
        folder,
        tags,
        created_at: summary.created_at.map(|ts| ts.timestamp()),
        modified_at: summary.modified_at.map(|ts| ts.timestamp()),
        is_pinned: summary.is_pinned,
        has_checklist: summary.has_checklist,
        is_locked: summary.is_locked,
        marked_for_deletion: summary.marked_for_deletion,
        has_attachments: summary.has_attachments,
    }
}

pub fn render_document(preamble: &NoteContentsPreamble, body: &NoteBody) -> String {
    let yaml = serde_yaml::to_string(preamble).unwrap_or_else(|_| "schema_version: 1\n".to_owned());
    let yaml = yaml.trim_end();
    let markdown_body = body_to_markdown(body);

    if markdown_body.is_empty() {
        format!("---\n{yaml}\n---\n")
    } else {
        format!("---\n{yaml}\n---\n\n{markdown_body}")
    }
}

pub fn body_to_markdown(body: &NoteBody) -> String {
    if body.decode_error.is_some() {
        return String::new();
    }

    let Some(text) = body.text.as_deref() else {
        return String::new();
    };

    if body.runs.is_empty() {
        let stripped = strip_object_replacement(text);
        let trimmed = stripped.trim();
        if trimmed.is_empty() {
            return String::new();
        }
        return format!("{trimmed}\n");
    }

    let mut out = String::new();
    let mut numbered = 0_u32;
    let mut at_line_start = true;

    for run in &body.runs {
        let raw: String = text
            .chars()
            .skip(run.start)
            .take(run.length as usize)
            .collect();

        let mut segment = String::new();
        for ch in raw.chars() {
            if ch == OBJECT_REPLACEMENT {
                continue;
            }
            if ch == '\n' {
                emit_segment(&mut out, &mut at_line_start, &mut numbered, run, &segment);
                out.push('\n');
                at_line_start = true;
                segment.clear();
            } else {
                segment.push(ch);
            }
        }
        emit_segment(&mut out, &mut at_line_start, &mut numbered, run, &segment);
    }

    let trimmed = out.trim_end_matches('\n').trim_end();
    if trimmed.is_empty() {
        String::new()
    } else {
        format!("{trimmed}\n")
    }
}

fn emit_segment(
    out: &mut String,
    at_line_start: &mut bool,
    numbered: &mut u32,
    run: &super::model::NoteRun,
    segment: &str,
) {
    if segment.is_empty() {
        return;
    }

    if *at_line_start {
        let style = run.paragraph_style.as_ref().map(|style| &style.style);

        match style {
            Some(ParagraphStyleKind::Title) => {
                out.push_str("# ");
                *numbered = 0;
            }
            Some(ParagraphStyleKind::Heading) => {
                out.push_str("## ");
                *numbered = 0;
            }
            Some(ParagraphStyleKind::BulletList | ParagraphStyleKind::DashList) => {
                out.push_str("- ");
                *numbered = 0;
            }
            Some(ParagraphStyleKind::NumberedList) => {
                *numbered = numbered.saturating_add(1);
                out.push_str(&format!("{numbered}. "));
            }
            Some(ParagraphStyleKind::Checklist) => {
                let done = run
                    .paragraph_style
                    .as_ref()
                    .and_then(|style| style.done)
                    .unwrap_or(false);
                if done {
                    out.push_str("- [x] ");
                } else {
                    out.push_str("- [ ] ");
                }
                *numbered = 0;
            }
            Some(ParagraphStyleKind::Monospace) | Some(ParagraphStyleKind::Unknown(_)) | None => {
                *numbered = 0;
            }
        }
        *at_line_start = false;
    }

    let content = format_inline(segment, run);
    out.push_str(&content);
}

fn format_inline(segment: &str, run: &super::model::NoteRun) -> String {
    let monospace = run
        .paragraph_style
        .as_ref()
        .is_some_and(|style| matches!(style.style, ParagraphStyleKind::Monospace));

    let escaped = if monospace {
        format!("`{}`", segment.replace('`', "\\`"))
    } else {
        segment.to_owned()
    };

    if let Some(link) = run.link.as_deref() {
        format!("[{escaped}]({link})")
    } else {
        escaped
    }
}

fn strip_object_replacement(text: &str) -> String {
    text.chars()
        .filter(|&ch| ch != OBJECT_REPLACEMENT)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::notes::model::{NoteRun, ParagraphStyle, ParagraphStyleKind};

    fn fixture_body(name: &str) -> NoteBody {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/notes/bodies")
            .join(name);
        let data = std::fs::read(&path).unwrap_or_else(|error| {
            panic!("failed to read fixture {}: {error}", path.display());
        });
        crate::notes::decode::decode_notedata(Some(&data), false)
    }

    #[test]
    fn strips_object_replacement_characters() {
        let body = NoteBody {
            text: Some("hello\u{FFFC} world".to_owned()),
            runs: vec![NoteRun {
                start: 0,
                length: 13,
                paragraph_style: None,
                font_hints: None,
                link: None,
            }],
            ..Default::default()
        };
        let markdown = body_to_markdown(&body);
        assert_eq!(markdown, "hello world\n");
        assert!(!markdown.contains('\u{FFFC}'));
    }

    #[test]
    fn renders_checklist_items() {
        let body = NoteBody {
            text: Some("Task one\nTask two".to_owned()),
            runs: vec![
                NoteRun {
                    start: 0,
                    length: 9,
                    paragraph_style: Some(ParagraphStyle {
                        style: ParagraphStyleKind::Checklist,
                        todo_uuid: Some("a".to_owned()),
                        done: Some(false),
                    }),
                    font_hints: None,
                    link: None,
                },
                NoteRun {
                    start: 9,
                    length: 8,
                    paragraph_style: Some(ParagraphStyle {
                        style: ParagraphStyleKind::Checklist,
                        todo_uuid: Some("b".to_owned()),
                        done: Some(true),
                    }),
                    font_hints: None,
                    link: None,
                },
            ],
            ..Default::default()
        };
        let markdown = body_to_markdown(&body);
        assert!(markdown.contains("- [ ] Task one"));
        assert!(markdown.contains("- [x] Task two"));
    }

    #[test]
    fn renders_plain_text_fixture() {
        let body = fixture_body("plain-text.bin");
        let markdown = body_to_markdown(&body);
        assert!(markdown.contains("IBAN"), "markdown: {markdown}");
        assert!(!markdown.contains('\u{FFFC}'));
    }

    #[test]
    fn renders_checklist_fixture() {
        let body = fixture_body("checklist.bin");
        let markdown = body_to_markdown(&body);
        assert!(!markdown.is_empty());
        assert!(
            markdown.contains("Simulacra") || markdown.contains("Algorithms"),
            "markdown: {markdown}"
        );
        assert!(
            markdown.contains("- [ ]") || markdown.contains("- [x]"),
            "markdown: {markdown}"
        );
        assert!(!markdown.contains('\u{FFFC}'));
    }

    #[test]
    fn render_document_includes_yaml_preamble_and_tags() {
        let preamble = NoteContentsPreamble {
            schema_version: 1,
            id: "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb".to_owned(),
            title: "Fixture Checklist".to_owned(),
            folder: Some(NoteContentsFolder {
                id: "22222222-2222-2222-2222-222222222222".to_owned(),
                name: Some("Projects".to_owned()),
                kind: NoteContentsFolderKind::Smart,
            }),
            tags: vec!["reading".to_owned()],
            created_at: Some(789004800),
            modified_at: Some(789145691),
            is_pinned: false,
            has_checklist: true,
            is_locked: false,
            marked_for_deletion: false,
            has_attachments: false,
        };
        let body = NoteBody {
            text: Some("Hello".to_owned()),
            runs: vec![NoteRun {
                start: 0,
                length: 5,
                paragraph_style: Some(ParagraphStyle {
                    style: ParagraphStyleKind::Title,
                    todo_uuid: None,
                    done: None,
                }),
                font_hints: None,
                link: None,
            }],
            ..Default::default()
        };
        let document = render_document(&preamble, &body);
        assert!(document.starts_with("---\n"));
        assert!(document.contains("schema_version: 1"));
        assert!(document.contains("tags:\n- reading") || document.contains("tags:\n  - reading"));
        assert!(document.contains("# Hello"));
    }

    #[test]
    fn decode_error_yields_empty_body() {
        let body = NoteBody {
            decode_error: Some("boom".to_owned()),
            text: Some("secret".to_owned()),
            ..Default::default()
        };
        assert!(body_to_markdown(&body).is_empty());
    }
}
