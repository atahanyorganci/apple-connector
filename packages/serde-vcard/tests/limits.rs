//! Reader entry points enforce a documented ceiling.

use std::io::Cursor;

use serde_vcard::{Error, MAX_INPUT_BYTES, VCard, from_reader, from_reader_with_limit, to_string};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn card_bytes() -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let card = VCard {
        formatted_name: Some("Ada Lovelace".to_owned()),
        ..VCard::default()
    };
    Ok(to_string(&card)?.into_bytes())
}

#[test]
fn input_within_the_limit_parses() -> TestResult {
    let bytes = card_bytes()?;
    let card = from_reader(Cursor::new(bytes))?;
    assert_eq!(card.formatted_name.as_deref(), Some("Ada Lovelace"));
    Ok(())
}

#[test]
fn input_over_the_limit_is_a_structured_error() -> TestResult {
    let bytes = card_bytes()?;
    let error = from_reader_with_limit(Cursor::new(bytes), 8)
        .err()
        .ok_or("expected an input size limit error")?;
    assert!(
        matches!(
            error,
            Error::LimitExceeded { limit, max, .. } if limit == "vCard input size" && max == 8
        ),
        "unexpected error: {error}"
    );
    Ok(())
}

#[test]
fn the_default_limit_is_documented() {
    // Named so the ceiling is discoverable rather than buried in the reader.
    assert_eq!(MAX_INPUT_BYTES, 16 * 1024 * 1024);
}

#[test]
fn a_reader_that_never_ends_does_not_exhaust_memory() -> TestResult {
    // `io::Repeat` is infinite; an uncapped read_to_end would never return.
    let error = from_reader_with_limit(std::io::repeat(b'A'), 4096)
        .err()
        .ok_or("expected an input size limit error")?;
    assert!(
        matches!(error, Error::LimitExceeded { .. }),
        "unexpected error: {error}"
    );
    Ok(())
}
