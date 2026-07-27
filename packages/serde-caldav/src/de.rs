use quick_xml::Reader;
use quick_xml::events::Event;
use serde_icalendar::CalendarEvent;

use crate::{
    error::{Error, Result},
    model::{CalDavCalendarObject, CalDavMultistatus, CalDavResponse},
};

pub fn parse_xml(input: &[u8]) -> Result<CalDavCalendarObject> {
    let text = std::str::from_utf8(input).map_err(|e| Error::Parse(e.to_string()))?;
    let mut reader = Reader::from_str(text);
    reader.config_mut().trim_text(true);

    let mut buf = Vec::new();
    let mut href = None;
    let mut calendar_data = String::new();
    let mut in_calendar_data = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                if name == "calendar-data" {
                    in_calendar_data = true;
                    calendar_data.clear();
                }
            }
            Ok(Event::Text(e)) => {
                if in_calendar_data {
                    calendar_data.push_str(&e.unescape().map_err(|err| Error::Parse(err.to_string()))?);
                }
            }
            Ok(Event::End(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                if name == "calendar-data" {
                    in_calendar_data = false;
                }
            }
            Ok(Event::Empty(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                if name == "href" {
                    // handled in text
                }
            }
            Ok(Event::Eof) => break,
            Err(error) => return Err(Error::Parse(error.to_string())),
            _ => {}
        }
        buf.clear();
    }

    // Second pass for href
    let mut reader = Reader::from_str(text);
    reader.config_mut().trim_text(true);
    buf.clear();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) if e.name().as_ref() == b"href" => {
                if let Ok(Event::Text(text)) = reader.read_event_into(&mut buf) {
                    href = Some(
                        text.unescape()
                            .map_err(|err| Error::Parse(err.to_string()))?
                            .into_owned(),
                    );
                }
            }
            Ok(Event::Eof) => break,
            Err(error) => return Err(Error::Parse(error.to_string())),
            _ => {}
        }
        buf.clear();
    }

    let event = serde_icalendar::from_str::<CalendarEvent>(&calendar_data)
        .map_err(|e| Error::Parse(e.to_string()))?;

    Ok(CalDavCalendarObject {
        href,
        etag: None,
        content_type: Some("text/calendar; charset=utf-8".to_owned()),
        event,
    })
}

pub fn parse_multistatus(input: &[u8]) -> Result<CalDavMultistatus> {
    let object = parse_xml(input)?;
    Ok(CalDavMultistatus {
        responses: vec![CalDavResponse {
            href: object.href.clone(),
            calendar_object: Some(object),
        }],
    })
}
