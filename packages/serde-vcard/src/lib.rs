//! Serde format crate for RFC 6350 vCard text.

mod de;
mod error;
mod model;
mod ser;

use std::io::{Read, Write};

pub use error::{Error, Result};
pub use model::{
    Address, DateOrDateTime, Email, ExtensionBag, Photo, SocialProfile, StructuredName,
    Telephone, VCard,
};
use serde::{Serialize, de::DeserializeOwned};

/// Serialize a value into a vCard string.
pub fn to_string<T>(value: &T) -> Result<String>
where
    T: Serialize,
{
    let mut buffer = Vec::new();
    to_writer(&mut buffer, value)?;
    String::from_utf8(buffer).map_err(|error| Error::Serialize(error.to_string()))
}

/// Deserialize a value from a vCard string.
pub fn from_str<T>(input: &str) -> Result<T>
where
    T: DeserializeOwned,
{
    from_slice(input.as_bytes())
}

/// Serialize a value into a vCard byte stream.
pub fn to_writer<W, T>(writer: W, value: &T) -> Result<()>
where
    W: Write,
    T: Serialize,
{
    ser::to_writer(writer, value)
}

/// Deserialize a value from a vCard byte slice.
pub fn from_slice<T>(input: &[u8]) -> Result<T>
where
    T: DeserializeOwned,
{
    let card = de::parse_vcard(input)?;
    serde_json::from_value(serde_json::to_value(&card).map_err(|e| Error::Parse(e.to_string()))?)
        .map_err(|e| Error::Parse(e.to_string()))
}

/// Deserialize a value from a reader containing vCard text.
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
    use super::{StructuredName, Telephone, VCard, from_str, to_string};

    #[test]
    fn round_trip_minimal_contact() {
        let card = VCard {
            formatted_name: Some("Jane Doe".to_owned()),
            structured_name: Some(StructuredName {
                given: Some("Jane".to_owned()),
                family: Some("Doe".to_owned()),
                ..StructuredName::default()
            }),
            ..VCard::default()
        };
        let vcf = to_string(&card).expect("serialize");
        assert!(vcf.contains("BEGIN:VCARD"));
        assert!(vcf.contains("FN:Jane Doe"));
        let decoded: VCard = from_str(&vcf).expect("deserialize");
        assert_eq!(decoded.formatted_name, card.formatted_name);
    }

    #[test]
    fn round_trip_multi_value_tel_email() {
        let card = VCard {
            formatted_name: Some("Test User".to_owned()),
            phones: vec![
                Telephone {
                    number: "+15551234567".to_owned(),
                    label: Some("CELL".to_owned()),
                    preferred: true,
                    phone_type: None,
                },
                Telephone {
                    number: "+15559876543".to_owned(),
                    label: Some("WORK".to_owned()),
                    preferred: false,
                    phone_type: None,
                },
            ],
            emails: vec![super::Email {
                address: "test@example.com".to_owned(),
                label: Some("WORK".to_owned()),
                preferred: true,
            }],
            ..VCard::default()
        };
        let vcf = to_string(&card).expect("serialize");
        let decoded: VCard = from_str(&vcf).expect("deserialize");
        assert_eq!(decoded.phones.len(), 2);
        assert_eq!(decoded.emails.len(), 1);
    }

    #[test]
    fn utf8_name_round_trip() {
        let card = VCard {
            formatted_name: Some("田中 太郎".to_owned()),
            ..VCard::default()
        };
        let vcf = to_string(&card).expect("serialize");
        let decoded: VCard = from_str(&vcf).expect("deserialize");
        assert_eq!(decoded.formatted_name, card.formatted_name);
    }

    #[test]
    fn folds_long_lines_with_newlines_without_hanging() {
        let card = VCard {
            formatted_name: Some("Mehmet Dora".to_owned()),
            addresses: vec![super::Address {
                street: Some(
                    "ODTU-Teknokent\n37-1 SATGEB-2 Titanyum C Blok".to_owned(),
                ),
                locality: Some("Ankara".to_owned()),
                label: Some("Work".to_owned()),
                preferred: true,
                ..super::Address::default()
            }],
            ..VCard::default()
        };
        let vcf = to_string(&card).expect("serialize long ADR");
        assert!(vcf.contains("BEGIN:VCARD"));
        assert!(vcf.contains("ADR"));
        assert!(
            vcf.lines().all(|line| {
                let content = line.strip_prefix(' ').unwrap_or(line);
                content.len() <= 75
            }),
            "folded lines must stay within 75 octets: {vcf}"
        );
    }
}
