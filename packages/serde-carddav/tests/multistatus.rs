//! RFC 6352 multistatus parsing, and dispatch that does not depend on type
//! names or on words appearing in the document.

use serde_carddav::{
    CardDavAddressObject, CardDavMultistatus, CardDavResponse, address_object_to_string, from_str,
    multistatus_to_string, parse_address_object, parse_multistatus,
};
use serde_vcard::VCard;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// An alias must not change which parser runs: dispatch used to read
/// `std::any::type_name::<T>()`.
type AddressBookReport = CardDavMultistatus;

fn fixture(name: &str) -> Result<Vec<u8>, std::io::Error> {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    std::fs::read(path)
}

#[test]
fn every_response_is_retained_with_its_own_href_and_etag() -> TestResult {
    let multistatus = parse_multistatus(&fixture("icloud-multistatus.xml")?)?;
    assert_eq!(multistatus.responses.len(), 3);

    let names: Vec<Option<&str>> = multistatus
        .responses
        .iter()
        .map(|response| {
            response
                .address_object
                .as_ref()
                .and_then(|object| object.vcard.formatted_name.as_deref())
        })
        .collect();
    assert_eq!(
        names,
        vec![Some("Ada Lovelace"), Some("Grace Hopper"), None]
    );

    assert_eq!(
        multistatus.responses[0].href.as_deref(),
        Some("/1234567/carddavhome/card/ada.vcf")
    );
    assert_eq!(
        multistatus.responses[1].href.as_deref(),
        Some("/1234567/carddavhome/card/grace.vcf")
    );
    assert_eq!(
        multistatus.responses[0].etag.as_deref(),
        Some("\"C=1@U=abc\"")
    );
    assert_eq!(
        multistatus.responses[1].etag.as_deref(),
        Some("\"C=2@U=def\"")
    );
    assert_eq!(
        multistatus.responses[2].status.as_deref(),
        Some("HTTP/1.1 404 Not Found")
    );
    Ok(())
}

#[test]
fn prefixed_elements_parse() -> TestResult {
    let multistatus = parse_multistatus(&fixture("prefixed-multistatus.xml")?)?;
    assert_eq!(multistatus.responses.len(), 1);
    assert_eq!(
        multistatus.responses[0]
            .address_object
            .as_ref()
            .and_then(|object| object.vcard.formatted_name.as_deref()),
        Some("Alan Turing")
    );
    Ok(())
}

#[test]
fn a_note_containing_multistatus_does_not_steer_routing() -> TestResult {
    // The old code keyed on `text.contains("multistatus")`, which a vCard NOTE
    // can satisfy.
    let multistatus = parse_multistatus(&fixture("icloud-multistatus.xml")?)?;
    let grace = multistatus.responses[1]
        .address_object
        .as_ref()
        .ok_or("expected Grace's card")?;
    assert_eq!(
        grace.vcard.note.as_deref(),
        Some("Ask about the multistatus report format")
    );
    Ok(())
}

#[test]
fn a_type_alias_parses_as_a_multistatus() -> TestResult {
    let report: AddressBookReport =
        from_str(&String::from_utf8(fixture("icloud-multistatus.xml")?)?)?;
    assert_eq!(report.responses.len(), 3);
    Ok(())
}

#[test]
fn a_single_object_can_be_pulled_out() -> TestResult {
    let object = parse_address_object(&fixture("prefixed-multistatus.xml")?)?;
    assert_eq!(object.vcard.formatted_name.as_deref(), Some("Alan Turing"));
    assert_eq!(object.etag.as_deref(), Some("\"etag-alan\""));
    Ok(())
}

fn object(href: &str, etag: &str, name: &str) -> CardDavAddressObject {
    CardDavAddressObject {
        href: Some(href.to_owned()),
        etag: Some(etag.to_owned()),
        content_type: Some("text/vcard; charset=utf-8".to_owned()),
        vcard: VCard {
            formatted_name: Some(name.to_owned()),
            ..VCard::default()
        },
    }
}

#[test]
fn href_and_etag_round_trip_per_response() -> TestResult {
    let multistatus = CardDavMultistatus {
        responses: vec![
            CardDavResponse {
                href: Some("/contacts/1.vcf".to_owned()),
                etag: Some("\"etag-1\"".to_owned()),
                status: Some("HTTP/1.1 200 OK".to_owned()),
                address_object: Some(object("/contacts/1.vcf", "\"etag-1\"", "One")),
            },
            CardDavResponse {
                href: Some("/contacts/2.vcf".to_owned()),
                etag: Some("\"etag-2\"".to_owned()),
                status: Some("HTTP/1.1 200 OK".to_owned()),
                address_object: Some(object("/contacts/2.vcf", "\"etag-2\"", "Two")),
            },
        ],
    };

    let xml = multistatus_to_string(&multistatus)?;
    let decoded = parse_multistatus(xml.as_bytes())?;

    assert_eq!(decoded.responses.len(), 2);
    assert_eq!(
        decoded.responses[0].href.as_deref(),
        Some("/contacts/1.vcf")
    );
    assert_eq!(decoded.responses[1].etag.as_deref(), Some("\"etag-2\""));
    assert_eq!(
        decoded.responses[1]
            .address_object
            .as_ref()
            .and_then(|object| object.vcard.formatted_name.as_deref()),
        Some("Two")
    );
    Ok(())
}

#[test]
fn the_serializer_emits_one_document() -> TestResult {
    let multistatus = CardDavMultistatus {
        responses: vec![
            CardDavResponse {
                href: Some("/contacts/1.vcf".to_owned()),
                address_object: Some(object("/contacts/1.vcf", "\"e1\"", "One")),
                ..CardDavResponse::default()
            },
            CardDavResponse {
                href: Some("/contacts/2.vcf".to_owned()),
                address_object: Some(object("/contacts/2.vcf", "\"e2\"", "Two")),
                ..CardDavResponse::default()
            },
        ],
    };

    let xml = multistatus_to_string(&multistatus)?;
    assert_eq!(xml.matches("<?xml").count(), 1, "{xml}");
    assert_eq!(xml.matches("<d:multistatus").count(), 1, "{xml}");
    assert_eq!(xml.matches("<d:response>").count(), 2, "{xml}");
    assert!(
        xml.contains("xmlns:card=\"urn:ietf:params:xml:ns:carddav\""),
        "{xml}"
    );
    Ok(())
}

#[test]
fn a_single_object_serializes_as_a_one_response_multistatus() -> TestResult {
    let xml = address_object_to_string(&object("/contacts/1.vcf", "\"e1\"", "Solo"))?;
    let decoded = parse_multistatus(xml.as_bytes())?;
    assert_eq!(decoded.responses.len(), 1);
    assert_eq!(decoded.responses[0].etag.as_deref(), Some("\"e1\""));
    Ok(())
}
