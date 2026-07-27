//! Serde format crate for RFC 4791 CalDAV XML.

mod de;
mod error;
mod model;
mod ser;
pub mod xmlns;

use std::io::{Read, Write};

pub use de::{parse_multistatus, parse_xml};
pub use error::{Error, Result};
pub use model::{CalDavCalendarObject, CalDavCalendarResource, CalDavMultistatus, CalDavResponse};
use serde::{Serialize, de::DeserializeOwned};

/// Serialize a value into CalDAV XML with embedded ICS calendar-data.
pub fn to_string<T>(value: &T) -> Result<String>
where
    T: Serialize,
{
    let mut buffer = Vec::new();
    to_writer(&mut buffer, value)?;
    String::from_utf8(buffer).map_err(|error| Error::Serialize(error.to_string()))
}

/// Deserialize a value from CalDAV XML.
pub fn from_str<T>(input: &str) -> Result<T>
where
    T: DeserializeOwned,
{
    from_slice(input.as_bytes())
}

/// Serialize a value into a CalDAV XML writer.
pub fn to_writer<W, T>(writer: W, value: &T) -> Result<()>
where
    W: Write,
    T: serde::Serialize,
{
    ser::to_writer(writer, value)
}

/// Deserialize a value from a CalDAV XML byte slice.
pub fn from_slice<T>(input: &[u8]) -> Result<T>
where
    T: DeserializeOwned,
{
    let object = de::parse_xml(input)?;
    serde_json::from_value(serde_json::to_value(object).map_err(|e| Error::Parse(e.to_string()))?)
        .map_err(|e| Error::Parse(e.to_string()))
}

/// Deserialize a value from a reader containing CalDAV XML.
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
    use serde_icalendar::CalendarEvent;

    use super::{CalDavCalendarObject, from_str, to_string};

    #[test]
    fn stub_round_trip_caldav_object() {
        let object = CalDavCalendarObject {
            href: Some("/calendars/home/event.ics".to_owned()),
            etag: None,
            content_type: Some("text/calendar; charset=utf-8".to_owned()),
            event: CalendarEvent::default(),
        };
        let xml = to_string(&object).expect("serialize");
        assert!(xml.contains("multistatus"));
        assert!(xml.contains("calendar-data"));
        let decoded: CalDavCalendarObject = from_str(&xml).expect("deserialize");
        assert_eq!(decoded.href, object.href);
    }
}
