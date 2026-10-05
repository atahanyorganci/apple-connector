//! Claims made in `packages/serde-caldav/docs/SPEC.md` that no other test covered.

use serde_caldav::from_str;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const EVENT: &str = "BEGIN:VCALENDAR\nVERSION:2.0\nPRODID:-//spec//EN\nBEGIN:VEVENT\nUID:spec\nDTSTART:20240101T120000Z\nEND:VEVENT\nEND:VCALENDAR";

/// One `calendar-data` payload that is not valid iCalendar fails the whole document, not just its
/// response.
#[test]
fn an_invalid_payload_fails_the_whole_document() -> TestResult {
    let xml = format!(
        r#"<d:multistatus xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav">
  <d:response><d:href>/good.ics</d:href><d:propstat><d:prop>
    <c:calendar-data>{EVENT}</c:calendar-data></d:prop><d:status>HTTP/1.1 200 OK</d:status></d:propstat></d:response>
  <d:response><d:href>/bad.ics</d:href><d:propstat><d:prop>
    <c:calendar-data>not icalendar</c:calendar-data></d:prop><d:status>HTTP/1.1 200 OK</d:status></d:propstat></d:response>
</d:multistatus>"#
    );
    assert!(from_str(&xml).is_err());
    Ok(())
}

/// The response's `href` is the response's own `DAV:href`, not an href nested inside a property
/// value such as `current-user-principal`.
#[test]
fn a_nested_href_does_not_replace_the_response_href() -> TestResult {
    let xml = format!(
        r#"<d:multistatus xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav">
  <d:response><d:href>/calendars/home/event.ics</d:href><d:propstat><d:prop>
    <d:current-user-principal><d:href>/principals/me/</d:href></d:current-user-principal>
    <c:calendar-data>{EVENT}</c:calendar-data></d:prop><d:status>HTTP/1.1 200 OK</d:status></d:propstat></d:response>
</d:multistatus>"#
    );
    let parsed = from_str(&xml)?;
    let href = parsed.responses.first().and_then(|r| r.href.as_deref());
    assert_eq!(href, Some("/calendars/home/event.ics"));
    Ok(())
}
