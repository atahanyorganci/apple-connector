use std::io::Write;

use base64::{Engine, engine::general_purpose::STANDARD};

use crate::{
    error::{Error, Result},
    model::{Address, DateOrDateTime, StructuredName, VCard},
};

const LINE_LIMIT: usize = 75;

pub fn to_writer<W, T>(mut writer: W, value: &T) -> Result<()>
where
    W: Write,
    T: serde::Serialize,
{
    let card: VCard = serde_json::from_value(
        serde_json::to_value(value).map_err(|e| Error::Serialize(e.to_string()))?,
    )
    .map_err(|e| Error::Serialize(e.to_string()))?;
    let vcf = card_to_vcard(&card)?;
    writer
        .write_all(vcf.as_bytes())
        .map_err(|e| Error::Serialize(e.to_string()))
}

fn card_to_vcard(card: &VCard) -> Result<String> {
    let mut lines = Vec::new();
    lines.push("BEGIN:VCARD".to_owned());
    lines.push("VERSION:4.0".to_owned());
    if let Some(uid) = &card.uid {
        lines.push(format!("UID:{uid}"));
    }
    if let Some(fn_) = &card.formatted_name {
        lines.push(format!("FN:{}", escape_value(fn_)));
    }
    if let Some(name) = &card.structured_name {
        lines.push(format!("N:{}", structured_name_value(name)));
    }
    if let Some(nickname) = &card.nickname {
        lines.push(format!("NICKNAME:{}", escape_value(nickname)));
    }
    if let Some(org) = &card.organization {
        lines.push(format!("ORG:{}", escape_value(org)));
    }
    if let Some(title) = &card.title {
        lines.push(format!("TITLE:{}", escape_value(title)));
    }
    if let Some(note) = &card.note {
        lines.push(format!("NOTE:{}", escape_value(note)));
    }
    if let Some(birthday) = &card.birthday {
        lines.push(format!("BDAY:{}", format_date(birthday)));
    }
    for phone in &card.phones {
        lines.push(format!(
            "{}{}:{}",
            property_params("TEL", phone.label.as_deref(), phone.preferred),
            phone_type_param(phone.phone_type.as_deref()),
            escape_value(&phone.number)
        ));
    }
    for email in &card.emails {
        lines.push(format!(
            "{}{}:{}",
            property_params("EMAIL", email.label.as_deref(), email.preferred),
            "",
            escape_value(&email.address)
        ));
    }
    for address in &card.addresses {
        lines.push(format!(
            "{}{}:{}",
            property_params("ADR", address.label.as_deref(), address.preferred),
            "",
            address_value(address)
        ));
    }
    if let Some(photo) = &card.photo {
        let media = photo.media_type.as_deref().unwrap_or("image/jpeg");
        lines.push(format!(
            "PHOTO;ENCODING=b;TYPE={media}:{}",
            STANDARD.encode(&photo.data)
        ));
    }
    if let Some(extensions) = &card.extensions {
        for (key, value) in &extensions.properties {
            if key.starts_with("X-") {
                let escaped = escape_value(value);
                lines.push(format!("{key}:{escaped}"));
            }
        }
    }
    lines.push("END:VCARD".to_owned());
    Ok(fold_lines(&lines))
}

fn structured_name_value(name: &StructuredName) -> String {
    [
        name.family.as_deref().unwrap_or(""),
        name.given.as_deref().unwrap_or(""),
        name.additional.as_deref().unwrap_or(""),
        name.prefixes.as_deref().unwrap_or(""),
        name.suffixes.as_deref().unwrap_or(""),
    ]
    .join(";")
}

fn address_value(address: &Address) -> String {
    [
        "",
        "",
        address.street.as_deref().unwrap_or(""),
        address.locality.as_deref().unwrap_or(""),
        address.region.as_deref().unwrap_or(""),
        address.postal_code.as_deref().unwrap_or(""),
        address.country.as_deref().unwrap_or(""),
    ]
    .iter()
    .map(|part| escape_value(part))
    .collect::<Vec<_>>()
    .join(";")
}

fn property_params(name: &str, label: Option<&str>, preferred: bool) -> String {
    let mut params = name.to_owned();
    if preferred {
        params.push_str(";PREF=1");
    }
    if let Some(label) = label {
        params.push_str(&format!(";TYPE={}", escape_param(label)));
    }
    params
}

fn phone_type_param(phone_type: Option<&str>) -> String {
    phone_type
        .map(|value| format!(";TYPE={}", escape_param(value)))
        .unwrap_or_default()
}

fn format_date(value: &DateOrDateTime) -> String {
    match value {
        DateOrDateTime::Date(date) => date.format("%Y%m%d").to_string(),
        DateOrDateTime::DateTime(dt) => dt.format("%Y%m%dT%H%M%SZ").to_string(),
    }
}

fn escape_value(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('\n', "\\n")
        .replace(',', "\\,")
        .replace(';', "\\;")
}

fn escape_param(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn fold_lines(lines: &[String]) -> String {
    let mut output = String::new();
    for line in lines {
        if line.len() <= LINE_LIMIT {
            output.push_str(line);
            output.push('\n');
            continue;
        }
        let mut remaining = line.as_str();
        let mut first = true;
        while !remaining.is_empty() {
            let chunk_len = if first {
                LINE_LIMIT
            } else {
                LINE_LIMIT - 1
            };
            let split_at = remaining
                .char_indices()
                .map(|(index, _)| index)
                .take_while(|index| *index <= chunk_len)
                .last()
                .unwrap_or(remaining.len());
            let (chunk, rest) = remaining.split_at(split_at);
            if first {
                output.push_str(chunk);
                first = false;
            } else {
                output.push('\n');
                output.push(' ');
                output.push_str(chunk);
            }
            remaining = rest;
        }
        output.push('\n');
    }
    output
}
