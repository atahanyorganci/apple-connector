//! Serde format crate for RFC 5545 iCalendar text.

mod de;
mod error;
mod model;
mod ser;

use std::io::{Read, Write};

pub use de::parse_ics;
pub use error::{Error, Result};
pub use model::{
    Alarm, AlarmTrigger, Attendee, CalendarEvent, CalendarUserType, EventDateTime, EventStatus,
    ExtensionBag, Organizer, ParticipationStatus, Role, TriggerRelation,
};

/// Maximum number of bytes [`from_reader`] will read.
pub const MAX_INPUT_BYTES: usize = 16 * 1024 * 1024;

/// Serialize an event into an iCalendar string.
pub fn to_string(event: &CalendarEvent) -> Result<String> {
    let mut buffer = Vec::new();
    to_writer(&mut buffer, event)?;
    String::from_utf8(buffer).map_err(|error| Error::Serialize(error.to_string()))
}

/// Serialize an event into an iCalendar byte stream.
pub fn to_writer<W: Write>(writer: W, event: &CalendarEvent) -> Result<()> {
    ser::to_writer(writer, event)
}

/// Parse the first VEVENT in an iCalendar string.
pub fn from_str(input: &str) -> Result<CalendarEvent> {
    from_slice(input.as_bytes())
}

/// Parse the first VEVENT in an iCalendar byte slice.
pub fn from_slice(input: &[u8]) -> Result<CalendarEvent> {
    de::parse_ics(input)
}

/// Parse the first VEVENT from a reader, reading at most [`MAX_INPUT_BYTES`].
pub fn from_reader<R: Read>(reader: R) -> Result<CalendarEvent> {
    from_reader_with_limit(reader, MAX_INPUT_BYTES)
}

/// Parse the first VEVENT from a reader, reading at most `limit` bytes.
pub fn from_reader_with_limit<R: Read>(reader: R, limit: usize) -> Result<CalendarEvent> {
    let ceiling = u64::try_from(limit).unwrap_or(u64::MAX).saturating_add(1);
    let mut bytes = Vec::new();
    reader
        .take(ceiling)
        .read_to_end(&mut bytes)
        .map_err(|error| Error::Parse(error.to_string()))?;
    if bytes.len() > limit {
        return Err(Error::Parse(format!(
            "iCalendar input exceeds the {limit} byte limit"
        )));
    }
    from_slice(&bytes)
}

#[cfg(test)]
mod tests {
    use super::{CalendarEvent, from_str, to_string};

    #[test]
    fn round_trip_empty_event() -> Result<(), Box<dyn std::error::Error>> {
        let event = CalendarEvent {
            uid: Some("test@example.com".to_owned()),
            summary: Some("Test Event".to_owned()),
            ..CalendarEvent::default()
        };
        let ics = to_string(&event)?;
        assert!(ics.contains("BEGIN:VCALENDAR"));
        assert!(ics.contains("SUMMARY:Test Event"));
        let decoded = from_str(&ics)?;
        assert_eq!(decoded.summary, event.summary);
        assert_eq!(decoded.uid, event.uid);
        Ok(())
    }
}
