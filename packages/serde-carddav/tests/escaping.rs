//! Embedded vCard text must be XML-escaped, not written verbatim.

use serde_carddav::{CardDavAddressObject, from_str, to_string};
use serde_vcard::VCard;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn object_with(note: &str) -> CardDavAddressObject {
    CardDavAddressObject {
        href: Some("/addressbooks/home/contact.vcf".to_owned()),
        etag: None,
        content_type: Some("text/vcard; charset=utf-8".to_owned()),
        vcard: VCard {
            formatted_name: Some("Ada Lovelace".to_owned()),
            note: Some(note.to_owned()),
            ..VCard::default()
        },
    }
}

fn assert_well_formed(xml: &str) -> TestResult {
    let mut reader = quick_xml::Reader::from_str(xml);
    let mut buffer = Vec::new();
    loop {
        match reader.read_event_into(&mut buffer)? {
            quick_xml::events::Event::Eof => break,
            _ => buffer.clear(),
        }
    }
    Ok(())
}

#[test]
fn ampersands_and_angle_brackets_round_trip() -> TestResult {
    let object = object_with("R&D <lab> notes");
    let xml = to_string(&object)?;

    assert_well_formed(&xml)?;
    assert!(xml.contains("&amp;"), "ampersand was not escaped:\n{xml}");
    assert!(
        xml.contains("&lt;"),
        "angle bracket was not escaped:\n{xml}"
    );

    let decoded: CardDavAddressObject = from_str(&xml)?;
    assert_eq!(decoded.vcard.note.as_deref(), Some("R&D <lab> notes"));
    Ok(())
}

#[test]
fn plain_text_is_unchanged() -> TestResult {
    let object = object_with("Nothing special here");
    let xml = to_string(&object)?;

    assert_well_formed(&xml)?;
    let decoded: CardDavAddressObject = from_str(&xml)?;
    assert_eq!(decoded.vcard.note.as_deref(), Some("Nothing special here"));
    assert!(!xml.contains("&amp;amp;"), "{xml}");
    Ok(())
}
