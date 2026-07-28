use quick_xml::{Reader, events::Event};
use serde_vcard::VCard;

use crate::{
    error::{Error, Result},
    model::{CardDavAddressObject, CardDavMultistatus, CardDavResponse},
};

pub fn parse_xml(input: &[u8]) -> Result<CardDavAddressObject> {
    let text = std::str::from_utf8(input).map_err(|e| Error::Parse(e.to_string()))?;
    let mut reader = Reader::from_str(text);
    reader.config_mut().trim_text(true);

    let mut buf = Vec::new();
    let mut href = None;
    let mut address_data = String::new();
    let mut in_address_data = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                if name.ends_with("address-data") {
                    in_address_data = true;
                    address_data.clear();
                }
            }
            Ok(Event::Text(e)) => {
                if in_address_data {
                    address_data
                        .push_str(&e.unescape().map_err(|err| Error::Parse(err.to_string()))?);
                }
            }
            Ok(Event::End(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                if name.ends_with("address-data") {
                    in_address_data = false;
                }
            }
            Ok(Event::Eof) => break,
            Err(error) => return Err(Error::Parse(error.to_string())),
            _ => {}
        }
        buf.clear();
    }

    let mut reader = Reader::from_str(text);
    reader.config_mut().trim_text(true);
    buf.clear();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) if e.name().as_ref().ends_with(b"href") => {
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

    let vcard = if address_data.trim().is_empty() {
        VCard::default()
    } else {
        serde_vcard::from_str::<VCard>(&address_data).map_err(|e| Error::Parse(e.to_string()))?
    };

    Ok(CardDavAddressObject {
        href,
        etag: None,
        content_type: Some("text/vcard; charset=utf-8".to_owned()),
        vcard,
    })
}

pub fn parse_multistatus(input: &[u8]) -> Result<CardDavMultistatus> {
    let object = parse_xml(input)?;
    Ok(CardDavMultistatus {
        responses: vec![CardDavResponse {
            href: object.href.clone(),
            address_object: Some(object),
        }],
    })
}
