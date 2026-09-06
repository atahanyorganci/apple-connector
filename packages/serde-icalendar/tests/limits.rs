//! Reader entry points enforce a documented ceiling.

use std::io::Cursor;

use serde_icalendar::{
    CalendarEvent, Error, MAX_INPUT_BYTES, from_reader, from_reader_with_limit, to_string,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn event_bytes() -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let event = CalendarEvent {
        uid: Some("limits@example.com".to_owned()),
        summary: Some("Bounded".to_owned()),
        ..CalendarEvent::default()
    };
    Ok(to_string(&event)?.into_bytes())
}

#[test]
fn input_within_the_limit_parses() -> TestResult {
    let event = from_reader(Cursor::new(event_bytes()?))?;
    assert_eq!(event.summary.as_deref(), Some("Bounded"));
    Ok(())
}

#[test]
fn input_over_the_limit_is_a_structured_error() -> TestResult {
    let error = from_reader_with_limit(Cursor::new(event_bytes()?), 8)
        .err()
        .ok_or("expected an input size limit error")?;
    assert!(
        matches!(
            error,
            Error::LimitExceeded { limit, max, .. } if limit == "iCalendar input size" && max == 8
        ),
        "unexpected error: {error}"
    );
    Ok(())
}

#[test]
fn a_reader_that_never_ends_does_not_exhaust_memory() -> TestResult {
    let error = from_reader_with_limit(std::io::repeat(b'A'), 4096)
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
    assert_eq!(MAX_INPUT_BYTES, 16 * 1024 * 1024);
}
