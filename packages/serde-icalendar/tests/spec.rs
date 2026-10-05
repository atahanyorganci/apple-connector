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
#[ignore = "bug: TZID is written without a VTIMEZONE (SPEC.md, Known bugs)"]
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
