//! RFC 6350 vCard reader and writer.
//!
//! The entry points are concrete: they read and write [`VCard`] values rather
//! than pretending to be a general serde format. The models still derive
//! `Serialize`/`Deserialize` so callers can put them in JSON, but the vCard
//! wire format never routes through `serde_json::Value`.

mod de;
mod error;
mod model;
mod ser;

use std::io::{Read, Write};

pub use de::{parse_vcard, parse_vcards};
pub use error::{Error, Result};
pub use model::{
    Address, DateOrDateTime, Email, ExtensionBag, Photo, RawProperty, SocialProfile,
    StructuredName, Telephone, Url, VCard,
};

/// Maximum number of bytes [`from_reader`] will read.
pub const MAX_INPUT_BYTES: usize = 16 * 1024 * 1024;

/// Serialize a card into a vCard string.
pub fn to_string(card: &VCard) -> Result<String> {
    let mut buffer = Vec::new();
    to_writer(&mut buffer, card)?;
    String::from_utf8(buffer).map_err(|error| Error::Serialize(error.to_string()))
}

/// Serialize a card into a vCard byte stream.
pub fn to_writer<W: Write>(writer: W, card: &VCard) -> Result<()> {
    ser::to_writer(writer, card)
}

/// Parse the first card in a vCard string.
pub fn from_str(input: &str) -> Result<VCard> {
    from_slice(input.as_bytes())
}

/// Parse the first card in a vCard byte slice.
pub fn from_slice(input: &[u8]) -> Result<VCard> {
    de::parse_vcard(input)
}

/// Parse the first card from a reader, reading at most [`MAX_INPUT_BYTES`].
pub fn from_reader<R: Read>(reader: R) -> Result<VCard> {
    from_reader_with_limit(reader, MAX_INPUT_BYTES)
}

/// Parse the first card from a reader, reading at most `limit` bytes.
pub fn from_reader_with_limit<R: Read>(reader: R, limit: usize) -> Result<VCard> {
    let bytes = read_bounded(reader, limit)?;
    from_slice(&bytes)
}

/// Read at most `limit` bytes, failing rather than growing without bound.
fn read_bounded<R: Read>(reader: R, limit: usize) -> Result<Vec<u8>> {
    let ceiling = u64::try_from(limit).unwrap_or(u64::MAX).saturating_add(1);
    let mut bytes = Vec::new();
    reader
        .take(ceiling)
        .read_to_end(&mut bytes)
        .map_err(|error| Error::Parse(error.to_string()))?;
    if bytes.len() > limit {
        return Err(Error::Parse(format!(
            "vCard input exceeds the {limit} byte limit"
        )));
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::{StructuredName, Telephone, VCard, from_str, to_string};

    #[test]
    fn round_trip_minimal_contact() -> Result<(), Box<dyn std::error::Error>> {
        let card = VCard {
            formatted_name: Some("Jane Doe".to_owned()),
            structured_name: Some(StructuredName {
                given: Some("Jane".to_owned()),
                family: Some("Doe".to_owned()),
                ..StructuredName::default()
            }),
            ..VCard::default()
        };
        let vcf = to_string(&card)?;
        assert!(vcf.contains("BEGIN:VCARD"));
        assert!(vcf.contains("FN:Jane Doe"));
        let decoded = from_str(&vcf)?;
        assert_eq!(decoded.formatted_name, card.formatted_name);
        Ok(())
    }

    #[test]
    fn round_trip_multi_value_tel_email() -> Result<(), Box<dyn std::error::Error>> {
        let card = VCard {
            formatted_name: Some("Test User".to_owned()),
            phones: vec![
                Telephone {
                    number: "+15551234567".to_owned(),
                    label: Some("CELL".to_owned()),
                    preferred: true,
                },
                Telephone {
                    number: "+15559876543".to_owned(),
                    label: Some("WORK".to_owned()),
                    preferred: false,
                },
            ],
            emails: vec![super::Email {
                address: "test@example.com".to_owned(),
                label: Some("WORK".to_owned()),
                preferred: true,
            }],
            ..VCard::default()
        };
        let vcf = to_string(&card)?;
        let decoded = from_str(&vcf)?;
        assert_eq!(decoded.phones.len(), 2);
        assert_eq!(decoded.emails.len(), 1);
        Ok(())
    }

    #[test]
    fn utf8_name_round_trip() -> Result<(), Box<dyn std::error::Error>> {
        let card = VCard {
            formatted_name: Some("田中 太郎".to_owned()),
            ..VCard::default()
        };
        let vcf = to_string(&card)?;
        let decoded = from_str(&vcf)?;
        assert_eq!(decoded.formatted_name, card.formatted_name);
        Ok(())
    }

    #[test]
    fn folds_long_lines_with_newlines_without_hanging() -> Result<(), Box<dyn std::error::Error>> {
        let card = VCard {
            formatted_name: Some("Mehmet Dora".to_owned()),
            addresses: vec![super::Address {
                street: Some("ODTU-Teknokent\n37-1 SATGEB-2 Titanyum C Blok".to_owned()),
                locality: Some("Ankara".to_owned()),
                label: Some("Work".to_owned()),
                preferred: true,
                ..super::Address::default()
            }],
            ..VCard::default()
        };
        let vcf = to_string(&card)?;
        assert!(vcf.contains("BEGIN:VCARD"));
        assert!(vcf.contains("ADR"));
        assert!(
            vcf.lines().all(|line| {
                let content = line.strip_prefix(' ').unwrap_or(line);
                content.len() <= 75
            }),
            "folded lines must stay within 75 octets: {vcf}"
        );
        Ok(())
    }
}
