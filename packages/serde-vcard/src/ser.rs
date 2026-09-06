use std::io::Write;

use base64::{Engine, engine::general_purpose::STANDARD};

use crate::{
    error::{Error, Result},
    model::{Address, DateOrDateTime, Photo, RawProperty, SocialProfile, StructuredName, VCard},
};

const LINE_LIMIT: usize = 75;

pub fn to_writer<W: Write>(mut writer: W, card: &VCard) -> Result<()> {
    let vcf = card_to_vcard(card)?;
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
            "{}:{}",
            property_params("TEL", phone.label.as_deref(), phone.preferred),
            escape_value(&phone.number)
        ));
    }
    for email in &card.emails {
        lines.push(format!(
            "{}:{}",
            property_params("EMAIL", email.label.as_deref(), email.preferred),
            escape_value(&email.address)
        ));
    }
    for address in &card.addresses {
        lines.push(format!(
            "{}:{}",
            property_params("ADR", address.label.as_deref(), address.preferred),
            address_value(address)
        ));
    }
    for url in &card.urls {
        lines.push(format!(
            "{}:{}",
            property_params("URL", url.label.as_deref(), url.preferred),
            escape_value(&url.url)
        ));
    }
    for profile in &card.social_profiles {
        lines.push(social_profile_line(profile));
    }
    if let Some(photo) = &card.photo {
        lines.push(photo_line(photo));
    }
    if let Some(extensions) = &card.extensions {
        for (key, value) in &extensions.properties {
            if key.starts_with("X-") {
                let escaped = escape_value(value);
                lines.push(format!("{key}:{escaped}"));
            }
        }
    }
    for property in &card.unknown {
        lines.push(raw_property_line(property));
    }
    lines.push("END:VCARD".to_owned());
    Ok(fold_lines(&lines))
}

/// RFC 6350 §6.2.4: PHOTO is a URI, so inline bytes are written as a `data:`
/// URI rather than the vCard 3.0 `ENCODING=b` form the header no longer claims.
fn photo_line(photo: &Photo) -> String {
    match photo {
        Photo::Uri { uri } => format!("PHOTO:{uri}"),
        Photo::Inline { data, media_type } => {
            let media = media_type.as_deref().unwrap_or("image/jpeg");
            format!("PHOTO:data:{media};base64,{}", STANDARD.encode(data))
        }
    }
}

fn social_profile_line(profile: &SocialProfile) -> String {
    let mut property = "X-SOCIALPROFILE".to_owned();
    if profile.preferred {
        property.push_str(";PREF=1");
    }
    if let Some(service) = &profile.service {
        property.push_str(&format!(";TYPE={}", escape_param(service)));
    }
    if let Some(username) = &profile.username {
        property.push_str(&format!(";x-user={}", escape_param(username)));
    }
    format!(
        "{property}:{}",
        escape_value(profile.url.as_deref().unwrap_or(""))
    )
}

fn raw_property_line(property: &RawProperty) -> String {
    let mut line = String::new();
    if let Some(group) = &property.group {
        line.push_str(group);
        line.push('.');
    }
    line.push_str(&property.name);
    for parameter in &property.parameters {
        line.push(';');
        line.push_str(parameter);
    }
    line.push(':');
    line.push_str(&property.value);
    line
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
            let chunk_len = if first { LINE_LIMIT } else { LINE_LIMIT - 1 };
            // Split on character *ends* so each chunk advances. Using start
            // indices left a 1-char remainder with split_at=0 (infinite loop).
            let split_at = remaining
                .char_indices()
                .map(|(index, ch)| index + ch.len_utf8())
                .take_while(|end| *end <= chunk_len)
                .last()
                .unwrap_or_else(|| {
                    remaining
                        .chars()
                        .next()
                        .map(|ch| ch.len_utf8())
                        .unwrap_or(remaining.len())
                });
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
