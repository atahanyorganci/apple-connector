use std::io::Write;

use quick_xml::{
    Writer,
    events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event},
};

use crate::{
    error::{Error, Result},
    model::{CardDavAddressObject, CardDavMultistatus, CardDavResponse},
    xmlns::{CARD_NS, DAV_NS},
};

const DEFAULT_STATUS: &str = "HTTP/1.1 200 OK";

pub fn to_writer<W, T>(mut writer: W, value: &T) -> Result<()>
where
    W: Write,
    T: serde::Serialize,
{
    // Structural, not name-based: a value that deserializes as a multistatus is
    // one, and anything else is tried as a single address object.
    let json = serde_json::to_value(value).map_err(|e| Error::Serialize(e.to_string()))?;
    let xml = match serde_json::from_value::<CardDavMultistatus>(json.clone()) {
        Ok(multistatus) if json.get("responses").is_some() => multistatus_to_string(&multistatus)?,
        _ => {
            let object: CardDavAddressObject =
                serde_json::from_value(json).map_err(|e| Error::Serialize(e.to_string()))?;
            address_object_to_string(&object)?
        }
    };
    writer
        .write_all(xml.as_bytes())
        .map_err(|e| Error::Serialize(e.to_string()))
}

/// Serialize one address object as a single-response multistatus.
pub fn address_object_to_string(object: &CardDavAddressObject) -> Result<String> {
    multistatus_to_string(&CardDavMultistatus {
        responses: vec![CardDavResponse {
            href: object.href.clone(),
            etag: object.etag.clone(),
            status: None,
            address_object: Some(object.clone()),
        }],
    })
}

/// Serialize every response into one `multistatus` document, with both
/// prefixes bound once on the root.
pub fn multistatus_to_string(multistatus: &CardDavMultistatus) -> Result<String> {
    let mut writer = Writer::new_with_indent(Vec::new(), b' ', 2);
    write(
        &mut writer,
        Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)),
    )?;

    let mut root = BytesStart::new("d:multistatus");
    root.push_attribute(("xmlns:d", DAV_NS));
    root.push_attribute(("xmlns:card", CARD_NS));
    write(&mut writer, Event::Start(root))?;

    for response in &multistatus.responses {
        write_response(&mut writer, response)?;
    }

    write(&mut writer, Event::End(BytesEnd::new("d:multistatus")))?;
    String::from_utf8(writer.into_inner()).map_err(|e| Error::Serialize(e.to_string()))
}

fn write(writer: &mut Writer<Vec<u8>>, event: Event<'_>) -> Result<()> {
    writer
        .write_event(event)
        .map_err(|e| Error::Serialize(e.to_string()))
}

fn write_response(writer: &mut Writer<Vec<u8>>, response: &CardDavResponse) -> Result<()> {
    write(writer, Event::Start(BytesStart::new("d:response")))?;

    if let Some(href) = response
        .href
        .as_deref()
        .or_else(|| response.address_object.as_ref()?.href.as_deref())
    {
        write_element(writer, "d:href", href)?;
    }

    write(writer, Event::Start(BytesStart::new("d:propstat")))?;
    write(writer, Event::Start(BytesStart::new("d:prop")))?;

    let etag = response
        .etag
        .as_deref()
        .or_else(|| response.address_object.as_ref()?.etag.as_deref());
    if let Some(etag) = etag {
        write_element(writer, "d:getetag", etag)?;
    }

    if let Some(object) = &response.address_object {
        if let Some(content_type) = &object.content_type {
            write_element(writer, "d:getcontenttype", content_type)?;
        }
        let vcf =
            serde_vcard::to_string(&object.vcard).map_err(|e| Error::Serialize(e.to_string()))?;
        write_element(writer, "card:address-data", vcf.trim())?;
    }

    write(writer, Event::End(BytesEnd::new("d:prop")))?;
    write_element(
        writer,
        "d:status",
        response.status.as_deref().unwrap_or(DEFAULT_STATUS),
    )?;
    write(writer, Event::End(BytesEnd::new("d:propstat")))?;
    write(writer, Event::End(BytesEnd::new("d:response")))?;
    Ok(())
}

fn write_element(writer: &mut Writer<Vec<u8>>, name: &str, value: &str) -> Result<()> {
    write(writer, Event::Start(BytesStart::new(name)))?;
    write(writer, Event::Text(BytesText::new(value)))?;
    write(writer, Event::End(BytesEnd::new(name)))?;
    Ok(())
}
