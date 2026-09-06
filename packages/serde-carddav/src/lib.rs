//! Serde format crate for RFC 6352 CardDAV XML.

mod de;
mod error;
mod model;
mod ser;
pub mod xmlns;

use std::io::{Read, Write};

pub use de::{parse_address_object, parse_multistatus};
pub use error::{Error, Result};
pub use model::{
    CardDavAddressBookResource, CardDavAddressObject, CardDavMultistatus, CardDavResponse,
};
pub use ser::{address_object_to_string, multistatus_to_string};

/// Maximum number of bytes [`from_reader`] will read.
pub const MAX_INPUT_BYTES: usize = 64 * 1024 * 1024;

/// Serialize a multistatus into CardDAV XML.
pub fn to_string(multistatus: &CardDavMultistatus) -> Result<String> {
    multistatus_to_string(multistatus)
}

/// Serialize a multistatus into a CardDAV XML writer.
pub fn to_writer<W: Write>(mut writer: W, multistatus: &CardDavMultistatus) -> Result<()> {
    writer
        .write_all(to_string(multistatus)?.as_bytes())
        .map_err(|error| Error::Serialize(error.to_string()))
}

/// Parse a RFC 6352 multistatus document.
pub fn from_str(input: &str) -> Result<CardDavMultistatus> {
    from_slice(input.as_bytes())
}

/// Parse a RFC 6352 multistatus document.
pub fn from_slice(input: &[u8]) -> Result<CardDavMultistatus> {
    de::parse_multistatus(input)
}

/// Parse a multistatus from a reader, reading at most [`MAX_INPUT_BYTES`].
pub fn from_reader<R: Read>(reader: R) -> Result<CardDavMultistatus> {
    from_reader_with_limit(reader, MAX_INPUT_BYTES)
}

/// Parse a multistatus from a reader, reading at most `limit` bytes.
pub fn from_reader_with_limit<R: Read>(reader: R, limit: usize) -> Result<CardDavMultistatus> {
    let ceiling = u64::try_from(limit).unwrap_or(u64::MAX).saturating_add(1);
    let mut bytes = Vec::new();
    reader
        .take(ceiling)
        .read_to_end(&mut bytes)
        .map_err(|error| Error::Parse(error.to_string()))?;
    if bytes.len() > limit {
        return Err(Error::LimitExceeded {
            limit: "CardDAV input size",
            actual: bytes.len(),
            max: limit,
        });
    }
    from_slice(&bytes)
}

#[cfg(test)]
mod tests {
    use serde_vcard::VCard;

    use super::{
        CardDavAddressObject, CardDavMultistatus, CardDavResponse, address_object_to_string,
        from_str, to_string,
    };

    #[test]
    fn round_trip_carddav_object() -> Result<(), Box<dyn std::error::Error>> {
        let object = CardDavAddressObject {
            href: Some("/addressbooks/home/contact.vcf".to_owned()),
            etag: None,
            content_type: Some("text/vcard; charset=utf-8".to_owned()),
            vcard: VCard {
                formatted_name: Some("Jane Doe".to_owned()),
                ..VCard::default()
            },
        };
        let xml = address_object_to_string(&object)?;
        assert!(xml.contains("multistatus"));
        assert!(xml.contains("address-data"));

        let decoded = from_str(&xml)?;
        assert_eq!(decoded.responses.len(), 1);
        assert_eq!(decoded.responses[0].href, object.href);
        Ok(())
    }

    #[test]
    fn multistatus_round_trip() -> Result<(), Box<dyn std::error::Error>> {
        let multistatus = CardDavMultistatus {
            responses: vec![CardDavResponse {
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
        let decoded = from_str(&xml)?;
        assert_eq!(decoded.responses.len(), 1);
        Ok(())
    }
}
