//! EXDATE parsing: no panics on malformed input, no silently dropped values.

use serde_icalendar::{CalendarEvent, from_str};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn ics_with(lines: &str) -> String {
    format!(
        "BEGIN:VCALENDAR\r\n\
         VERSION:2.0\r\n\
         PRODID:-//apple-connector//test//EN\r\n\
         BEGIN:VEVENT\r\n\
         UID:exdate@example.com\r\n\
         DTSTAMP:20240101T000000Z\r\n\
         DTSTART:20240101T120000Z\r\n\
         SUMMARY:Exception test\r\n\
         {lines}\r\n\
         END:VEVENT\r\n\
         END:VCALENDAR\r\n"
    )
}

fn parse(lines: &str) -> Result<CalendarEvent, serde_icalendar::Error> {
    from_str::<CalendarEvent>(&ics_with(lines))
}

#[test]
fn truncated_exdate_is_an_error_not_a_panic() -> TestResult {
    // Thirteen characters after the trailing Z. The old fixed-offset slicing
    // indexed bytes 13..15 and panicked.
    let error = parse("EXDATE:20240101T1200Z")
        .err()
        .ok_or("expected a parse error for a truncated EXDATE")?;
    assert!(
        error.to_string().contains("EXDATE"),
        "unexpected error: {error}"
    );
    Ok(())
}

#[test]
fn multi_byte_exdate_is_an_error_not_a_panic() -> TestResult {
    // Slicing at fixed byte offsets used to split this character in half.
    let error = parse("EXDATE:2024Ω101T120000Z")
        .err()
        .ok_or("expected a parse error for a multi-byte EXDATE")?;
    assert!(
        error.to_string().contains("EXDATE"),
        "unexpected error: {error}"
    );
    Ok(())
}

#[test]
fn empty_exdate_is_an_error() -> TestResult {
    parse("EXDATE:not-a-date")
        .err()
        .ok_or("expected a parse error for a non-date EXDATE")?;
    Ok(())
}

#[test]
fn comma_separated_values_are_all_retained() -> TestResult {
    let event = parse("EXDATE:20240101T120000Z,20240102T120000Z,20240103T120000Z")?;
    assert_eq!(event.exception_dates.len(), 3);
    assert_eq!(
        event.exception_dates[1].timestamp.to_rfc3339(),
        "2024-01-02T12:00:00+00:00"
    );
    Ok(())
}

#[test]
fn date_only_values_are_marked_all_day() -> TestResult {
    let event = parse("EXDATE;VALUE=DATE:20240101,20240102")?;
    assert_eq!(event.exception_dates.len(), 2);
    assert!(event.exception_dates.iter().all(|date| date.all_day));
    Ok(())
}

#[test]
fn tzid_is_preserved_and_converted_to_utc() -> TestResult {
    // Europe/Istanbul is permanently UTC+03, so noon local is 09:00Z.
    let event = parse("EXDATE;TZID=Europe/Istanbul:20240701T120000")?;
    let exception = event
        .exception_dates
        .first()
        .ok_or("expected one exception date")?;
    assert_eq!(exception.tzid.as_deref(), Some("Europe/Istanbul"));
    assert_eq!(
        exception.timestamp.to_rfc3339(),
        "2024-07-01T09:00:00+00:00"
    );
    Ok(())
}

#[test]
fn tzid_applies_to_every_value_in_the_list() -> TestResult {
    let event = parse("EXDATE;TZID=Europe/Istanbul:20240701T120000,20240702T120000")?;
    assert_eq!(event.exception_dates.len(), 2);
    assert!(
        event
            .exception_dates
            .iter()
            .all(|date| date.tzid.as_deref() == Some("Europe/Istanbul"))
    );
    Ok(())
}

#[test]
fn unknown_time_zones_are_reported() -> TestResult {
    let error = parse("EXDATE;TZID=Mars/Olympus:20240701T120000")
        .err()
        .ok_or("expected an unknown time zone error")?;
    assert!(
        error.to_string().contains("unknown time zone"),
        "unexpected error: {error}"
    );
    Ok(())
}

#[test]
fn a_dst_gap_is_reported_rather_than_shifted() -> TestResult {
    // 02:30 on 2024-03-10 does not exist in America/New_York.
    let error = parse("EXDATE;TZID=America/New_York:20240310T023000")
        .err()
        .ok_or("expected a non-existent local time error")?;
    assert!(
        error.to_string().contains("does not exist"),
        "unexpected error: {error}"
    );
    Ok(())
}

#[test]
fn events_without_exdate_parse_cleanly() -> TestResult {
    let event = parse("DESCRIPTION:no exceptions here")?;
    assert!(event.exception_dates.is_empty());
    Ok(())
}
