mod protobuf;

use std::io::Read;

use flate2::read::GzDecoder;
use plist::Value;
use protobuf::{Budget, all_bytes, fields_by_number, first_bytes, parse_message};
use uuid::Uuid;

/// Decompressed payloads below this size are never rejected by the compression
/// ratio budget, so ordinary short notes cannot trip it.
const RATIO_FLOOR: usize = 1024 * 1024;

/// Budgets applied while decoding one note body.
///
/// The defaults are sized well above anything Apple Notes produces in practice;
/// they exist to bound hostile or corrupt payloads, not to shape valid ones.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// Absolute ceiling on the gzip-decompressed document, in bytes.
    pub max_decompressed: usize,
    /// Maximum decompressed-to-compressed size ratio, applied only once the
    /// decompressed payload exceeds one mebibyte.
    pub max_compression_ratio: usize,
    /// Maximum number of protobuf fields decoded across the whole document.
    pub max_fields: usize,
    /// Maximum protobuf sub-message nesting depth.
    pub max_message_depth: usize,
    /// Maximum traversal depth inside a legacy binary plist body.
    pub max_plist_depth: usize,
    /// Maximum number of attribute runs in one note.
    pub max_runs: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_decompressed: 64 * 1024 * 1024,
            max_compression_ratio: 200,
            max_fields: 262_144,
            max_message_depth: 16,
            max_plist_depth: 64,
            max_runs: 131_072,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeError {
    Empty,
    InvalidGzip(String),
    InvalidProtobuf(String),
    InvalidPlist(String),
    LimitExceeded {
        limit: &'static str,
        actual: usize,
        max: usize,
    },
}

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => write!(f, "note body is empty"),
            Self::InvalidGzip(message) => write!(f, "invalid gzip payload: {message}"),
            Self::InvalidProtobuf(message) => write!(f, "invalid protobuf payload: {message}"),
            Self::InvalidPlist(message) => write!(f, "invalid legacy plist payload: {message}"),
            Self::LimitExceeded { limit, actual, max } => {
                write!(f, "{limit} limit exceeded: {actual} exceeds maximum {max}")
            }
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
    /// Offset of this run in the note text, in UTF-16 code units.
    pub start: usize,
    /// Length of this run, in UTF-16 code units, as Apple encodes it.
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

/// Decompress a gzip-wrapped Notes protobuf blob (or legacy bplist) and extract
/// plain text, using the default [`Limits`].
pub fn decode_plain_text(data: &[u8]) -> Result<String, DecodeError> {
    decode_plain_text_with_limits(data, &Limits::default())
}

/// Decompress a gzip-wrapped Notes protobuf blob (or legacy bplist) and extract
/// plain text under caller-supplied budgets.
pub fn decode_plain_text_with_limits(data: &[u8], limits: &Limits) -> Result<String, DecodeError> {
    if data.is_empty() {
        return Err(DecodeError::Empty);
    }

    if is_legacy_bplist(data) {
        return decode_legacy_bplist(data, limits);
    }

    let decompressed = decompress_gzip(data, limits)?;
    let note_string = parse_note_string_from_document(&decompressed, limits)?;
    Ok(note_string.text)
}

/// Decode a note body into structured text, formatting runs, checklist items,
/// and attachments, using the default [`Limits`].
pub fn decode_note_body(data: &[u8]) -> DecodedNoteBody {
    decode_note_body_with_limits(data, &Limits::default())
}

/// Decode a note body under caller-supplied budgets.
///
/// Decoding failures, limit overruns included, are reported through
/// [`DecodedNoteBody::decode_error`] rather than as a `Result`, so one corrupt
/// note never fails a page of otherwise valid ones.
pub fn decode_note_body_with_limits(data: &[u8], limits: &Limits) -> DecodedNoteBody {
    if data.is_empty() {
        return DecodedNoteBody {
            decode_error: Some(DecodeError::Empty.to_string()),
            ..Default::default()
        };
    }

    let result = if is_legacy_bplist(data) {
        decode_legacy_bplist(data, limits).map(|text| NoteString {
            text,
            runs: Vec::new(),
            embedded: Vec::new(),
        })
    } else {
        decompress_gzip(data, limits)
            .and_then(|decompressed| parse_note_string_from_document(&decompressed, limits))
    };

    match result.and_then(|note_string| build_decoded_body(note_string, limits)) {
        Ok(body) => body,
        Err(error) => DecodedNoteBody {
            decode_error: Some(error.to_string()),
            ..Default::default()
        },
    }
}

fn build_decoded_body(
    note_string: NoteString,
    limits: &Limits,
) -> Result<DecodedNoteBody, DecodeError> {
    if note_string.runs.len() > limits.max_runs {
        return Err(DecodeError::LimitExceeded {
            limit: "attribute run count",
            actual: note_string.runs.len(),
            max: limits.max_runs,
        });
    }

    // Apple encodes run lengths in UTF-16 code units, so every offset in this
    // function is measured the same way.
    let text_length = note_string.text.encode_utf16().count();
    let mut offset = 0_usize;
    let mut runs = Vec::with_capacity(note_string.runs.len());
    for run in &note_string.runs {
        offset = offset.saturating_add(run.length as usize);
        if offset > text_length {
            return Err(DecodeError::InvalidProtobuf(format!(
                "attribute run lengths cover {offset} UTF-16 code units but the note text has {text_length}"
            )));
        }
        runs.push(NoteRun {
            start: offset - run.length as usize,
            length: run.length,
            paragraph_style: run.paragraph_style.clone(),
            font_hints: run.font_hints,
            link: run.link.clone(),
        });
    }

    Ok(DecodedNoteBody {
        checklist_items: extract_checklist_items(&note_string.text, &note_string.runs),
        text: Some(note_string.text),
        runs,
        embedded: note_string.embedded,
        decode_error: None,
    })
}

fn is_legacy_bplist(data: &[u8]) -> bool {
    data.len() >= BPLIST00_MAGIC.len() && data.starts_with(BPLIST00_MAGIC)
}

fn decompress_gzip(data: &[u8], limits: &Limits) -> Result<Vec<u8>, DecodeError> {
    if data.len() < GZIP_MAGIC.len() || !data.starts_with(GZIP_MAGIC) {
        return Err(DecodeError::InvalidGzip(
            "payload does not start with gzip magic".to_owned(),
        ));
    }

    let ratio_cap = data
        .len()
        .saturating_mul(limits.max_compression_ratio)
        .max(RATIO_FLOOR);
    let cap = limits.max_decompressed.min(ratio_cap);
    let take_limit = u64::try_from(cap).unwrap_or(u64::MAX).saturating_add(1);

    let mut decompressed = Vec::new();
    GzDecoder::new(data)
        .take(take_limit)
        .read_to_end(&mut decompressed)
        .map_err(|error| DecodeError::InvalidGzip(error.to_string()))?;

    if decompressed.len() > cap {
        return Err(DecodeError::LimitExceeded {
            limit: "decompressed note body",
            actual: decompressed.len(),
            max: cap,
        });
    }
    Ok(decompressed)
}

fn decode_legacy_bplist(data: &[u8], limits: &Limits) -> Result<String, DecodeError> {
    let value: Value =
        plist::from_bytes(data).map_err(|error| DecodeError::InvalidPlist(error.to_string()))?;
    extract_text_from_plist(&value, 0, limits)?.ok_or_else(|| {
        DecodeError::InvalidPlist("legacy plist did not contain note text".to_owned())
    })
}

fn extract_text_from_plist(
    value: &Value,
    depth: usize,
    limits: &Limits,
) -> Result<Option<String>, DecodeError> {
    if depth > limits.max_plist_depth {
        return Err(DecodeError::LimitExceeded {
            limit: "legacy plist nesting depth",
            actual: depth,
            max: limits.max_plist_depth,
        });
    }

    match value {
        Value::String(text) if !text.trim().is_empty() => Ok(Some(text.clone())),
        Value::Dictionary(dict) => {
            for key in ["NS.string", "Text", "text", "content", "ZCONTENT"] {
                if let Some(Value::String(text)) = dict.get(key)
                    && !text.trim().is_empty()
                {
                    return Ok(Some(text.clone()));
                }
            }

            if let Some(Value::Dictionary(root)) = dict.get("NS.objects") {
                return extract_text_from_plist(
                    &Value::Dictionary(root.clone()),
                    depth + 1,
                    limits,
                );
            }

            if let Some(Value::Array(objects)) = dict.get("$objects") {
                for object in objects {
                    if let Some(text) = extract_text_from_plist(object, depth + 1, limits)? {
                        return Ok(Some(text));
                    }
                }
            }

            for nested in dict.values() {
                if let Some(text) = extract_text_from_plist(nested, depth + 1, limits)? {
                    return Ok(Some(text));
                }
            }
            Ok(None)
        }
        Value::Array(values) => {
            for nested in values {
                if let Some(text) = extract_text_from_plist(nested, depth + 1, limits)? {
                    return Ok(Some(text));
                }
            }
            Ok(None)
        }
        _ => Ok(None),
    }
}

fn parse_note_string_from_document(
    document: &[u8],
    limits: &Limits,
) -> Result<NoteString, DecodeError> {
    let mut budget = Budget::new(limits.max_fields, limits.max_message_depth);
    let fields = parse_message(document, &mut budget, 0)?;
    let version_data = extract_version_data(&fields, &mut budget)?;
    parse_note_string(version_data, &mut budget)
}

fn extract_version_data<'a>(
    fields: &[protobuf::Field<'a>],
    budget: &mut Budget,
) -> Result<&'a [u8], DecodeError> {
    for version_blob in all_bytes(fields, 2) {
        let version_fields = parse_message(version_blob, budget, 1)?;
        if let Some(data) = first_bytes(&version_fields, 3) {
            return Ok(data);
        }
    }

    if let Some(data) = first_bytes(fields, 3) {
        return Ok(data);
    }

    Err(DecodeError::InvalidProtobuf(
        "document wrapper did not contain version data".to_owned(),
    ))
}

fn parse_note_string(data: &[u8], budget: &mut Budget) -> Result<NoteString, DecodeError> {
    let fields = parse_message(data, budget, 2)?;

    let text = first_bytes(&fields, 2)
        .and_then(|bytes| std::str::from_utf8(bytes).ok())
        .map(str::to_owned)
        .ok_or_else(|| {
            DecodeError::InvalidProtobuf("string message missing field 2 text".to_owned())
        })?;

    let mut runs = Vec::new();
    let mut embedded = Vec::new();

    for run_blob in all_bytes(&fields, 5) {
        let parsed = parse_attribute_run(run_blob, budget)?;
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

fn parse_attribute_run(
    data: &[u8],
    budget: &mut Budget,
) -> Result<ParsedAttributeRun, DecodeError> {
    let fields = parse_message(data, budget, 3)?;

    let length = fields_by_number(&fields, 1)
        .find_map(|field| field.varint)
        .and_then(|value| u32::try_from(value).ok())
        .unwrap_or(0);

    let paragraph_style = match fields_by_number(&fields, 2).find_map(|field| field.bytes) {
        Some(bytes) => parse_paragraph_style(bytes, budget)?,
        None => None,
    };

    let font_hints = fields_by_number(&fields, 5)
        .find_map(|field| field.varint)
        .and_then(|value| u32::try_from(value).ok());

    let link = fields_by_number(&fields, 9)
        .find_map(|field| field.bytes)
        .and_then(|bytes| std::str::from_utf8(bytes).ok())
        .map(str::to_owned);

    let attachment = match fields_by_number(&fields, 12).find_map(|field| field.bytes) {
        Some(bytes) => parse_attachment_info(bytes, budget)?,
        None => None,
    };

    Ok(ParsedAttributeRun {
        length,
        paragraph_style,
        font_hints,
        link,
        attachment,
    })
}

fn parse_paragraph_style(
    data: &[u8],
    budget: &mut Budget,
) -> Result<Option<ParagraphStyle>, DecodeError> {
    let fields = parse_message(data, budget, 4)?;
    let Some(style) = fields_by_number(&fields, 1)
        .find_map(|field| field.varint)
        .and_then(|value| u32::try_from(value).ok())
        .map(ParagraphStyleKind::from_raw)
    else {
        return Ok(None);
    };

    let todo = match fields_by_number(&fields, 5).find_map(|field| field.bytes) {
        Some(bytes) => parse_todo(bytes, budget)?,
        None => None,
    };

    Ok(Some(ParagraphStyle {
        style,
        todo_uuid: todo.as_ref().map(|(uuid, _)| uuid.clone()),
        done: todo.map(|(_, done)| done),
    }))
}

fn parse_todo(data: &[u8], budget: &mut Budget) -> Result<Option<(String, bool)>, DecodeError> {
    let fields = parse_message(data, budget, 5)?;
    let Some(uuid_bytes) = first_bytes(&fields, 1) else {
        return Ok(None);
    };
    let Ok(uuid_bytes) = <[u8; 16]>::try_from(uuid_bytes) else {
        return Ok(None);
    };
    let uuid = Uuid::from_bytes(uuid_bytes).to_string();
    let done = fields_by_number(&fields, 2)
        .find_map(|field| field.varint)
        .map(|value| value != 0)
        .unwrap_or(false);
    Ok(Some((uuid, done)))
}

fn parse_attachment_info(
    data: &[u8],
    budget: &mut Budget,
) -> Result<Option<EmbeddedObject>, DecodeError> {
    let fields = parse_message(data, budget, 4)?;
    let attachment_identifier = first_bytes(&fields, 1)
        .and_then(|bytes| std::str::from_utf8(bytes).ok())
        .map(str::to_owned);
    let type_uti = first_bytes(&fields, 2)
        .and_then(|bytes| std::str::from_utf8(bytes).ok())
        .map(str::to_owned);

    if attachment_identifier.is_none() && type_uti.is_none() {
        return Ok(None);
    }

    Ok(Some(EmbeddedObject {
        attachment_identifier,
        type_uti,
    }))
}

/// Forward cursor over a string, advancing by UTF-16 code units.
///
/// Apple measures attribute runs in UTF-16, so slicing by `char` misaligns every
/// run that follows an astral character such as an emoji. The cursor never
/// splits a surrogate pair.
struct Utf16Cursor<'a> {
    text: &'a str,
    offset: usize,
}

impl<'a> Utf16Cursor<'a> {
    fn new(text: &'a str) -> Self {
        Self { text, offset: 0 }
    }

    fn take(&mut self, units: usize) -> &'a str {
        let start = self.offset;
        let mut remaining = units;
        let mut end = start;
        for character in self.text.get(start..).unwrap_or_default().chars() {
            let width = character.len_utf16();
            if width > remaining {
                break;
            }
            remaining -= width;
            end += character.len_utf8();
        }
        self.offset = end;
        self.text.get(start..end).unwrap_or_default()
    }
}

/// Walk the note text once, slicing it into per-run chunks, and group
/// consecutive checklist runs that share a todo identifier.
fn extract_checklist_items(text: &str, runs: &[AttributeRun]) -> Vec<ChecklistItem> {
    let mut items = Vec::new();
    let mut cursor = Utf16Cursor::new(text);
    let mut current_id: Option<String> = None;
    let mut current_done = false;
    let mut current_parts: Vec<String> = Vec::new();

    for run in runs {
        let chunk = cursor.take(run.length as usize).to_owned();

        if run
            .paragraph_style
            .as_ref()
            .is_some_and(|style| matches!(style.style, ParagraphStyleKind::Checklist))
            && let Some(id) = run
                .paragraph_style
                .as_ref()
                .and_then(|style| style.todo_uuid.clone())
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

    use super::{ParagraphStyleKind, decode_note_body, decode_plain_text};

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
    fn decodes_plain_text_fixture() -> Result<(), Box<dyn std::error::Error>> {
        let data = read_fixture("plain-text.bin");
        assert_eq!(data.len(), 605);

        let text = decode_plain_text(&data)?;
        assert!(text.contains("IBAN"), "decoded text: {text}");
        Ok(())
    }

    #[test]
    fn decodes_plain_text_fixture_structured() {
        let data = read_fixture("plain-text.bin");
        let body = decode_note_body(&data);

        assert!(body.decode_error.is_none(), "{:?}", body.decode_error);
        assert!(body.text.as_ref().is_some_and(|text| text.contains("IBAN")));
        assert!(!body.runs.is_empty());
        assert!(body.runs.iter().any(|run| {
            run.paragraph_style
                .as_ref()
                .is_some_and(|style| matches!(style.style, ParagraphStyleKind::Title))
        }));
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
        assert!(
            body.checklist_items.iter().any(|item| {
                item.text.contains("Simulacra") || item.text.contains("Algorithms")
            })
        );
    }

    #[test]
    fn detects_invalid_gzip() -> Result<(), Box<dyn std::error::Error>> {
        let err = decode_plain_text(b"not gzip")
            .err()
            .ok_or("expected invalid gzip error")?;
        assert!(matches!(err, super::DecodeError::InvalidGzip(_)));
        Ok(())
    }

    #[test]
    fn protobuf_varint_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
        let encoded = [0x96, 0x01];
        let (value, next) = super::protobuf::read_varint(&encoded, 0)?;
        assert_eq!(value, 150);
        assert_eq!(next, 2);
        Ok(())
    }
}
