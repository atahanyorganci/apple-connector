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

pub fn to_writer<W, T>(mut writer: W, value: &T) -> Result<()>
where
    W: Write,
    T: serde::Serialize,
{
    let json = serde_json::to_value(value).map_err(|e| Error::Serialize(e.to_string()))?;
    if json.get("responses").is_some() {
        let multistatus: CardDavMultistatus =
            serde_json::from_value(json).map_err(|e| Error::Serialize(e.to_string()))?;
        let xml = multistatus_to_xml(&multistatus)?;
        writer
            .write_all(xml.as_bytes())
            .map_err(|e| Error::Serialize(e.to_string()))?;
    } else {
        let object: CardDavAddressObject =
            serde_json::from_value(json).map_err(|e| Error::Serialize(e.to_string()))?;
        let xml = object_to_xml(&object)?;
        writer
            .write_all(xml.as_bytes())
            .map_err(|e| Error::Serialize(e.to_string()))?;
    }
    Ok(())
}

fn multistatus_to_xml(multistatus: &CardDavMultistatus) -> Result<String> {
    let mut writer = Writer::new_with_indent(Vec::new(), b' ', 2);
    writer
        .write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))
        .map_err(|e| Error::Serialize(e.to_string()))?;

    let mut root = BytesStart::new("multistatus");
    root.push_attribute(("xmlns:d", DAV_NS));
    root.push_attribute(("xmlns:card", CARD_NS));
    writer
        .write_event(Event::Start(root))
        .map_err(|e| Error::Serialize(e.to_string()))?;

    for response in &multistatus.responses {
        write_response(&mut writer, response)?;
    }

    writer
        .write_event(Event::End(BytesEnd::new("multistatus")))
        .map_err(|e| Error::Serialize(e.to_string()))?;
    String::from_utf8(writer.into_inner()).map_err(|e| Error::Serialize(e.to_string()))
}

fn object_to_xml(object: &CardDavAddressObject) -> Result<String> {
    multistatus_to_xml(&CardDavMultistatus {
        responses: vec![CardDavResponse {
            href: object.href.clone(),
            address_object: Some(object.clone()),
        }],
    })
}

fn write_response(writer: &mut Writer<Vec<u8>>, response: &CardDavResponse) -> Result<()> {
    writer
        .write_event(Event::Start(BytesStart::new("response")))
        .map_err(|e| Error::Serialize(e.to_string()))?;
    if let Some(href) = &response.href {
        write_element(writer, "href", href)?;
    }
    if let Some(object) = &response.address_object {
        writer
            .write_event(Event::Start(BytesStart::new("propstat")))
            .map_err(|e| Error::Serialize(e.to_string()))?;
        writer
            .write_event(Event::Start(BytesStart::new("prop")))
            .map_err(|e| Error::Serialize(e.to_string()))?;
        write_address_data(writer, object)?;
        writer
            .write_event(Event::End(BytesEnd::new("prop")))
            .map_err(|e| Error::Serialize(e.to_string()))?;
        write_element(writer, "status", "HTTP/1.1 200 OK")?;
        writer
            .write_event(Event::End(BytesEnd::new("propstat")))
            .map_err(|e| Error::Serialize(e.to_string()))?;
    }
    writer
        .write_event(Event::End(BytesEnd::new("response")))
        .map_err(|e| Error::Serialize(e.to_string()))?;
    Ok(())
}

fn write_address_data(writer: &mut Writer<Vec<u8>>, object: &CardDavAddressObject) -> Result<()> {
    let mut address_data = BytesStart::new("card:address-data");
    address_data.push_attribute(("xmlns:card", CARD_NS));
    writer
        .write_event(Event::Start(address_data))
        .map_err(|e| Error::Serialize(e.to_string()))?;
    let vcf = serde_vcard::to_string(&object.vcard).map_err(|e| Error::Serialize(e.to_string()))?;
    writer
        .write_event(Event::Text(BytesText::from_escaped(vcf.trim())))
        .map_err(|e| Error::Serialize(e.to_string()))?;
    writer
        .write_event(Event::End(BytesEnd::new("card:address-data")))
        .map_err(|e| Error::Serialize(e.to_string()))?;
    Ok(())
}

fn write_element(writer: &mut Writer<Vec<u8>>, name: &str, value: &str) -> Result<()> {
    writer
        .write_event(Event::Start(BytesStart::new(name)))
        .map_err(|e| Error::Serialize(e.to_string()))?;
    writer
        .write_event(Event::Text(BytesText::new(value)))
        .map_err(|e| Error::Serialize(e.to_string()))?;
    writer
        .write_event(Event::End(BytesEnd::new(name)))
        .map_err(|e| Error::Serialize(e.to_string()))?;
    Ok(())
}
