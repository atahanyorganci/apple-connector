//! Embedded iCalendar text must be XML-escaped, not written verbatim.

use serde_caldav::{CalDavCalendarObject, from_str, to_string};
use serde_icalendar::CalendarEvent;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn object_with(summary: &str, description: &str) -> CalDavCalendarObject {
    CalDavCalendarObject {
        href: Some("/calendars/home/event.ics".to_owned()),
        etag: None,
        content_type: Some("text/calendar; charset=utf-8".to_owned()),
        event: CalendarEvent {
            uid: Some("escaping@example.com".to_owned()),
            summary: Some(summary.to_owned()),
            description: Some(description.to_owned()),
            ..CalendarEvent::default()
        },
    }
}

/// A minimal well-formedness check: a bare `&` or `<` inside element content
/// makes quick-xml's reader fail, which is exactly what a real client would do.
fn assert_well_formed(xml: &str) -> TestResult {
    let mut reader = quick_xml::Reader::from_str(xml);
    let mut buffer = Vec::new();
    loop {
        match reader.read_event_into(&mut buffer)? {
            quick_xml::events::Event::Eof => break,
            _ => buffer.clear(),
        }
    }
    Ok(())
}

#[test]
fn ampersands_and_angle_brackets_round_trip() -> TestResult {
    let object = object_with("Q&A <draft>", "Costs < 5 & > 1");
    let xml = to_string(&object)?;

    assert_well_formed(&xml)?;
    // The raw characters must not appear inside element content unescaped.
    assert!(xml.contains("&amp;"), "ampersand was not escaped:\n{xml}");
    assert!(
        xml.contains("&lt;"),
        "angle bracket was not escaped:\n{xml}"
    );

    let decoded: CalDavCalendarObject = from_str(&xml)?;
    assert_eq!(decoded.event.summary.as_deref(), Some("Q&A <draft>"));
    assert_eq!(
        decoded.event.description.as_deref(),
        Some("Costs < 5 & > 1")
    );
    Ok(())
}

#[test]
fn quotes_and_apostrophes_survive() -> TestResult {
    let object = object_with("The \"big\" review", "Ada's agenda");
    let xml = to_string(&object)?;

    assert_well_formed(&xml)?;
    let decoded: CalDavCalendarObject = from_str(&xml)?;
    assert_eq!(decoded.event.summary.as_deref(), Some("The \"big\" review"));
    assert_eq!(decoded.event.description.as_deref(), Some("Ada's agenda"));
    Ok(())
}

#[test]
fn plain_text_is_unchanged() -> TestResult {
    let object = object_with("Design review", "Nothing special here");
    let xml = to_string(&object)?;

    assert_well_formed(&xml)?;
    let decoded: CalDavCalendarObject = from_str(&xml)?;
    assert_eq!(decoded.event.summary.as_deref(), Some("Design review"));
    // No double-escaping: an unescaped document gains no entities.
    assert!(!xml.contains("&amp;amp;"), "{xml}");
    Ok(())
}
