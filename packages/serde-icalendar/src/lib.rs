//! Serde format crate for RFC 5545 iCalendar text.

mod de;
mod error;
mod model;
mod ser;

use std::io::{Read, Write};

pub use error::{Error, Result};
pub use model::{
    Alarm, Attendee, CalendarEvent, EventDateTime, EventStatus, ExtensionBag, Organizer,
};
use serde::{Serialize, de::DeserializeOwned};

/// Serialize a value into an iCalendar string.
pub fn to_string<T>(value: &T) -> Result<String>
where
    T: Serialize,
{
    let mut buffer = Vec::new();
    to_writer(&mut buffer, value)?;
    String::from_utf8(buffer).map_err(|error| Error::Serialize(error.to_string()))
}

/// Deserialize a value from an iCalendar string.
pub fn from_str<T>(input: &str) -> Result<T>
where
    T: DeserializeOwned,
{
    from_slice(input.as_bytes())
}

/// Serialize a value into an iCalendar byte stream.
pub fn to_writer<W, T>(writer: W, value: &T) -> Result<()>
where
    W: Write,
    T: Serialize,
{
    ser::to_writer(writer, value)
}

/// Deserialize a value from an iCalendar byte slice.
pub fn from_slice<T>(input: &[u8]) -> Result<T>
where
    T: DeserializeOwned,
{
    let event = de::parse_ics(input)?;
    serde_json::from_value(serde_json::to_value(&event).map_err(|e| Error::Parse(e.to_string()))?)
        .map_err(|e| Error::Parse(e.to_string()))
}

/// Deserialize a value from a reader containing iCalendar text.
pub fn from_reader<R, T>(mut reader: R) -> Result<T>
where
    R: Read,
    T: DeserializeOwned,
{
    let mut bytes = Vec::new();
    reader
        .read_to_end(&mut bytes)
        .map_err(|error| Error::Parse(error.to_string()))?;
    from_slice(&bytes)
}

#[cfg(test)]
mod tests {
    use super::{CalendarEvent, from_str, to_string};

    #[test]
    fn round_trip_empty_event() {
        let event = CalendarEvent {
            uid: Some("test@example.com".to_owned()),
            summary: Some("Test Event".to_owned()),
            ..CalendarEvent::default()
        };
        let ics = to_string(&event).expect("serialize");
        assert!(ics.contains("BEGIN:VCALENDAR"));
        assert!(ics.contains("SUMMARY:Test Event"));
        let decoded: CalendarEvent = from_str(&ics).expect("deserialize");
        assert_eq!(decoded.summary, event.summary);
        assert_eq!(decoded.uid, event.uid);
    }
}
