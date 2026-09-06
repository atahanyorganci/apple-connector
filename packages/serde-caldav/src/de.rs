use quick_xml::{
    NsReader,
    events::Event,
    name::{Namespace, ResolveResult},
};
use serde_icalendar::CalendarEvent;

use crate::{
    error::{Error, Result},
    model::{CalDavCalendarObject, CalDavMultistatus, CalDavResponse},
    xmlns::{CALDAV_NS, DAV_NS},
};

/// The element whose character data is currently being collected.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Capture {
    Href,
    CalendarData,
    Etag,
    ContentType,
    Status,
}

#[derive(Default)]
struct ResponseBuilder {
    href: Option<String>,
    etag: Option<String>,
    content_type: Option<String>,
    status: Option<String>,
    propstat_status: Option<String>,
    calendar_data: String,
    saw_calendar_data: bool,
}

impl ResponseBuilder {
    fn finish(self) -> Result<CalDavResponse> {
        // Servers indent the embedded payload and often leave a trailing CR
        // from a `&#13;` entity; neither is part of the iCalendar object.
        let payload = self.calendar_data.trim();
        let calendar_object = if self.saw_calendar_data && !payload.is_empty() {
            let event = serde_icalendar::from_str::<CalendarEvent>(payload).map_err(|error| {
                Error::Parse(format!("calendar-data is not valid iCalendar: {error}"))
            })?;
            Some(CalDavCalendarObject {
                href: self.href.clone(),
                etag: self.etag.clone(),
                content_type: self
                    .content_type
                    .clone()
                    .or_else(|| Some("text/calendar; charset=utf-8".to_owned())),
                event,
            })
        } else {
            None
        };

        Ok(CalDavResponse {
            href: self.href,
            etag: self.etag,
            // A propstat status stands in when the response carries none.
            status: self.status.or(self.propstat_status),
            calendar_object,
        })
    }
}

/// Parse an RFC 4791 multistatus document.
///
/// Element matching is on the resolved namespace and local name, so a server
/// sending `<c:calendar-data>`, `<cal:calendar-data>` or a default-namespaced
/// `<calendar-data>` all parse identically.
pub fn parse_multistatus(input: &[u8]) -> Result<CalDavMultistatus> {
    let dav = Namespace(DAV_NS.as_bytes());
    let caldav = Namespace(CALDAV_NS.as_bytes());

    let mut reader = NsReader::from_reader(input);
    reader.config_mut().trim_text(true);

    let mut buffer = Vec::new();
    let mut responses = Vec::new();
    let mut current: Option<ResponseBuilder> = None;
    let mut capture: Option<Capture> = None;
    let mut in_propstat = false;

    loop {
        let (namespace, event) = reader
            .read_resolved_event_into(&mut buffer)
            .map_err(|error| Error::Parse(error.to_string()))?;

        match event {
            Event::Start(start) => {
                let local = start.local_name();
                let local = local.as_ref();
                let bound = match namespace {
                    ResolveResult::Bound(ns) => Some(ns),
                    _ => None,
                };

                if bound == Some(dav) {
                    match local {
                        b"response" => {
                            current = Some(ResponseBuilder::default());
                            in_propstat = false;
                        }
                        b"propstat" => in_propstat = true,
                        b"href" => capture = Some(Capture::Href),
                        b"getetag" => capture = Some(Capture::Etag),
                        b"getcontenttype" => capture = Some(Capture::ContentType),
                        b"status" => capture = Some(Capture::Status),
                        _ => {}
                    }
                } else if bound == Some(caldav) && local == b"calendar-data" {
                    capture = Some(Capture::CalendarData);
                    if let Some(builder) = current.as_mut() {
                        builder.saw_calendar_data = true;
                        builder.calendar_data.clear();
                    }
                }
            }
            Event::Text(text) => {
                if let Some(field) = capture {
                    let value = text
                        .unescape()
                        .map_err(|error| Error::Parse(error.to_string()))?;
                    store(current.as_mut(), field, &value, in_propstat);
                }
            }
            Event::CData(data) => {
                if let Some(field) = capture {
                    let value = data
                        .escape()
                        .map_err(|error| Error::Parse(error.to_string()))?
                        .unescape()
                        .map_err(|error| Error::Parse(error.to_string()))?
                        .into_owned();
                    store(current.as_mut(), field, &value, in_propstat);
                }
            }
            Event::End(end) => {
                let local = end.local_name();
                let local = local.as_ref();
                capture = None;
                if matches!(namespace, ResolveResult::Bound(ns) if ns == dav) {
                    match local {
                        b"propstat" => in_propstat = false,
                        b"response" => {
                            if let Some(builder) = current.take() {
                                responses.push(builder.finish()?);
                            }
                        }
                        _ => {}
                    }
                }
            }
            Event::Eof => break,
            _ => {}
        }
        buffer.clear();
    }

    Ok(CalDavMultistatus { responses })
}

fn store(builder: Option<&mut ResponseBuilder>, field: Capture, value: &str, in_propstat: bool) {
    let Some(builder) = builder else {
        return;
    };
    match field {
        // calendar-data may arrive in several text events; the rest are single
        // values where the last one written wins.
        Capture::CalendarData => builder.calendar_data.push_str(value),
        Capture::Href => builder.href = Some(value.to_owned()),
        Capture::Etag => builder.etag = Some(value.to_owned()),
        Capture::ContentType => builder.content_type = Some(value.to_owned()),
        Capture::Status => {
            if in_propstat {
                builder.propstat_status = Some(value.to_owned());
            } else {
                builder.status = Some(value.to_owned());
            }
        }
    }
}

/// Parse a document expected to describe a single calendar object.
pub fn parse_calendar_object(input: &[u8]) -> Result<CalDavCalendarObject> {
    parse_multistatus(input)?
        .responses
        .into_iter()
        .find_map(|response| response.calendar_object)
        .ok_or_else(|| Error::Parse("no calendar-data found in multistatus".to_owned()))
}

/// Retained for the generic serde entry points; superseded by
/// [`parse_calendar_object`].
pub fn parse_xml(input: &[u8]) -> Result<CalDavCalendarObject> {
    parse_calendar_object(input)
}
