//! Serde format crate for RFC 4791 CalDAV XML.

mod de;
mod error;
mod model;
mod ser;
pub mod xmlns;

use std::io::{Read, Write};

pub use de::{parse_calendar_object, parse_multistatus};
pub use error::{Error, Result};
pub use model::{CalDavCalendarObject, CalDavCalendarResource, CalDavMultistatus, CalDavResponse};
pub use ser::{calendar_object_to_string, multistatus_to_string};

/// Maximum number of bytes [`from_reader`] will read.
pub const MAX_INPUT_BYTES: usize = 64 * 1024 * 1024;

/// Serialize a multistatus into CalDAV XML.
pub fn to_string(multistatus: &CalDavMultistatus) -> Result<String> {
    multistatus_to_string(multistatus)
}

/// Serialize a multistatus into a CalDAV XML writer.
pub fn to_writer<W: Write>(mut writer: W, multistatus: &CalDavMultistatus) -> Result<()> {
    writer
        .write_all(to_string(multistatus)?.as_bytes())
        .map_err(|error| Error::Serialize(error.to_string()))
}

/// Parse a RFC 4791 multistatus document.
pub fn from_str(input: &str) -> Result<CalDavMultistatus> {
    from_slice(input.as_bytes())
}

/// Parse a RFC 4791 multistatus document.
pub fn from_slice(input: &[u8]) -> Result<CalDavMultistatus> {
    de::parse_multistatus(input)
}

/// Parse a multistatus from a reader, reading at most [`MAX_INPUT_BYTES`].
pub fn from_reader<R: Read>(reader: R) -> Result<CalDavMultistatus> {
    from_reader_with_limit(reader, MAX_INPUT_BYTES)
}

/// Parse a multistatus from a reader, reading at most `limit` bytes.
pub fn from_reader_with_limit<R: Read>(reader: R, limit: usize) -> Result<CalDavMultistatus> {
    let ceiling = u64::try_from(limit).unwrap_or(u64::MAX).saturating_add(1);
    let mut bytes = Vec::new();
    reader
        .take(ceiling)
        .read_to_end(&mut bytes)
        .map_err(|error| Error::Parse(error.to_string()))?;
    if bytes.len() > limit {
        return Err(Error::LimitExceeded {
            limit: "CalDAV input size",
            actual: bytes.len(),
            max: limit,
        });
    }
    from_slice(&bytes)
}

#[cfg(test)]
mod tests {
    use serde_icalendar::CalendarEvent;

    use super::{CalDavCalendarObject, calendar_object_to_string, from_str};

    #[test]
    fn round_trip_caldav_object() -> Result<(), Box<dyn std::error::Error>> {
        let object = CalDavCalendarObject {
            href: Some("/calendars/home/event.ics".to_owned()),
            etag: None,
            content_type: Some("text/calendar; charset=utf-8".to_owned()),
            event: CalendarEvent::default(),
        };
        let xml = calendar_object_to_string(&object)?;
        assert!(xml.contains("multistatus"));
        assert!(xml.contains("calendar-data"));

        let decoded = from_str(&xml)?;
        assert_eq!(decoded.responses.len(), 1);
        assert_eq!(decoded.responses[0].href, object.href);
        Ok(())
    }
}
