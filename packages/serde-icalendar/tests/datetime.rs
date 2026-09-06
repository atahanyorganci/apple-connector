//! DTSTART/DTEND semantics: zone conversion, TZID round-trip, all-day ends.

use serde_icalendar::{CalendarEvent, EventDateTime, from_str, to_string};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn ics_with(lines: &str) -> String {
    format!(
        "BEGIN:VCALENDAR\r\n\
         VERSION:2.0\r\n\
         PRODID:-//apple-connector//test//EN\r\n\
         BEGIN:VEVENT\r\n\
         UID:datetime@example.com\r\n\
         DTSTAMP:20240101T000000Z\r\n\
         SUMMARY:Date time test\r\n\
         {lines}\r\n\
         END:VEVENT\r\n\
         END:VCALENDAR\r\n"
    )
}

fn parse(lines: &str) -> Result<CalendarEvent, serde_icalendar::Error> {
    from_str(&ics_with(lines))
}

#[test]
fn tzid_wall_time_converts_to_the_right_instant() -> TestResult {
    // Europe/Istanbul is permanently UTC+03. Reading the wall time as if it
    // were already UTC put this event three hours late.
    let event = parse("DTSTART;TZID=Europe/Istanbul:20240701T120000")?;
    let start = event.start.ok_or("expected a start")?;
    assert_eq!(start.timestamp().to_rfc3339(), "2024-07-01T09:00:00+00:00");
    assert_eq!(start.tzid(), Some("Europe/Istanbul"));
    Ok(())
}

#[test]
fn tzid_conversion_respects_dst() -> TestResult {
    // New York is UTC-05 in January and UTC-04 in July.
    let winter = parse("DTSTART;TZID=America/New_York:20240115T090000")?
        .start
        .ok_or("expected a start")?;
    assert_eq!(winter.timestamp().to_rfc3339(), "2024-01-15T14:00:00+00:00");

    let summer = parse("DTSTART;TZID=America/New_York:20240715T090000")?
        .start
        .ok_or("expected a start")?;
    assert_eq!(summer.timestamp().to_rfc3339(), "2024-07-15T13:00:00+00:00");
    Ok(())
}

#[test]
fn tzid_survives_a_round_trip() -> TestResult {
    let event = parse("DTSTART;TZID=Europe/Istanbul:20240701T120000")?;
    let ics = to_string(&event)?;
    assert!(
        ics.contains("TZID=Europe/Istanbul"),
        "TZID was dropped on serialize:\n{ics}"
    );
    assert!(
        ics.contains("20240701T120000"),
        "local wall time was not preserved:\n{ics}"
    );

    let reparsed = from_str(&ics)?;
    assert_eq!(reparsed.start, event.start);
    Ok(())
}

#[test]
fn utc_values_round_trip_as_utc() -> TestResult {
    let event = parse("DTSTART:20240701T120000Z")?;
    assert!(matches!(event.start, Some(EventDateTime::Utc { .. })));

    let ics = to_string(&event)?;
    assert!(ics.contains("DTSTART:20240701T120000Z"), "{ics}");
    Ok(())
}

#[test]
fn floating_times_stay_floating() -> TestResult {
    // A value with neither a Z nor a TZID has no zone at all, and used to be
    // indistinguishable from UTC once parsed.
    let event = parse("DTSTART:20240701T120000")?;
    assert!(matches!(event.start, Some(EventDateTime::Floating { .. })));

    let ics = to_string(&event)?;
    assert!(ics.contains("DTSTART:20240701T120000"), "{ics}");
    assert!(!ics.contains("DTSTART:20240701T120000Z"), "{ics}");
    Ok(())
}

#[test]
fn all_day_end_is_exclusive_and_round_trips() -> TestResult {
    // RFC 5545 3.8.2.2: a DATE-valued DTEND is exclusive, so a one-day event on
    // 1 January ends on 2 January. The end must not be shifted on the way out.
    let event = parse("DTSTART;VALUE=DATE:20240101\r\nDTEND;VALUE=DATE:20240102")?;
    let start = event.start.clone().ok_or("expected a start")?;
    let end = event.end.clone().ok_or("expected an end")?;
    assert!(start.is_all_day());
    assert!(end.is_all_day());

    let ics = to_string(&event)?;
    assert!(ics.contains("DTSTART;VALUE=DATE:20240101"), "{ics}");
    assert!(ics.contains("DTEND;VALUE=DATE:20240102"), "{ics}");

    let reparsed = from_str(&ics)?;
    assert_eq!(reparsed.start, event.start);
    assert_eq!(reparsed.end, event.end);
    Ok(())
}

#[test]
fn multi_day_all_day_events_keep_their_span() -> TestResult {
    let event = parse("DTSTART;VALUE=DATE:20240101\r\nDTEND;VALUE=DATE:20240105")?;
    let ics = to_string(&event)?;
    assert!(ics.contains("DTEND;VALUE=DATE:20240105"), "{ics}");
    Ok(())
}

#[test]
fn every_exception_date_is_serialized() -> TestResult {
    // EXDATE went through append_property, which writes into a map keyed by
    // property name, so only the last exception survived.
    let event = parse("DTSTART:20240101T120000Z\r\nEXDATE:20240102T120000Z,20240103T120000Z")?;
    assert_eq!(event.exception_dates.len(), 2);

    let ics = to_string(&event)?;
    assert!(ics.contains("20240102T120000Z"), "{ics}");
    assert!(ics.contains("20240103T120000Z"), "{ics}");

    let reparsed = from_str(&ics)?;
    assert_eq!(reparsed.exception_dates.len(), 2);
    Ok(())
}

#[test]
fn zoned_exception_dates_round_trip_with_their_zone() -> TestResult {
    let event = parse("DTSTART:20240101T120000Z\r\nEXDATE;TZID=Europe/Istanbul:20240701T120000")?;
    let ics = to_string(&event)?;
    assert!(ics.contains("TZID=Europe/Istanbul"), "{ics}");

    let reparsed = from_str(&ics)?;
    assert_eq!(reparsed.exception_dates, event.exception_dates);
    Ok(())
}

#[test]
fn unknown_start_time_zones_are_reported() -> TestResult {
    let error = parse("DTSTART;TZID=Mars/Olympus:20240701T120000")
        .err()
        .ok_or("expected an unknown time zone error")?;
    assert!(
        error.to_string().contains("unknown time zone"),
        "unexpected error: {error}"
    );
    Ok(())
}
