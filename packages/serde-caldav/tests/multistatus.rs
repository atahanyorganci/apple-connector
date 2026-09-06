//! RFC 4791 multistatus parsing: prefixes, every response, and metadata.

use serde_caldav::{
    CalDavCalendarObject, CalDavMultistatus, CalDavResponse, multistatus_to_string,
    parse_calendar_object, parse_multistatus,
};
use serde_icalendar::CalendarEvent;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn fixture(name: &str) -> Result<Vec<u8>, std::io::Error> {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    std::fs::read(path)
}

#[test]
fn default_namespaced_icloud_responses_parse() -> TestResult {
    // The old parser compared the *qualified* name against "calendar-data", so
    // a `C:`-prefixed element never matched and the whole document failed.
    let multistatus = parse_multistatus(&fixture("icloud-multistatus.xml")?)?;
    assert_eq!(multistatus.responses.len(), 3);

    let first = &multistatus.responses[0];
    assert_eq!(
        first.href.as_deref(),
        Some("/1234567/calendars/home/event-one.ics")
    );
    assert_eq!(first.etag.as_deref(), Some("\"C=1@U=abc\""));
    assert_eq!(first.status.as_deref(), Some("HTTP/1.1 200 OK"));

    let object = first
        .calendar_object
        .as_ref()
        .ok_or("expected a calendar object")?;
    assert_eq!(object.event.summary.as_deref(), Some("First event"));
    assert_eq!(
        object.content_type.as_deref(),
        Some("text/calendar; charset=utf-8; component=VEVENT")
    );
    Ok(())
}

#[test]
fn every_response_is_retained_with_its_own_href() -> TestResult {
    // Only the last response used to survive, and every href collapsed to the
    // last one in the document.
    let multistatus = parse_multistatus(&fixture("icloud-multistatus.xml")?)?;

    let summaries: Vec<Option<&str>> = multistatus
        .responses
        .iter()
        .map(|response| {
            response
                .calendar_object
                .as_ref()
                .and_then(|object| object.event.summary.as_deref())
        })
        .collect();
    assert_eq!(
        summaries,
        vec![
            Some("First event"),
            Some("Second event"),
            Some("Third event")
        ]
    );

    let hrefs: Vec<Option<&str>> = multistatus
        .responses
        .iter()
        .map(|response| response.href.as_deref())
        .collect();
    assert_eq!(
        hrefs,
        vec![
            Some("/1234567/calendars/home/event-one.ics"),
            Some("/1234567/calendars/home/event-two.ics"),
            Some("/1234567/calendars/home/event-three.ics"),
        ]
    );

    let etags: Vec<Option<&str>> = multistatus
        .responses
        .iter()
        .map(|response| response.etag.as_deref())
        .collect();
    assert_eq!(
        etags,
        vec![
            Some("\"C=1@U=abc\""),
            Some("\"C=2@U=def\""),
            Some("\"C=3@U=ghi\""),
        ]
    );
    Ok(())
}

#[test]
fn d_and_c_prefixed_responses_parse() -> TestResult {
    let multistatus = parse_multistatus(&fixture("prefixed-multistatus.xml")?)?;
    assert_eq!(multistatus.responses.len(), 2);
    assert_eq!(
        multistatus.responses[0]
            .calendar_object
            .as_ref()
            .and_then(|object| object.event.summary.as_deref()),
        Some("Weekly meeting")
    );
    Ok(())
}

#[test]
fn a_mixed_status_multistatus_keeps_the_not_found_response() -> TestResult {
    let multistatus = parse_multistatus(&fixture("prefixed-multistatus.xml")?)?;
    let missing = multistatus
        .responses
        .iter()
        .find(|response| response.calendar_object.is_none())
        .ok_or("expected a response with no calendar data")?;
    assert_eq!(missing.status.as_deref(), Some("HTTP/1.1 404 Not Found"));
    assert_eq!(
        missing.href.as_deref(),
        Some("/calendars/user/default/missing.ics")
    );
    Ok(())
}

#[test]
fn a_single_object_can_be_pulled_out() -> TestResult {
    let object = parse_calendar_object(&fixture("prefixed-multistatus.xml")?)?;
    assert_eq!(object.event.summary.as_deref(), Some("Weekly meeting"));
    Ok(())
}

fn event(uid: &str, summary: &str) -> CalendarEvent {
    CalendarEvent {
        uid: Some(uid.to_owned()),
        summary: Some(summary.to_owned()),
        ..CalendarEvent::default()
    }
}

#[test]
fn metadata_round_trips_through_the_serializer() -> TestResult {
    let multistatus = CalDavMultistatus {
        responses: vec![
            CalDavResponse {
                href: Some("/calendars/home/a.ics".to_owned()),
                etag: Some("\"etag-a\"".to_owned()),
                status: Some("HTTP/1.1 200 OK".to_owned()),
                calendar_object: Some(CalDavCalendarObject {
                    href: Some("/calendars/home/a.ics".to_owned()),
                    etag: Some("\"etag-a\"".to_owned()),
                    content_type: Some("text/calendar; charset=utf-8".to_owned()),
                    event: event("a@example.com", "Event A"),
                }),
            },
            CalDavResponse {
                href: Some("/calendars/home/b.ics".to_owned()),
                etag: Some("\"etag-b\"".to_owned()),
                status: Some("HTTP/1.1 200 OK".to_owned()),
                calendar_object: Some(CalDavCalendarObject {
                    href: Some("/calendars/home/b.ics".to_owned()),
                    etag: Some("\"etag-b\"".to_owned()),
                    content_type: Some("text/calendar; charset=utf-8".to_owned()),
                    event: event("b@example.com", "Event B"),
                }),
            },
        ],
    };

    let xml = multistatus_to_string(&multistatus)?;
    let decoded = parse_multistatus(xml.as_bytes())?;

    assert_eq!(decoded.responses.len(), 2);
    assert_eq!(decoded.responses[0].etag.as_deref(), Some("\"etag-a\""));
    assert_eq!(
        decoded.responses[1].href.as_deref(),
        Some("/calendars/home/b.ics")
    );
    assert_eq!(
        decoded.responses[1]
            .calendar_object
            .as_ref()
            .and_then(|object| object.event.summary.as_deref()),
        Some("Event B")
    );
    Ok(())
}

#[test]
fn the_serializer_emits_one_document_with_bound_prefixes() -> TestResult {
    let multistatus = CalDavMultistatus {
        responses: vec![CalDavResponse {
            href: Some("/calendars/home/a.ics".to_owned()),
            calendar_object: Some(CalDavCalendarObject {
                href: Some("/calendars/home/a.ics".to_owned()),
                etag: None,
                content_type: Some("text/calendar; charset=utf-8".to_owned()),
                event: event("a@example.com", "Event A"),
            }),
            ..CalDavResponse::default()
        }],
    };

    let xml = multistatus_to_string(&multistatus)?;
    assert_eq!(xml.matches("<?xml").count(), 1, "{xml}");
    assert_eq!(xml.matches("multistatus").count(), 2, "{xml}");
    assert!(xml.contains("xmlns:d=\"DAV:\""), "{xml}");
    assert!(
        xml.contains("xmlns:c=\"urn:ietf:params:xml:ns:caldav\""),
        "{xml}"
    );
    assert!(xml.contains("<c:calendar-data>"), "{xml}");
    Ok(())
}

#[test]
fn documents_this_crate_writes_parse_back_with_a_namespace_aware_reader() -> TestResult {
    let multistatus = CalDavMultistatus {
        responses: vec![CalDavResponse {
            href: Some("/calendars/home/a.ics".to_owned()),
            calendar_object: Some(CalDavCalendarObject {
                href: Some("/calendars/home/a.ics".to_owned()),
                etag: None,
                content_type: None,
                event: event("a@example.com", "Round trip"),
            }),
            ..CalDavResponse::default()
        }],
    };
    let xml = multistatus_to_string(&multistatus)?;
    let decoded = parse_multistatus(xml.as_bytes())?;
    assert_eq!(
        decoded.responses[0]
            .calendar_object
            .as_ref()
            .and_then(|object| object.event.summary.as_deref()),
        Some("Round trip")
    );
    Ok(())
}
