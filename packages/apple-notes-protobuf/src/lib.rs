mod protobuf;

use std::io::Read;

use flate2::read::GzDecoder;
use plist::Value;
use protobuf::{all_bytes, fields_by_number, first_bytes, parse_message};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeError {
    Empty,
    InvalidGzip(String),
    InvalidProtobuf(String),
    InvalidPlist(String),
}

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => write!(f, "note body is empty"),
            Self::InvalidGzip(message) => write!(f, "invalid gzip payload: {message}"),
            Self::InvalidProtobuf(message) => write!(f, "invalid protobuf payload: {message}"),
            Self::InvalidPlist(message) => write!(f, "invalid legacy plist payload: {message}"),
        }
    }
}

impl std::error::Error for DecodeError {}

#[derive(Debug, Clone, PartialEq, Eq)]
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

impl ParagraphStyleKind {
    fn from_raw(value: u32) -> Self {
        match value {
            0 => Self::Title,
            1 => Self::Heading,
            4 => Self::Monospace,
            100 => Self::BulletList,
            101 => Self::DashList,
            102 => Self::NumberedList,
            103 => Self::Checklist,
            other => Self::Unknown(other),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParagraphStyle {
    pub style: ParagraphStyleKind,
    pub todo_uuid: Option<String>,
    pub done: Option<bool>,
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

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DecodedNoteBody {
    pub text: Option<String>,
    pub runs: Vec<NoteRun>,
    pub checklist_items: Vec<ChecklistItem>,
    pub embedded: Vec<EmbeddedObject>,
    pub decode_error: Option<String>,
}

struct NoteString {
    text: String,
    runs: Vec<AttributeRun>,
    embedded: Vec<EmbeddedObject>,
}

struct AttributeRun {
    length: u32,
    paragraph_style: Option<ParagraphStyle>,
    font_hints: Option<u32>,
    link: Option<String>,
}

const BPLIST00_MAGIC: &[u8; 8] = b"bplist00";
const GZIP_MAGIC: &[u8; 2] = b"\x1f\x8b";

/// Decompress a gzip-wrapped Notes protobuf blob (or legacy bplist) and extract plain text.
pub fn decode_plain_text(data: &[u8]) -> Result<String, DecodeError> {
    if data.is_empty() {
        return Err(DecodeError::Empty);
    }

    if is_legacy_bplist(data) {
        return decode_legacy_bplist(data);
    }

    let decompressed = decompress_gzip(data)?;
    let note_string = parse_note_string_from_document(&decompressed)
        .map_err(DecodeError::InvalidProtobuf)?;
    Ok(note_string.text)
}

/// Decode a note body into structured text, formatting runs, checklist items, and attachments.
pub fn decode_note_body(data: &[u8]) -> DecodedNoteBody {
    if data.is_empty() {
        return DecodedNoteBody {
            decode_error: Some(DecodeError::Empty.to_string()),
            ..Default::default()
        };
    }

    let result = if is_legacy_bplist(data) {
        decode_legacy_bplist(data).map(|text| NoteString {
            text,
            runs: Vec::new(),
            embedded: Vec::new(),
        })
    } else {
        decompress_gzip(data).and_then(|decompressed| {
            parse_note_string_from_document(&decompressed).map_err(DecodeError::InvalidProtobuf)
        })
    };

    match result {
        Ok(note_string) => build_decoded_body(note_string),
        Err(error) => DecodedNoteBody {
            decode_error: Some(error.to_string()),
            ..Default::default()
        },
    }
}

fn char_range(text: &str, start: usize, length: u32) -> (String, usize) {
    let end = start.saturating_add(length as usize);
    let chunk: String = text.chars().skip(start).take(length as usize).collect();
    (chunk, end)
}

fn build_decoded_body(note_string: NoteString) -> DecodedNoteBody {
    let mut offset = 0;
    let mut runs = Vec::with_capacity(note_string.runs.len());
    for run in &note_string.runs {
        runs.push(NoteRun {
            start: offset,
            length: run.length,
            paragraph_style: run.paragraph_style.clone(),
            font_hints: run.font_hints,
            link: run.link.clone(),
        });
        offset = offset.saturating_add(run.length as usize);
    }

    DecodedNoteBody {
        text: Some(note_string.text.clone()),
        runs,
        checklist_items: extract_checklist_items(&note_string.text, &note_string.runs),
        embedded: note_string.embedded,
        decode_error: None,
    }
}

fn is_legacy_bplist(data: &[u8]) -> bool {
    data.len() >= BPLIST00_MAGIC.len() && data.starts_with(BPLIST00_MAGIC)
}

fn decompress_gzip(data: &[u8]) -> Result<Vec<u8>, DecodeError> {
    if data.len() < GZIP_MAGIC.len() || !data.starts_with(GZIP_MAGIC) {
        return Err(DecodeError::InvalidGzip(
            "payload does not start with gzip magic".to_owned(),
        ));
    }

    let mut decoder = GzDecoder::new(data);
    let mut decompressed = Vec::new();
    decoder
        .read_to_end(&mut decompressed)
        .map_err(|error| DecodeError::InvalidGzip(error.to_string()))?;
    Ok(decompressed)
}

fn decode_legacy_bplist(data: &[u8]) -> Result<String, DecodeError> {
    let value: Value = plist::from_bytes(data).map_err(|error| DecodeError::InvalidPlist(error.to_string()))?;
    extract_text_from_plist(&value).ok_or_else(|| {
        DecodeError::InvalidPlist("legacy plist did not contain note text".to_owned())
    })
}

fn extract_text_from_plist(value: &Value) -> Option<String> {
    match value {
        Value::String(text) if !text.trim().is_empty() => Some(text.clone()),
        Value::Dictionary(dict) => {
            for key in ["NS.string", "Text", "text", "content", "ZCONTENT"] {
                if let Some(Value::String(text)) = dict.get(key)
                    && !text.trim().is_empty()
                {
                    return Some(text.clone());
                }
            }

            if let Some(Value::Dictionary(root)) = dict.get("NS.objects") {
                return extract_text_from_plist(&Value::Dictionary(root.clone()));
            }

            if let Some(Value::Array(objects)) = dict.get("$objects") {
                for object in objects {
                    if let Some(text) = extract_text_from_plist(object) {
                        return Some(text);
                    }
                }
            }

            dict.values().find_map(extract_text_from_plist)
        }
        Value::Array(values) => values.iter().find_map(extract_text_from_plist),
        _ => None,
    }
}

fn parse_note_string_from_document(document: &[u8]) -> Result<NoteString, String> {
    let fields = parse_message(document)?;
    let version_data = extract_version_data(&fields)?;
    parse_note_string(&version_data)
}

fn extract_version_data(fields: &[protobuf::Field]) -> Result<Vec<u8>, String> {
    for version_blob in all_bytes(fields, 2) {
        if let Some(data) = version_data_from_blob(version_blob) {
            return Ok(data);
        }
    }

    if let Some(data) = first_bytes(fields, 3) {
        return Ok(data.to_vec());
    }

    Err("document wrapper did not contain version data".to_owned())
}

fn version_data_from_blob(version_blob: &[u8]) -> Option<Vec<u8>> {
    let version_fields = parse_message(version_blob).ok()?;
    first_bytes(&version_fields, 3).map(|data| data.to_vec())
}

fn parse_note_string(data: &[u8]) -> Result<NoteString, String> {
    let fields = parse_message(data)?;

    let text = first_bytes(&fields, 2)
        .and_then(|bytes| std::str::from_utf8(bytes).ok())
        .map(str::to_owned)
        .ok_or_else(|| "string message missing field 2 text".to_owned())?;

    let mut runs = Vec::new();
    let mut embedded = Vec::new();

    for run_blob in all_bytes(&fields, 5) {
        let parsed = parse_attribute_run(run_blob)?;
        if let Some(attachment) = parsed.attachment {
            embedded.push(attachment);
        }
        runs.push(AttributeRun {
            length: parsed.length,
            paragraph_style: parsed.paragraph_style,
            font_hints: parsed.font_hints,
            link: parsed.link,
        });
    }

    Ok(NoteString {
        text,
        runs,
        embedded,
    })
}

struct ParsedAttributeRun {
    length: u32,
    paragraph_style: Option<ParagraphStyle>,
    font_hints: Option<u32>,
    link: Option<String>,
    attachment: Option<EmbeddedObject>,
}

fn parse_attribute_run(data: &[u8]) -> Result<ParsedAttributeRun, String> {
    let fields = parse_message(data)?;

    let length = fields_by_number(&fields, 1)
        .find_map(|field| field.varint)
        .and_then(|value| u32::try_from(value).ok())
        .unwrap_or(0);

    let paragraph_style = fields_by_number(&fields, 2)
        .find_map(|field| field.bytes.as_deref())
        .and_then(parse_paragraph_style);

    let font_hints = fields_by_number(&fields, 5)
        .find_map(|field| field.varint)
        .and_then(|value| u32::try_from(value).ok());

    let link = fields_by_number(&fields, 9)
        .find_map(|field| field.bytes.as_deref())
        .and_then(|bytes| std::str::from_utf8(bytes).ok())
        .map(str::to_owned);

    let attachment = fields_by_number(&fields, 12)
        .find_map(|field| field.bytes.as_deref())
        .and_then(parse_attachment_info);

    Ok(ParsedAttributeRun {
        length,
        paragraph_style,
        font_hints,
        link,
        attachment,
    })
}

fn parse_paragraph_style(data: &[u8]) -> Option<ParagraphStyle> {
    let fields = parse_message(data).ok()?;
    let style = fields_by_number(&fields, 1)
        .find_map(|field| field.varint)
        .and_then(|value| u32::try_from(value).ok())
        .map(ParagraphStyleKind::from_raw)?;

    let todo = fields_by_number(&fields, 5)
        .find_map(|field| field.bytes.as_deref())
        .and_then(parse_todo);

    Some(ParagraphStyle {
        style,
        todo_uuid: todo.as_ref().map(|(uuid, _)| uuid.clone()),
        done: todo.map(|(_, done)| done),
    })
}

fn parse_todo(data: &[u8]) -> Option<(String, bool)> {
    let fields = parse_message(data).ok()?;
    let uuid_bytes = first_bytes(&fields, 1)?;
    let uuid = Uuid::from_bytes(uuid_bytes.try_into().ok()?).to_string();
    let done = fields_by_number(&fields, 2)
        .find_map(|field| field.varint)
        .map(|value| value != 0)
        .unwrap_or(false);
    Some((uuid, done))
}

fn parse_attachment_info(data: &[u8]) -> Option<EmbeddedObject> {
    let fields = parse_message(data).ok()?;
    let attachment_identifier = first_bytes(&fields, 1)
        .and_then(|bytes| std::str::from_utf8(bytes).ok())
        .map(str::to_owned);
    let type_uti = first_bytes(&fields, 2)
        .and_then(|bytes| std::str::from_utf8(bytes).ok())
        .map(str::to_owned);

    if attachment_identifier.is_none() && type_uti.is_none() {
        return None;
    }

    Some(EmbeddedObject {
        attachment_identifier,
        type_uti,
    })
}

fn extract_checklist_items(text: &str, runs: &[AttributeRun]) -> Vec<ChecklistItem> {
    let mut items = Vec::new();
    let mut offset = 0;
    let mut current_id: Option<String> = None;
    let mut current_done = false;
    let mut current_parts: Vec<String> = Vec::new();

    for run in runs {
        let (chunk, next_offset) = char_range(text, offset, run.length);
        offset = next_offset;

        if run
            .paragraph_style
            .as_ref()
            .is_some_and(|style| matches!(style.style, ParagraphStyleKind::Checklist))
            && let Some(id) = run.paragraph_style.as_ref().and_then(|style| style.todo_uuid.clone())
        {
            if current_id.as_ref() != Some(&id) {
                flush_checklist_item(
                    &mut items,
                    &mut current_id,
                    &mut current_done,
                    &mut current_parts,
                );
                current_id = Some(id);
                current_done = run
                    .paragraph_style
                    .as_ref()
                    .and_then(|style| style.done)
                    .unwrap_or(false);
            }
            current_parts.push(chunk);
        } else {
            flush_checklist_item(
                &mut items,
                &mut current_id,
                &mut current_done,
                &mut current_parts,
            );
        }
    }

    flush_checklist_item(
        &mut items,
        &mut current_id,
        &mut current_done,
        &mut current_parts,
    );
    items
}

fn flush_checklist_item(
    items: &mut Vec<ChecklistItem>,
    current_id: &mut Option<String>,
    current_done: &mut bool,
    current_parts: &mut Vec<String>,
) {
    if let Some(id) = current_id.take() {
        let joined = current_parts.join("").trim().to_owned();
        current_parts.clear();
        if !joined.is_empty() {
            items.push(ChecklistItem {
                id,
                text: joined,
                done: *current_done,
            });
        }
    }
    *current_done = false;
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use super::{decode_note_body, decode_plain_text, ParagraphStyleKind};

    fn fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../apple-connector/fixtures/notes/bodies")
            .join(name)
    }

    fn read_fixture(name: &str) -> Vec<u8> {
        fs::read(fixture_path(name)).unwrap_or_else(|error| {
            panic!("failed to read fixture {name}: {error}");
        })
    }

    #[test]
    fn rejects_empty_payload() {
        assert!(matches!(
            decode_plain_text(&[]),
            Err(super::DecodeError::Empty)
        ));
    }

    #[test]
    fn decodes_plain_text_fixture() {
        let data = read_fixture("plain-text.bin");
        assert_eq!(data.len(), 605);

        let text = decode_plain_text(&data).expect("plain-text fixture should decode");
        assert!(text.contains("IBAN"), "decoded text: {text}");
    }

    #[test]
    fn decodes_plain_text_fixture_structured() {
        let data = read_fixture("plain-text.bin");
        let body = decode_note_body(&data);

        assert!(body.decode_error.is_none(), "{:?}", body.decode_error);
        assert!(body.text.as_ref().is_some_and(|text| text.contains("IBAN")));
        assert!(!body.runs.is_empty());
        assert!(body
            .runs
            .iter()
            .any(|run| run.paragraph_style.as_ref().is_some_and(|style| {
                matches!(style.style, ParagraphStyleKind::Title)
            })));
    }

    #[test]
    fn decodes_checklist_fixture() {
        let path = fixture_path("checklist.bin");
        if !path.exists() {
            return;
        }

        let data = read_fixture("checklist.bin");
        let body = decode_note_body(&data);

        assert!(body.decode_error.is_none(), "{:?}", body.decode_error);
        assert!(body.text.as_ref().is_some_and(|text| !text.is_empty()));
        assert!(!body.checklist_items.is_empty());
        assert!(body.checklist_items.iter().any(|item| {
            item.text.contains("Simulacra") || item.text.contains("Algorithms")
        }));
    }

    #[test]
    fn detects_invalid_gzip() {
        let err = decode_plain_text(b"not gzip").unwrap_err();
        assert!(matches!(err, super::DecodeError::InvalidGzip(_)));
    }

    #[test]
    fn protobuf_varint_roundtrip() {
        let encoded = [0x96, 0x01];
        let (value, next) = super::protobuf::read_varint(&encoded, 0).unwrap();
        assert_eq!(value, 150);
        assert_eq!(next, 2);
    }
}
