//! Serde format crate for RFC 6352 CardDAV XML.

mod de;
mod error;
mod model;
mod ser;
pub mod xmlns;

use std::io::{Read, Write};

pub use de::{parse_address_object, parse_multistatus, parse_xml};
pub use error::{Error, Result};
pub use model::{
    CardDavAddressBookResource, CardDavAddressObject, CardDavMultistatus, CardDavResponse,
};
pub use ser::{address_object_to_string, multistatus_to_string};
use serde::{Serialize, de::DeserializeOwned};

/// Serialize a value into CardDAV XML with embedded vCard address-data.
pub fn to_string<T>(value: &T) -> Result<String>
where
    T: Serialize,
{
    let mut buffer = Vec::new();
    to_writer(&mut buffer, value)?;
    String::from_utf8(buffer).map_err(|error| Error::Serialize(error.to_string()))
}

/// Deserialize a value from CardDAV XML.
pub fn from_str<T>(input: &str) -> Result<T>
where
    T: DeserializeOwned,
{
    from_slice(input.as_bytes())
}

/// Serialize a value into a CardDAV XML writer.
pub fn to_writer<W, T>(writer: W, value: &T) -> Result<()>
where
    W: Write,
    T: serde::Serialize,
{
    ser::to_writer(writer, value)
}

/// Deserialize a value from a CardDAV XML byte slice.
///
/// The document is always parsed as a multistatus, and the result is shaped to
/// `T` structurally. Routing used to depend on `std::any::type_name::<T>()`
/// containing "CardDavMultistatus" and on the word "multistatus" appearing
/// anywhere in the document — including inside a vCard NOTE — so a type alias
/// or a wrapper changed which parser ran.
pub fn from_slice<T>(input: &[u8]) -> Result<T>
where
    T: DeserializeOwned,
{
    let multistatus = de::parse_multistatus(input)?;
    let as_multistatus =
        serde_json::to_value(&multistatus).map_err(|e| Error::Parse(e.to_string()))?;
    if let Ok(value) = serde_json::from_value::<T>(as_multistatus) {
        return Ok(value);
    }

    let object = multistatus
        .responses
        .into_iter()
        .find_map(|response| response.address_object)
        .ok_or_else(|| Error::Parse("no address-data found in multistatus".to_owned()))?;
    serde_json::from_value(serde_json::to_value(object).map_err(|e| Error::Parse(e.to_string()))?)
        .map_err(|e| Error::Parse(e.to_string()))
}

/// Deserialize a value from a reader containing CardDAV XML.
pub fn from_reader<R, T>(mut reader: R) -> Result<T>
where
    R: Read,
    T: DeserializeOwned,
{
    let mut bytes = Vec::new();
    reader
        .read_to_end(&mut bytes)
        .map_err(|error| Error::Parse(error.to_string()))?;
    from_slice(&bytes)
}

#[cfg(test)]
mod tests {
    use serde_vcard::VCard;

    use super::{CardDavAddressObject, CardDavMultistatus, from_str, to_string};

    #[test]
    fn stub_round_trip_carddav_object() -> Result<(), Box<dyn std::error::Error>> {
        let object = CardDavAddressObject {
            href: Some("/addressbooks/home/contact.vcf".to_owned()),
            etag: None,
            content_type: Some("text/vcard; charset=utf-8".to_owned()),
            vcard: VCard {
                formatted_name: Some("Jane Doe".to_owned()),
                ..VCard::default()
            },
        };
        let xml = to_string(&object)?;
        assert!(xml.contains("multistatus"));
        assert!(xml.contains("address-data"));
        let decoded: CardDavAddressObject = from_str(&xml)?;
        assert_eq!(decoded.href, object.href);
        Ok(())
    }

    #[test]
    fn multistatus_round_trip() -> Result<(), Box<dyn std::error::Error>> {
        let multistatus = CardDavMultistatus {
            responses: vec![super::CardDavResponse {
                href: Some("/contacts/1.vcf".to_owned()),
                etag: None,
                status: None,
                address_object: Some(CardDavAddressObject {
                    href: Some("/contacts/1.vcf".to_owned()),
                    etag: None,
                    content_type: None,
                    vcard: VCard {
                        formatted_name: Some("Test".to_owned()),
                        ..VCard::default()
                    },
                }),
            }],
        };
        let xml = to_string(&multistatus)?;
        let decoded: CardDavMultistatus = from_str(&xml)?;
        assert_eq!(decoded.responses.len(), 1);
        Ok(())
    }
}
