use std::io::Write;

use quick_xml::{
    Writer,
    events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event},
};

use crate::{
    error::{Error, Result},
    model::CalDavCalendarObject,
    xmlns::{CALDAV_NS, DAV_NS},
};

pub fn to_writer<W, T>(mut writer: W, value: &T) -> Result<()>
where
    W: Write,
    T: serde::Serialize,
{
    let object: CalDavCalendarObject = serde_json::from_value(
        serde_json::to_value(value).map_err(|e| Error::Serialize(e.to_string()))?,
    )
    .map_err(|e| Error::Serialize(e.to_string()))?;
    let xml = object_to_xml(&object)?;
    writer
        .write_all(xml.as_bytes())
        .map_err(|e| Error::Serialize(e.to_string()))
}

fn object_to_xml(object: &CalDavCalendarObject) -> Result<String> {
    let mut writer = Writer::new_with_indent(Vec::new(), b' ', 2);
    writer
        .write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))
        .map_err(|e| Error::Serialize(e.to_string()))?;

    let mut multistatus = BytesStart::new("multistatus");
    multistatus.push_attribute(("xmlns:d", DAV_NS));
    multistatus.push_attribute(("xmlns:c", CALDAV_NS));
    writer
        .write_event(Event::Start(multistatus))
        .map_err(|e| Error::Serialize(e.to_string()))?;

    write_response(&mut writer, object)?;
    writer
        .write_event(Event::End(BytesEnd::new("multistatus")))
        .map_err(|e| Error::Serialize(e.to_string()))?;

    String::from_utf8(writer.into_inner()).map_err(|e| Error::Serialize(e.to_string()))
}

fn write_response(writer: &mut Writer<Vec<u8>>, object: &CalDavCalendarObject) -> Result<()> {
    writer
        .write_event(Event::Start(BytesStart::new("response")))
        .map_err(|e| Error::Serialize(e.to_string()))?;

    if let Some(href) = &object.href {
        write_element(writer, "href", href)?;
    }
    writer
        .write_event(Event::Start(BytesStart::new("propstat")))
        .map_err(|e| Error::Serialize(e.to_string()))?;
    writer
        .write_event(Event::Start(BytesStart::new("prop")))
        .map_err(|e| Error::Serialize(e.to_string()))?;

    let mut cal_data = BytesStart::new("calendar-data");
    cal_data.push_attribute(("xmlns", CALDAV_NS));
    writer
        .write_event(Event::Start(cal_data))
        .map_err(|e| Error::Serialize(e.to_string()))?;
    let ics =
        serde_icalendar::to_string(&object.event).map_err(|e| Error::Serialize(e.to_string()))?;
    writer
        .write_event(Event::Text(BytesText::from_escaped(ics.trim())))
        .map_err(|e| Error::Serialize(e.to_string()))?;
    writer
        .write_event(Event::End(BytesEnd::new("calendar-data")))
        .map_err(|e| Error::Serialize(e.to_string()))?;

    writer
        .write_event(Event::End(BytesEnd::new("prop")))
        .map_err(|e| Error::Serialize(e.to_string()))?;
    write_element(writer, "status", "HTTP/1.1 200 OK")?;
    writer
        .write_event(Event::End(BytesEnd::new("propstat")))
        .map_err(|e| Error::Serialize(e.to_string()))?;
    writer
        .write_event(Event::End(BytesEnd::new("response")))
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
