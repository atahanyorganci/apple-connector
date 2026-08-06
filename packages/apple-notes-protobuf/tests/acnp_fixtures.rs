//! Fixture tests sourced from apple_cloud_notes_parser `exported_blobs`.
//! Expectations mirror `spec/base_classes/apple_note.rb` (see
//! `fixtures/notes/bodies/acnp/manifest.toml`).

use std::{fs, path::PathBuf};

use apple_notes_protobuf::decode_note_body;

fn acnp_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../apple-connector/fixtures/notes/bodies/acnp")
}

fn read_acnp_fixture(name: &str) -> Vec<u8> {
    let path = acnp_dir().join(name);
    fs::read(&path).unwrap_or_else(|error| {
        panic!("failed to read ACNP fixture {}: {error}", path.display());
    })
}

fn assert_text_contains(file: &str, needles: &[&str]) {
    let data = read_acnp_fixture(file);
    let body = decode_note_body(&data);
    assert!(
        body.decode_error.is_none(),
        "{file}: decode_error={:?}",
        body.decode_error
    );
    let text = body
        .text
        .as_deref()
        .unwrap_or_else(|| panic!("{file}: expected decoded text"));
    for needle in needles {
        assert!(
            text.contains(needle),
            "{file}: missing {needle:?} in text:\n{text}"
        );
    }
}

#[test]
fn acnp_simple_note_plaintext() {
    let data = read_acnp_fixture("simple_note_protobuf_gzipped.bin");
    let body = decode_note_body(&data);
    assert!(body.decode_error.is_none(), "{:?}", body.decode_error);
    assert_eq!(body.text.as_deref(), Some("Title"));
}

#[test]
fn acnp_color_formatting() {
    assert_text_contains(
        "color_formatting_gzipped.bin",
        &[
            "Red",
            "Blue",
            "Checklist, unchecked, red in the middle",
            "bold red",
        ],
    );
}

#[test]
fn acnp_wide_characters() {
    assert_text_contains(
        "wide_characters_gzipped.bin",
        &["但他還是希望", "體驗看法覺得很大"],
    );
}

#[test]
fn acnp_html_literal() {
    assert_text_contains("html_gzipped.bin", &["<HTML>"]);
}

#[test]
fn acnp_emoji_formatting_1() {
    assert_text_contains("emoji_formatting_1_gzipped.bin", &["🚀", "🧑‍💻"]);
}

#[test]
fn acnp_emoji_formatting_2() {
    assert_text_contains(
        "emoji_formatting_2_gzipped.bin",
        &["projects", "Graphic Designer"],
    );
}

#[test]
fn acnp_emoji_formatting_3() {
    assert_text_contains(
        "emoji_formatting_3_gzipped.bin",
        &["bold", "italic", "underlined", "🖤"],
    );
}

#[test]
fn acnp_url_link_runs() -> Result<(), Box<dyn std::error::Error>> {
    let data = read_acnp_fixture("url_gzipped.bin");
    let body = decode_note_body(&data);
    assert!(body.decode_error.is_none(), "{:?}", body.decode_error);
    let text = body.text.as_deref().ok_or("missing text")?;
    assert!(text.contains("Jim Nettles"), "text: {text}");
    assert!(text.contains("his older brother"), "text: {text}");
    let link_runs = body.runs.iter().filter(|run| run.link.is_some()).count();
    assert!(
        link_runs >= 1,
        "expected at least one link run, got {link_runs}; runs={:?}",
        body.runs
    );
    Ok(())
}

#[test]
fn acnp_text_decorations() {
    assert_text_contains(
        "text_decorations_gzipped.bin",
        &[
            "Bold body",
            "Italic body",
            "Bold italic body",
            "Underlined body",
            "Strikethrough body",
        ],
    );
}

#[test]
fn acnp_block_quotes() {
    assert_text_contains(
        "block_quotes_gzipped.bin",
        &[
            "This is a block quote",
            "This is monostyled",
            "This is a monostyled block quote",
        ],
    );
}

#[test]
#[ignore = "upstream Ruby list-indent HTML test is pending (xit); keep fixture for future work"]
fn acnp_list_indents() {
    assert_text_contains(
        "list_indents_gzipped.bin",
        &["Dotted list second indent", "Dashed list indent 2"],
    );
}

#[test]
#[ignore = "embedded table protobuf not yet decoded by apple-notes-protobuf"]
fn acnp_table_simple() {
    let data = read_acnp_fixture("table_gzipped.bin");
    let body = decode_note_body(&data);
    assert!(body.decode_error.is_none(), "{:?}", body.decode_error);
}

#[test]
#[ignore = "embedded table protobuf not yet decoded by apple-notes-protobuf"]
fn acnp_table_formats() {
    let data = read_acnp_fixture("table_formats_gzipped.bin");
    let body = decode_note_body(&data);
    assert!(body.decode_error.is_none(), "{:?}", body.decode_error);
}

#[test]
#[ignore = "embedded table protobuf not yet decoded by apple-notes-protobuf"]
fn acnp_table_right_to_left() {
    let data = read_acnp_fixture("right_to_left_table_gzipped.bin");
    let body = decode_note_body(&data);
    assert!(body.decode_error.is_none(), "{:?}", body.decode_error);
}
