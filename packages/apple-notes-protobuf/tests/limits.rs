use std::io::{Cursor, Write};

use apple_notes_protobuf::{
    DecodeError, Limits, decode_note_body, decode_note_body_with_limits,
    decode_plain_text_with_limits,
};
use flate2::{Compression, write::GzEncoder};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn varint(mut value: u64) -> Vec<u8> {
    let mut bytes = Vec::new();
    loop {
        let byte = u8::try_from(value & 0x7f).unwrap_or(0);
        value >>= 7;
        if value == 0 {
            bytes.push(byte);
            return bytes;
        }
        bytes.push(byte | 0x80);
    }
}

fn tag(field: u32, wire: u8) -> Vec<u8> {
    varint((u64::from(field) << 3) | u64::from(wire))
}

fn varint_field(field: u32, value: u64) -> Vec<u8> {
    let mut bytes = tag(field, 0);
    bytes.extend(varint(value));
    bytes
}

fn length_delimited(field: u32, payload: &[u8]) -> Vec<u8> {
    let mut bytes = tag(field, 2);
    bytes.extend(varint(u64::try_from(payload.len()).unwrap_or(u64::MAX)));
    bytes.extend_from_slice(payload);
    bytes
}

/// Build the `document -> version -> string` wrapper the decoder expects.
fn note_document(text: &str, run_lengths: &[u32]) -> Vec<u8> {
    let runs: Vec<Vec<u8>> = run_lengths
        .iter()
        .map(|length| varint_field(1, u64::from(*length)))
        .collect();
    note_document_with_runs(text, &runs)
}

fn note_document_with_runs(text: &str, runs: &[Vec<u8>]) -> Vec<u8> {
    let mut note_string = length_delimited(2, text.as_bytes());
    for run in runs {
        note_string.extend(length_delimited(5, run));
    }
    length_delimited(3, &note_string)
}

/// A `checklist` paragraph style carrying the given todo identifier.
fn checklist_run(length: u32, uuid: u8) -> Vec<u8> {
    let mut todo = length_delimited(1, &[uuid; 16]);
    todo.extend(varint_field(2, 0));

    let mut style = varint_field(1, 103);
    style.extend(length_delimited(5, &todo));

    let mut run = varint_field(1, u64::from(length));
    run.extend(length_delimited(2, &style));
    run
}

fn gzip(data: &[u8]) -> Result<Vec<u8>, std::io::Error> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::best());
    encoder.write_all(data)?;
    encoder.finish()
}

fn limit_name(error: &DecodeError) -> Option<&'static str> {
    match error {
        DecodeError::LimitExceeded { limit, .. } => Some(limit),
        _ => None,
    }
}

#[test]
fn rejects_gzip_bomb_under_default_limits() -> TestResult {
    // 16 MiB of zeros compresses to roughly 16 KiB, far past the ratio budget.
    let bomb = gzip(&vec![0_u8; 16 * 1024 * 1024])?;
    let error = decode_plain_text_with_limits(&bomb, &Limits::default())
        .err()
        .ok_or("expected the gzip bomb to be rejected")?;
    assert_eq!(limit_name(&error), Some("decompressed note body"));
    Ok(())
}

#[test]
fn rejects_decompressed_payload_over_absolute_cap() -> TestResult {
    let payload = gzip(&vec![b'a'; 512 * 1024])?;
    let limits = Limits {
        max_decompressed: 64 * 1024,
        ..Limits::default()
    };
    let error = decode_plain_text_with_limits(&payload, &limits)
        .err()
        .ok_or("expected the absolute decompressed cap to be enforced")?;
    assert_eq!(limit_name(&error), Some("decompressed note body"));
    Ok(())
}

#[test]
fn rejects_excessive_field_count() -> TestResult {
    let document = gzip(&note_document("hello", &[1, 1, 1, 1, 1]))?;
    let limits = Limits {
        max_fields: 4,
        ..Limits::default()
    };
    let error = decode_plain_text_with_limits(&document, &limits)
        .err()
        .ok_or("expected the field budget to be enforced")?;
    assert_eq!(limit_name(&error), Some("protobuf field count"));
    Ok(())
}

#[test]
fn rejects_excessive_message_nesting() -> TestResult {
    let document = gzip(&note_document("hello", &[5]))?;
    let limits = Limits {
        max_message_depth: 1,
        ..Limits::default()
    };
    let error = decode_plain_text_with_limits(&document, &limits)
        .err()
        .ok_or("expected the nesting budget to be enforced")?;
    assert_eq!(limit_name(&error), Some("protobuf nesting depth"));
    Ok(())
}

#[test]
fn rejects_deeply_nested_legacy_plist() -> TestResult {
    let mut value = plist::Value::String("leaf".to_owned());
    for _ in 0..256 {
        value = plist::Value::Array(vec![value]);
    }
    let mut buffer = Cursor::new(Vec::new());
    plist::to_writer_binary(&mut buffer, &value)?;

    let body = decode_note_body(&buffer.into_inner());
    let error = body.decode_error.ok_or("expected a plist depth error")?;
    assert!(
        error.contains("legacy plist nesting depth"),
        "unexpected error: {error}"
    );
    Ok(())
}

#[test]
fn rejects_run_lengths_beyond_the_note_text() -> TestResult {
    let document = gzip(&note_document("abc", &[64]))?;
    let body = decode_note_body(&document);
    let error = body
        .decode_error
        .ok_or("expected an attribute run length error")?;
    assert!(
        error.contains("attribute run lengths"),
        "unexpected error: {error}"
    );
    Ok(())
}

#[test]
fn rejects_excessive_run_count() -> TestResult {
    let lengths = vec![0_u32; 32];
    let document = gzip(&note_document("abc", &lengths))?;
    let limits = Limits {
        max_runs: 8,
        ..Limits::default()
    };
    let body = decode_note_body_with_limits(&document, &limits);
    let error = body.decode_error.ok_or("expected a run count error")?;
    assert!(
        error.contains("attribute run count"),
        "unexpected error: {error}"
    );
    Ok(())
}

#[test]
fn accepts_a_well_formed_document_under_default_limits() -> TestResult {
    let document = gzip(&note_document("hello world", &[5, 6]))?;
    let body = decode_note_body(&document);
    assert_eq!(body.decode_error, None);
    assert_eq!(body.text.as_deref(), Some("hello world"));
    assert_eq!(body.runs.len(), 2);
    assert_eq!(body.runs[0].start, 0);
    assert_eq!(body.runs[1].start, 5);
    Ok(())
}

#[test]
fn runs_are_sliced_in_utf16_code_units() -> TestResult {
    // "😀" is one `char` but two UTF-16 code units, which is how Apple counts
    // run lengths. Slicing by `char` would swallow "b" into the first run and
    // leave the second empty.
    let runs = vec![checklist_run(3, 0xa1), checklist_run(1, 0xb2)];
    let document = gzip(&note_document_with_runs("😀ab", &runs))?;

    let body = decode_note_body(&document);
    assert_eq!(body.decode_error, None);
    assert_eq!(body.runs.len(), 2);
    assert_eq!(body.runs[0].start, 0);
    assert_eq!(body.runs[1].start, 3);

    let texts: Vec<&str> = body
        .checklist_items
        .iter()
        .map(|item| item.text.as_str())
        .collect();
    assert_eq!(texts, vec!["😀a", "b"]);
    Ok(())
}

#[test]
fn real_fixtures_decode_under_default_limits() -> TestResult {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../apple-connector/fixtures/notes/bodies");
    for name in ["plain-text.bin", "checklist.bin"] {
        let path = root.join(name);
        if !path.exists() {
            continue;
        }
        let body = decode_note_body(&std::fs::read(&path)?);
        assert_eq!(body.decode_error, None, "fixture {name} failed to decode");
    }
    Ok(())
}
