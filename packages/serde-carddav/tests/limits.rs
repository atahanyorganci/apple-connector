//! Reader entry points enforce a documented ceiling.

use serde_carddav::{Error, MAX_INPUT_BYTES, from_reader_with_limit};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn input_over_the_limit_is_a_structured_error() -> TestResult {
    let xml = vec![b'x'; 512];
    let error = from_reader_with_limit(std::io::Cursor::new(xml), 8)
        .err()
        .ok_or("expected an input size limit error")?;
    assert!(
        matches!(
            error,
            Error::LimitExceeded { limit, max, .. } if limit == "CardDAV input size" && max == 8
        ),
        "unexpected error: {error}"
    );
    Ok(())
}

#[test]
fn a_reader_that_never_ends_does_not_exhaust_memory() -> TestResult {
    let error = from_reader_with_limit(std::io::repeat(b'<'), 4096)
        .err()
        .ok_or("expected an input size limit error")?;
    assert!(
        matches!(error, Error::LimitExceeded { .. }),
        "unexpected error: {error}"
    );
    Ok(())
}

#[test]
fn the_default_limit_is_documented() {
    assert_eq!(MAX_INPUT_BYTES, 64 * 1024 * 1024);
}
