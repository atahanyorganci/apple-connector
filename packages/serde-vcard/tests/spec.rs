//! Claims made in `packages/serde-vcard/docs/SPEC.md` that no other test covered.
//!
//! `#[ignore = "bug: ..."]` tests assert what the spec records as correct and currently fail; each
//! is listed under "Known bugs" in the spec.

use serde_vcard::{VCard, from_str, parse_vcards, to_string};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn card(body: &str) -> String {
    format!("BEGIN:VCARD\r\nVERSION:4.0\r\nFN:Jane Doe\r\n{body}END:VCARD\r\n")
}

/// The writer always declares vCard 4.0, whatever version was read.
#[test]
fn output_is_always_version_4() -> TestResult {
    let input = "BEGIN:VCARD\r\nVERSION:3.0\r\nFN:Jane Doe\r\nEND:VCARD\r\n";
    let written = to_string(&from_str(input)?)?;
    assert!(written.contains("VERSION:4.0"), "{written}");
    assert!(!written.contains("VERSION:3.0"), "{written}");
    Ok(())
}

/// `from_str` returns the first card; `parse_vcards` returns all of them.
#[test]
fn from_str_returns_only_the_first_card() -> TestResult {
    let input = format!(
        "{}{}",
        card(""),
        "BEGIN:VCARD\r\nVERSION:4.0\r\nFN:John Roe\r\nEND:VCARD\r\n"
    );
    assert_eq!(
        from_str(&input)?.formatted_name.as_deref(),
        Some("Jane Doe")
    );
    assert_eq!(parse_vcards(&input)?.len(), 2);
    Ok(())
}

/// `IMPP` is read into `social_profiles` and written back as `X-SOCIALPROFILE`: the property name
/// does not survive a round trip.
#[test]
fn impp_is_written_back_as_x_socialprofile() -> TestResult {
    let parsed = from_str(&card("IMPP:xmpp:jane@example.com\r\n"))?;
    assert_eq!(parsed.social_profiles.len(), 1);
    let written = to_string(&parsed)?;
    assert!(written.contains("X-SOCIALPROFILE"), "{written}");
    assert!(!written.contains("IMPP"), "{written}");
    Ok(())
}

/// RFC 6350 §3.2: lines are delimited by CRLF.
#[test]
fn lines_end_with_crlf() -> TestResult {
    let written = to_string(&VCard {
        formatted_name: Some("Jane Doe".to_owned()),
        ..VCard::default()
    })?;
    let lines = written.split_inclusive('\n').count();
    let crlf = written.matches("\r\n").count();
    assert_eq!(lines, crlf, "{written:?}");
    Ok(())
}

/// A `BDAY` the date parser cannot read — here RFC 6350's year-less `--MMDD` — must survive
/// somewhere: either as a birthday or in `unknown`. Today it is dropped silently.
#[test]
#[ignore = "bug: an unparseable BDAY is dropped (SPEC.md, Known bugs)"]
fn an_unparseable_birthday_is_not_dropped() -> TestResult {
    let parsed = from_str(&card("BDAY:--0415\r\n"))?;
    let kept = parsed.birthday.is_some()
        || parsed
            .unknown
            .iter()
            .any(|property| property.name.eq_ignore_ascii_case("BDAY"));
    assert!(kept, "{parsed:?}");
    Ok(())
}

/// A `BEGIN:VCARD` inside an open card and an `END:VCARD` with no open card are errors.
#[test]
fn unbalanced_card_markers_are_errors() {
    assert!(parse_vcards("BEGIN:VCARD\r\nFN:A\r\nBEGIN:VCARD\r\nFN:B\r\nEND:VCARD\r\n").is_err());
    assert!(parse_vcards("FN:A\r\nEND:VCARD\r\n").is_err());
}

/// Long lines are folded so no physical line exceeds 75 octets, a continuation starts with one
/// space, no character is split, and the value unfolds back unchanged.
#[test]
fn long_lines_fold_at_75_octets_without_splitting_characters() -> TestResult {
    let note = "çok uzun bir not ".repeat(12);
    let written = to_string(&VCard {
        formatted_name: Some("Jane Doe".to_owned()),
        note: Some(note.clone()),
        ..VCard::default()
    })?;
    for line in written.lines() {
        assert!(line.len() <= 75, "{} octets: {line:?}", line.len());
    }
    assert!(
        written.lines().any(|line| line.starts_with(' ')),
        "nothing was folded"
    );
    assert_eq!(from_str(&written)?.note.as_deref(), Some(note.as_str()));
    Ok(())
}
