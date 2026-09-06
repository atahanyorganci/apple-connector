use quick_xml::{
    NsReader,
    events::Event,
    name::{Namespace, ResolveResult},
};
use serde_vcard::VCard;

use crate::{
    error::{Error, Result},
    model::{CardDavAddressObject, CardDavMultistatus, CardDavResponse},
    xmlns::{CARD_NS, DAV_NS},
};

/// The element whose character data is currently being collected.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Capture {
    Href,
    AddressData,
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
    address_data: String,
    saw_address_data: bool,
}

impl ResponseBuilder {
    fn finish(self) -> Result<CardDavResponse> {
        // Servers indent the embedded payload; the surrounding whitespace is
        // not part of the vCard.
        let payload = self.address_data.trim();
        let address_object = if self.saw_address_data && !payload.is_empty() {
            let vcard = serde_vcard::from_str::<VCard>(payload).map_err(|error| {
                Error::Parse(format!("address-data is not a valid vCard: {error}"))
            })?;
            Some(CardDavAddressObject {
                href: self.href.clone(),
                etag: self.etag.clone(),
                content_type: self
                    .content_type
                    .clone()
                    .or_else(|| Some("text/vcard; charset=utf-8".to_owned())),
                vcard,
            })
        } else {
            None
        };

        Ok(CardDavResponse {
            href: self.href,
            etag: self.etag,
            status: self.status.or(self.propstat_status),
            address_object,
        })
    }
}

/// Parse an RFC 6352 multistatus document.
///
/// Elements are matched on their resolved namespace and local name, so any
/// prefix parses. Suffix matching — the previous approach — also accepted
/// unrelated elements such as `<x-my-address-data>` and `<myhref>`.
pub fn parse_multistatus(input: &[u8]) -> Result<CardDavMultistatus> {
    let dav = Namespace(DAV_NS.as_bytes());
    let carddav = Namespace(CARD_NS.as_bytes());

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
                } else if bound == Some(carddav) && local == b"address-data" {
                    capture = Some(Capture::AddressData);
                    if let Some(builder) = current.as_mut() {
                        builder.saw_address_data = true;
                        builder.address_data.clear();
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

    Ok(CardDavMultistatus { responses })
}

fn store(builder: Option<&mut ResponseBuilder>, field: Capture, value: &str, in_propstat: bool) {
    let Some(builder) = builder else {
        return;
    };
    match field {
        Capture::AddressData => builder.address_data.push_str(value),
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

/// Parse a document expected to describe a single address object.
pub fn parse_address_object(input: &[u8]) -> Result<CardDavAddressObject> {
    parse_multistatus(input)?
        .responses
        .into_iter()
        .find_map(|response| response.address_object)
        .ok_or_else(|| Error::Parse("no address-data found in multistatus".to_owned()))
}

/// Retained for the generic serde entry points; superseded by
/// [`parse_address_object`].
pub fn parse_xml(input: &[u8]) -> Result<CardDavAddressObject> {
    parse_address_object(input)
}
