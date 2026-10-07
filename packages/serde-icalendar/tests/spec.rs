//! Claims made in `packages/serde-icalendar/docs/SPEC.md` that no other test covered.
//!
//! `#[ignore = "bug: ..."]` tests assert what the spec records as correct and currently fail; each
//! is listed under "Known bugs" in the spec.

use chrono::{TimeZone, Utc};
use serde_icalendar::{Alarm, AlarmTrigger, CalendarEvent, EventDateTime, from_str, to_string};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn calendar(events: &str) -> String {
    format!("BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//spec//EN\r\n{events}END:VCALENDAR\r\n")
}

/// Only the first VEVENT of a calendar is returned.
#[test]
fn only_the_first_vevent_is_parsed() -> TestResult {
    let ics = calendar(
        "BEGIN:VEVENT\r\nUID:first\r\nSUMMARY:First\r\nDTSTART:20240101T120000Z\r\nEND:VEVENT\r\n\
         BEGIN:VEVENT\r\nUID:second\r\nSUMMARY:Second\r\nDTSTART:20240102T120000Z\r\nEND:VEVENT\r\n",
    );
    assert_eq!(from_str(&ics)?.uid.as_deref(), Some("first"));
    Ok(())
}

/// Vendor `X-` properties round-trip through `extensions`; other properties the model has no field
/// for (here `CATEGORIES`) are dropped.
#[test]
fn only_x_properties_survive_outside_the_model() -> TestResult {
    let ics = calendar(
        "BEGIN:VEVENT\r\nUID:spec\r\nDTSTART:20240101T120000Z\r\nCATEGORIES:WORK\r\n\
         X-APPLE-TRAVEL-ADVISORY-BEHAVIOR:AUTOMATIC\r\nEND:VEVENT\r\n",
    );
    let written = to_string(&from_str(&ics)?)?;
    assert!(
        written.contains("X-APPLE-TRAVEL-ADVISORY-BEHAVIOR:AUTOMATIC"),
        "{written}"
    );
    assert!(!written.contains("CATEGORIES"), "{written}");
    Ok(())
}

/// TRIGGER durations are kept as written on parse and validated only when serializing, so a
/// document can parse and then fail to serialize.
#[test]
fn trigger_durations_are_validated_on_write_only() -> TestResult {
    let ics = calendar(
        "BEGIN:VEVENT\r\nUID:spec\r\nDTSTART:20240101T120000Z\r\nBEGIN:VALARM\r\n\
         ACTION:DISPLAY\r\nTRIGGER:-PXM\r\nEND:VALARM\r\nEND:VEVENT\r\n",
    );
    let parsed = from_str(&ics)?;
    assert_eq!(
        parsed
            .alarms
            .first()
            .and_then(|alarm| alarm.trigger.clone()),
        Some(AlarmTrigger::Duration {
            value: "-PXM".to_owned(),
            related: None
        })
    );
    assert!(to_string(&parsed).is_err());

    let written = to_string(&CalendarEvent {
        alarms: vec![Alarm {
            trigger: Some(AlarmTrigger::Duration {
                value: "-PT15M".to_owned(),
                related: None,
            }),
            ..Alarm::default()
        }],
        ..CalendarEvent::default()
    })?;
    assert!(written.contains("TRIGGER:-PT15M"), "{written}");
    Ok(())
}

/// RFC 5545 §3.2.19: every TZID referenced must have a matching VTIMEZONE in the same object.
#[test]
fn zoned_times_are_written_with_their_vtimezone() -> TestResult {
    let start = Utc
        .with_ymd_and_hms(2024, 1, 1, 9, 0, 0)
        .single()
        .ok_or("invalid instant")?;
    let written = to_string(&CalendarEvent {
        uid: Some("spec".to_owned()),
        start: Some(EventDateTime::zoned(start, "Europe/Istanbul")),
        ..CalendarEvent::default()
    })?;
    assert!(written.contains("TZID=Europe/Istanbul"), "{written}");
    assert!(written.contains("BEGIN:VTIMEZONE"), "{written}");
    assert!(written.contains("TZID:Europe/Istanbul"), "{written}");
    Ok(())
}

/// RFC 5545 §3.1: content lines are delimited by CRLF.
#[test]
fn lines_end_with_crlf() -> TestResult {
    let written = to_string(&CalendarEvent {
        uid: Some("spec".to_owned()),
        summary: Some("Spec".to_owned()),
        ..CalendarEvent::default()
    })?;
    let lines = written.split_inclusive('\n').count();
    let crlf = written.matches("\r\n").count();
    assert_eq!(lines, crlf, "{written:?}");
    Ok(())
}

fn instant(
    year: i32,
    month: u32,
    day: u32,
    hour: u32,
) -> Result<chrono::DateTime<Utc>, Box<dyn std::error::Error>> {
    Ok(Utc
        .with_ymd_and_hms(year, month, day, hour, 0, 0)
        .single()
        .ok_or("invalid instant")?)
}

/// A zone with daylight saving gets an observance for each change in the years its date-times fall
/// in, with `DTSTART` in the offset before the change (RFC 5545 §3.6.5).
#[test]
fn vtimezone_observances_follow_the_zone_rules() -> TestResult {
    let written = to_string(&CalendarEvent {
        uid: Some("spec".to_owned()),
        start: Some(EventDateTime::zoned(
            instant(2024, 7, 1, 16)?,
            "America/New_York",
        )),
        ..CalendarEvent::default()
    })?;
    // 2024: EDT starts 10 March 02:00 EST, ends 3 November 02:00 EDT.
    assert!(
        written.contains("BEGIN:DAYLIGHT\r\nDTSTART:20240310T020000\r\nTZOFFSETFROM:-0500\r\nTZOFFSETTO:-0400\r\nTZNAME:EDT\r\nEND:DAYLIGHT"),
        "{written}"
    );
    assert!(
        written.contains("BEGIN:STANDARD\r\nDTSTART:20241103T020000\r\nTZOFFSETFROM:-0400\r\nTZOFFSETTO:-0500\r\nTZNAME:EST\r\nEND:STANDARD"),
        "{written}"
    );
    let vtimezone = written.find("BEGIN:VTIMEZONE").ok_or("no VTIMEZONE")?;
    let vevent = written.find("BEGIN:VEVENT").ok_or("no VEVENT")?;
    assert!(vtimezone < vevent, "{written}");
    Ok(())
}

/// Every distinct TZID gets exactly one VTIMEZONE, and UTC-only events get none.
#[test]
fn one_vtimezone_per_zone_and_none_without_zones() -> TestResult {
    let written = to_string(&CalendarEvent {
        uid: Some("spec".to_owned()),
        start: Some(EventDateTime::zoned(
            instant(2024, 1, 1, 9)?,
            "Europe/Istanbul",
        )),
        end: Some(EventDateTime::zoned(
            instant(2024, 1, 1, 10)?,
            "Europe/Istanbul",
        )),
        exception_dates: vec![EventDateTime::zoned(instant(2024, 2, 1, 9)?, "Asia/Tokyo")],
        ..CalendarEvent::default()
    })?;
    assert_eq!(written.matches("BEGIN:VTIMEZONE").count(), 2, "{written}");
    assert!(written.contains("TZID:Asia/Tokyo\r\n"), "{written}");
    assert!(written.contains("TZID:Europe/Istanbul\r\n"), "{written}");

    let utc = to_string(&CalendarEvent {
        uid: Some("spec".to_owned()),
        start: Some(EventDateTime::utc(instant(2024, 1, 1, 9)?)),
        ..CalendarEvent::default()
    })?;
    assert!(!utc.contains("VTIMEZONE"), "{utc}");
    Ok(())
}

/// A recurring event's observances reach past its listed date-times, where its occurrences fall.
#[test]
fn recurring_events_get_observances_for_later_years() -> TestResult {
    let written = to_string(&CalendarEvent {
        uid: Some("spec".to_owned()),
        start: Some(EventDateTime::zoned(
            instant(2024, 7, 1, 16)?,
            "America/New_York",
        )),
        recurrence_rule: Some("FREQ=YEARLY".to_owned()),
        ..CalendarEvent::default()
    })?;
    assert!(written.contains("DTSTART:20300310T020000"), "{written}");
    Ok(())
}

/// What this crate writes still parses back, zone and instant intact.
#[test]
fn documents_with_a_vtimezone_parse_back() -> TestResult {
    let start = EventDateTime::zoned(instant(2024, 7, 1, 16)?, "America/New_York");
    let written = to_string(&CalendarEvent {
        uid: Some("spec".to_owned()),
        start: Some(start.clone()),
        ..CalendarEvent::default()
    })?;
    assert_eq!(from_str(&written)?.start, Some(start));
    Ok(())
}

/// Observances are found by stepping through the covered years, so the span is bounded.
#[test]
fn a_vtimezone_span_over_a_thousand_years_is_a_limit_error() -> TestResult {
    let error = to_string(&CalendarEvent {
        uid: Some("spec".to_owned()),
        start: Some(EventDateTime::zoned(
            instant(2024, 1, 1, 9)?,
            "Europe/Istanbul",
        )),
        exception_dates: vec![EventDateTime::zoned(
            instant(3100, 1, 1, 9)?,
            "Europe/Istanbul",
        )],
        ..CalendarEvent::default()
    })
    .err()
    .ok_or("a 1076-year span serialized")?;
    assert!(
        matches!(error, serde_icalendar::Error::LimitExceeded { .. }),
        "{error}"
    );
    Ok(())
}
