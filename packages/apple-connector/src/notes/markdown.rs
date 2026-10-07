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
        let raw = run_text(text, run.start, run.length as usize);

        let mut segment = String::new();
        for ch in raw.chars() {
            if ch == OBJECT_REPLACEMENT {
                // A table renders where its placeholder stands; other objects are dropped.
                if let Some(table) = table_for_run(body, run) {
                    emit_segment(&mut out, &mut at_line_start, &mut numbered, run, &segment);
                    segment.clear();
                    if !at_line_start {
                        out.push('\n');
                    }
                    out.push_str(&table_to_markdown(table));
                    at_line_start = true;
                }
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

/// The text a run covers. Run offsets and lengths are UTF-16 code units (NOTES-L-0001, #174); a
/// boundary inside a surrogate pair moves to the end of that character.
fn run_text(text: &str, start: usize, length: usize) -> &str {
    let end = start.saturating_add(length);
    let (mut from, mut to) = (None, text.len());
    let mut units = 0_usize;
    for (index, ch) in text.char_indices() {
        if from.is_none() && units >= start {
            from = Some(index);
        }
        if units >= end {
            to = index;
            break;
        }
        units += ch.len_utf16();
    }
    let from = from.unwrap_or(text.len());
    text.get(from..to.max(from)).unwrap_or_default()
}

fn table_for_run<'a>(body: &'a NoteBody, run: &super::model::NoteRun) -> Option<&'a [Vec<String>]> {
    let identifier = run.attachment_identifier.as_deref()?;
    body.embedded
        .iter()
        .find(|object| object.attachment_identifier.as_deref() == Some(identifier))
        .and_then(|object| object.table.as_ref())
        .map(|table| table.rows.as_slice())
        .filter(|rows| !rows.is_empty())
}

/// A GitHub-flavoured Markdown table. Apple tables have no header row, but Markdown requires one,
/// so the first row is used.
fn table_to_markdown(rows: &[Vec<String>]) -> String {
    let columns = rows.iter().map(Vec::len).max().unwrap_or(0).max(1);
    let line = |row: &[String]| {
        let cells: Vec<String> = (0..columns)
            .map(|column| {
                row.get(column)
                    .map(|cell| table_cell(cell))
                    .unwrap_or_default()
            })
            .collect();
        format!("| {} |\n", cells.join(" | "))
    };
    let mut out = String::new();
    for (index, row) in rows.iter().enumerate() {
        out.push_str(&line(row));
        if index == 0 {
            out.push_str(&format!("|{}\n", " --- |".repeat(columns)));
        }
    }
    out
}

fn table_cell(cell: &str) -> String {
    cell.trim()
        .replace('\\', "\\\\")
        .replace('|', "\\|")
        .replace('\n', "<br>")
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

    /// Run offsets and lengths are UTF-16 code units (NOTES-L-0001). An emoji is two units and one
    /// `char`, so slicing by `char` shifts every run after it.
    #[test]
    fn runs_after_an_emoji_keep_their_text() {
        let text = "\u{1F600} bold\nHeading";
        let first = "\u{1F600} bold\n".encode_utf16().count();
        let body = NoteBody {
            text: Some(text.to_owned()),
            runs: vec![
                NoteRun {
                    start: 0,
                    length: u32::try_from(first).unwrap_or_default(),
                    paragraph_style: None,
                    font_hints: None,
                    link: None,
                    attachment_identifier: None,
                },
                NoteRun {
                    start: first,
                    length: 7,
                    paragraph_style: Some(ParagraphStyle {
                        style: ParagraphStyleKind::Heading,
                        todo_uuid: None,
                        done: None,
                    }),
                    font_hints: None,
                    link: None,
                    attachment_identifier: None,
                },
            ],
            ..Default::default()
        };
        assert_eq!(body_to_markdown(&body), "\u{1F600} bold\n## Heading\n");
    }

    /// A table renders where its placeholder stands, first row as the header, with `|` escaped and
    /// line breaks kept inside the cell (#168).
    #[test]
    fn tables_render_in_place() {
        let run = |start, length, attachment: Option<&str>| NoteRun {
            start,
            length,
            paragraph_style: None,
            font_hints: None,
            link: None,
            attachment_identifier: attachment.map(str::to_owned),
        };
        let body = NoteBody {
            text: Some("Intro \u{FFFC} outro".to_owned()),
            runs: vec![run(0, 6, None), run(6, 1, Some("T")), run(7, 6, None)],
            embedded: vec![crate::notes::model::EmbeddedObject {
                attachment_identifier: Some("T".to_owned()),
                type_uti: Some(crate::notes::model::TABLE_UTI.to_owned()),
                table: Some(crate::notes::model::EmbeddedTable {
                    rows: vec![
                        vec!["a|b".to_owned(), "c".to_owned()],
                        vec!["two\nlines".to_owned(), String::new()],
                    ],
                    ..Default::default()
                }),
            }],
            ..Default::default()
        };
        assert_eq!(
            body_to_markdown(&body),
            "Intro \n| a\\|b | c |\n| --- | --- |\n| two<br>lines |  |\n outro\n"
        );
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
                attachment_identifier: None,
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
                    attachment_identifier: None,
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
                    attachment_identifier: None,
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
                attachment_identifier: None,
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
