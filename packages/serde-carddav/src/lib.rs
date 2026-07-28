//! Serde format crate for RFC 6352 CardDAV XML.

mod de;
mod error;
mod model;
mod ser;
pub mod xmlns;

use std::io::{Read, Write};

pub use de::{parse_multistatus, parse_xml};
pub use error::{Error, Result};
pub use model::{
    CardDavAddressBookResource, CardDavAddressObject, CardDavMultistatus, CardDavResponse,
};
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
pub fn from_slice<T>(input: &[u8]) -> Result<T>
where
    T: DeserializeOwned,
{
    let text = std::str::from_utf8(input).map_err(|e| Error::Parse(e.to_string()))?;
    let value = if text.contains("multistatus")
        && std::any::type_name::<T>().contains("CardDavMultistatus")
    {
        serde_json::to_value(de::parse_multistatus(input)?)
            .map_err(|e| Error::Parse(e.to_string()))?
    } else {
        serde_json::to_value(de::parse_xml(input)?).map_err(|e| Error::Parse(e.to_string()))?
    };
    serde_json::from_value(value).map_err(|e| Error::Parse(e.to_string()))
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
    fn stub_round_trip_carddav_object() {
        let object = CardDavAddressObject {
            href: Some("/addressbooks/home/contact.vcf".to_owned()),
            etag: None,
            content_type: Some("text/vcard; charset=utf-8".to_owned()),
            vcard: VCard {
                formatted_name: Some("Jane Doe".to_owned()),
                ..VCard::default()
            },
        };
        let xml = to_string(&object).expect("serialize");
        assert!(xml.contains("multistatus"));
        assert!(xml.contains("address-data"));
        let decoded: CardDavAddressObject = from_str(&xml).expect("deserialize");
        assert_eq!(decoded.href, object.href);
    }

    #[test]
    fn multistatus_round_trip() {
        let multistatus = CardDavMultistatus {
            responses: vec![super::CardDavResponse {
                href: Some("/contacts/1.vcf".to_owned()),
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
        let xml = to_string(&multistatus).expect("serialize");
        let decoded: CardDavMultistatus = from_str(&xml).expect("deserialize");
        assert_eq!(decoded.responses.len(), 1);
    }
}
