use base64::{Engine, engine::general_purpose::STANDARD};
use chrono::{NaiveDate, Utc};

use crate::{
    error::{Error, Result},
    model::{
        Address, DateOrDateTime, Email, ExtensionBag, Photo, StructuredName, Telephone, VCard,
    },
};

pub fn parse_vcard(input: &[u8]) -> Result<VCard> {
    let text = std::str::from_utf8(input).map_err(|e| Error::Parse(e.to_string()))?;
    let cards = parse_vcards(text)?;
    cards
        .into_iter()
        .next()
        .ok_or_else(|| Error::Parse("no VCARD found".to_owned()))
}

pub fn parse_vcards(input: &str) -> Result<Vec<VCard>> {
    let unfolded = unfold_lines(input);
    let mut cards = Vec::new();
    let mut current: Option<VCard> = None;

    for line in unfolded {
        if line == "BEGIN:VCARD" {
            current = Some(VCard::default());
            continue;
        }
        if line == "END:VCARD" {
            if let Some(card) = current.take() {
                cards.push(card);
            }
            continue;
        }
        let Some(card) = current.as_mut() else {
            continue;
        };
        apply_line(card, &line)?;
    }

    Ok(cards)
}

fn unfold_lines(input: &str) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for raw in input.lines() {
        if raw.starts_with(' ') || raw.starts_with('\t') {
            current.push_str(raw.trim_start());
        } else {
            if !current.is_empty() {
                lines.push(current.clone());
            }
            current = raw.to_owned();
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

fn apply_line(card: &mut VCard, line: &str) -> Result<()> {
    let (name, params, value) = split_property(line)?;
    let upper = name.to_ascii_uppercase();
    match upper.as_str() {
        "VERSION" => {}
        "UID" => card.uid = Some(unescape_value(value)),
        "FN" => card.formatted_name = Some(unescape_value(value)),
        "N" => card.structured_name = Some(parse_structured_name(value)),
        "NICKNAME" => card.nickname = Some(unescape_value(value)),
        "ORG" => card.organization = Some(unescape_value(value)),
        "TITLE" => card.title = Some(unescape_value(value)),
        "NOTE" => card.note = Some(unescape_value(value)),
        "BDAY" => card.birthday = parse_date(value),
        "TEL" => card.phones.push(parse_telephone(&params, value)),
        "EMAIL" => card.emails.push(parse_email(&params, value)),
        "ADR" => card.addresses.push(parse_address(&params, value)),
        "PHOTO" => card.photo = Some(parse_photo(&params, value)?),
        key if key.starts_with("X-") => {
            let bag = card.extensions.get_or_insert_with(ExtensionBag::default);
            bag.properties
                .insert(key.to_owned(), unescape_value(value));
        }
        _ => {}
    }
    Ok(())
}

fn split_property(line: &str) -> Result<(&str, Vec<String>, &str)> {
    let (left, value) = line
        .split_once(':')
        .ok_or_else(|| Error::Parse(format!("invalid property line: {line}")))?;
    let mut parts = left.split(';');
    let name = parts.next().unwrap_or("");
    let params = parts.map(str::to_owned).collect();
    Ok((name, params, value))
}

fn parse_structured_name(value: &str) -> StructuredName {
    let parts: Vec<&str> = value.split(';').collect();
    StructuredName {
        family: parts.first().map(|v| unescape_value(v)),
        given: parts.get(1).map(|v| unescape_value(v)),
        additional: parts.get(2).map(|v| unescape_value(v)),
        prefixes: parts.get(3).map(|v| unescape_value(v)),
        suffixes: parts.get(4).map(|v| unescape_value(v)),
    }
}

fn parse_telephone(params: &[String], value: &str) -> Telephone {
    let (label, preferred, phone_type) = parse_params(params);
    Telephone {
        number: unescape_value(value),
        label,
        preferred,
        phone_type,
    }
}

fn parse_email(params: &[String], value: &str) -> Email {
    let (label, preferred, _) = parse_params(params);
    Email {
        address: unescape_value(value),
        label,
        preferred,
    }
}

fn parse_address(params: &[String], value: &str) -> Address {
    let (label, preferred, _) = parse_params(params);
    let parts: Vec<&str> = value.split(';').collect();
    Address {
        street: parts.get(2).map(|v| unescape_value(v)),
        locality: parts.get(3).map(|v| unescape_value(v)),
        region: parts.get(4).map(|v| unescape_value(v)),
        postal_code: parts.get(5).map(|v| unescape_value(v)),
        country: parts.get(6).map(|v| unescape_value(v)),
        label,
        preferred,
    }
}

fn parse_photo(params: &[String], value: &str) -> Result<Photo> {
    let mut media_type = None;
    for param in params {
        if let Some(rest) = param.strip_prefix("TYPE=") {
            media_type = Some(rest.to_owned());
        }
    }
    let data = STANDARD
        .decode(value.trim())
        .map_err(|error| Error::Parse(error.to_string()))?;
    Ok(Photo {
        data,
        media_type,
    })
}

fn parse_params(params: &[String]) -> (Option<String>, bool, Option<String>) {
    let mut label = None;
    let mut preferred = false;
    let mut phone_type = None;
    for param in params {
        if param == "PREF=1" {
            preferred = true;
        } else if let Some(rest) = param.strip_prefix("TYPE=") {
            label = Some(unescape_param(rest));
            phone_type = Some(unescape_param(rest));
        }
    }
    (label, preferred, phone_type)
}

fn parse_date(value: &str) -> Option<DateOrDateTime> {
    if value.contains('T') {
        chrono::DateTime::parse_from_rfc3339(value)
            .ok()
            .map(|dt| DateOrDateTime::DateTime(dt.with_timezone(&Utc)))
    } else {
        NaiveDate::parse_from_str(value, "%Y%m%d")
            .ok()
            .map(DateOrDateTime::Date)
    }
}

fn unescape_value(value: &str) -> String {
    let mut output = String::new();
    let mut chars = value.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.next() {
                Some('\\') => output.push('\\'),
                Some('n') | Some('N') => output.push('\n'),
                Some(',') => output.push(','),
                Some(';') => output.push(';'),
                Some(other) => output.push(other),
                None => {}
            }
        } else {
            output.push(ch);
        }
    }
    output
}

fn unescape_param(value: &str) -> String {
    value.replace("\\\"", "\"").replace("\\\\", "\\")
}

#[cfg(test)]
mod tests {
    use super::{parse_vcard, parse_vcards};

    #[test]
    fn parses_vcard_3_and_4() {
        let v3 = "BEGIN:VCARD\r\nVERSION:3.0\r\nFN:Alice\r\nEND:VCARD\r\n";
        let card = parse_vcard(v3.as_bytes()).expect("parse v3");
        assert_eq!(card.formatted_name.as_deref(), Some("Alice"));

        let v4 = "BEGIN:VCARD\nVERSION:4.0\nFN:Bob\nEND:VCARD\n";
        let card = parse_vcard(v4.as_bytes()).expect("parse v4");
        assert_eq!(card.formatted_name.as_deref(), Some("Bob"));
    }

    #[test]
    fn parses_multiple_cards() {
        let input = "BEGIN:VCARD\nFN:One\nEND:VCARD\nBEGIN:VCARD\nFN:Two\nEND:VCARD\n";
        let cards = parse_vcards(input).expect("parse");
        assert_eq!(cards.len(), 2);
    }

    #[test]
    fn preserves_x_properties() {
        let input = "BEGIN:VCARD\nFN:Test\nX-CUSTOM:hello\nEND:VCARD\n";
        let card = parse_vcard(input.as_bytes()).expect("parse");
        let extensions = card.extensions.expect("extensions");
        assert_eq!(extensions.properties.get("X-CUSTOM").map(String::as_str), Some("hello"));
    }
}
