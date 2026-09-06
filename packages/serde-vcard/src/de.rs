use base64::{Engine, engine::general_purpose::STANDARD};
use chrono::{NaiveDate, Utc};

use crate::{
    error::{Error, Result},
    model::{
        Address, DateOrDateTime, Email, ExtensionBag, Photo, RawProperty, SocialProfile,
        StructuredName, Telephone, Url, VCard,
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
    let mut cards = Vec::new();
    let mut current: Option<Vec<Line>> = None;

    for raw in unfold_lines(input) {
        if raw.trim().is_empty() {
            continue;
        }
        if raw.eq_ignore_ascii_case("BEGIN:VCARD") {
            if current.is_some() {
                return Err(Error::Parse(
                    "BEGIN:VCARD inside an unterminated card".to_owned(),
                ));
            }
            current = Some(Vec::new());
            continue;
        }
        if raw.eq_ignore_ascii_case("END:VCARD") {
            let lines = current
                .take()
                .ok_or_else(|| Error::Parse("END:VCARD without BEGIN:VCARD".to_owned()))?;
            cards.push(build_card(lines)?);
            continue;
        }
        let Some(lines) = current.as_mut() else {
            continue;
        };
        lines.push(split_property(&raw)?);
    }

    if current.is_some() {
        return Err(Error::Parse("vCard ended without END:VCARD".to_owned()));
    }
    Ok(cards)
}

/// A single logical property line, already unfolded and split.
struct Line {
    group: Option<String>,
    name: String,
    params: Params,
    value: String,
}

/// Parameters as written, so a bare `TEL;WORK:` (vCard 2.1) and a
/// `TEL;TYPE=WORK:` (3.0/4.0) both round-trip.
#[derive(Default)]
struct Params {
    entries: Vec<(String, Option<String>)>,
}

impl Params {
    fn first(&self, key: &str) -> Option<&str> {
        self.entries
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case(key))
            .and_then(|(_, value)| value.as_deref())
    }

    /// TYPE values, including the bare `;WORK` form older vCards use.
    fn types(&self) -> Vec<String> {
        let mut types = Vec::new();
        for (name, value) in &self.entries {
            match value {
                Some(value) if name.eq_ignore_ascii_case("TYPE") => types.extend(
                    value
                        .split(',')
                        .map(|entry| unescape_param(entry.trim()))
                        .filter(|entry| !entry.is_empty()),
                ),
                // A segment with no `=` is a bare type token.
                None if !name.eq_ignore_ascii_case("PREF") => types.push(name.clone()),
                _ => {}
            }
        }
        types
    }

    fn preferred(&self) -> bool {
        self.entries.iter().any(|(name, value)| {
            name.eq_ignore_ascii_case("PREF")
                && value.as_deref().is_none_or(|value| value.trim() != "0")
        })
    }

    /// Render back to the `KEY=VALUE` segments the property was written with.
    fn segments(&self) -> Vec<String> {
        self.entries
            .iter()
            .map(|(name, value)| match value {
                Some(value) => format!("{name}={value}"),
                None => name.clone(),
            })
            .collect()
    }
}

fn unfold_lines(input: &str) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for raw in input.split('\n') {
        let raw = raw.strip_suffix('\r').unwrap_or(raw);
        // A folded continuation is exactly one space or tab followed by the
        // rest of the value; trimming further would eat significant spaces.
        if let Some(rest) = raw.strip_prefix(' ').or_else(|| raw.strip_prefix('\t'))
            && let Some(last) = lines.last_mut()
        {
            last.push_str(rest);
            continue;
        }
        lines.push(raw.to_owned());
    }
    lines
}

/// Split on `separator`, ignoring separators inside a quoted parameter value.
fn split_unquoted(input: &str, separator: char) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut in_quotes = false;
    let mut start = 0;
    for (index, character) in input.char_indices() {
        match character {
            '"' => in_quotes = !in_quotes,
            _ if character == separator && !in_quotes => {
                parts.push(input.get(start..index).unwrap_or_default());
                start = index + character.len_utf8();
            }
            _ => {}
        }
    }
    parts.push(input.get(start..).unwrap_or_default());
    parts
}

/// Split `group.NAME;PARAM="a:b":value`.
///
/// The value separator is the first unquoted colon: a quoted parameter value is
/// allowed to contain one, and splitting on the first colon regardless cut the
/// property in the wrong place.
fn split_property(line: &str) -> Result<Line> {
    let mut in_quotes = false;
    let mut split_at = None;
    for (index, character) in line.char_indices() {
        match character {
            '"' => in_quotes = !in_quotes,
            ':' if !in_quotes => {
                split_at = Some(index);
                break;
            }
            _ => {}
        }
    }
    let split_at =
        split_at.ok_or_else(|| Error::Parse(format!("invalid property line: {line}")))?;
    let left = line.get(..split_at).unwrap_or_default();
    let value = line.get(split_at + 1..).unwrap_or_default().to_owned();

    let mut segments = split_unquoted(left, ';').into_iter();
    let name_part = segments.next().unwrap_or_default();
    let (group, name) = match name_part.split_once('.') {
        Some((group, name)) => (Some(group.to_owned()), name),
        None => (None, name_part),
    };

    let entries = segments
        .filter(|segment| !segment.is_empty())
        .map(|segment| match segment.split_once('=') {
            Some((key, value)) => (key.trim().to_owned(), Some(value.to_owned())),
            None => (segment.trim().to_owned(), None),
        })
        .collect();

    Ok(Line {
        group,
        name: name.trim().to_owned(),
        params: Params { entries },
        value,
    })
}

/// Apple writes custom labels as `_$!<Home>!$_`.
fn decode_apple_label(value: &str) -> String {
    value
        .strip_prefix("_$!<")
        .and_then(|rest| rest.strip_suffix(">!$_"))
        .unwrap_or(value)
        .to_owned()
}

fn build_card(lines: Vec<Line>) -> Result<VCard> {
    // Apple ties a custom label to a property through a shared group prefix:
    // `item1.TEL` and `item1.X-ABLabel` describe the same phone number.
    let mut group_labels = std::collections::BTreeMap::new();
    for line in &lines {
        if line.name.eq_ignore_ascii_case("X-ABLabel")
            && let Some(group) = &line.group
        {
            group_labels.insert(
                group.clone(),
                decode_apple_label(&unescape_value(&line.value)),
            );
        }
    }

    let mut card = VCard::default();
    for line in &lines {
        apply_line(&mut card, line, &group_labels)?;
    }
    Ok(card)
}

fn label_for(
    line: &Line,
    group_labels: &std::collections::BTreeMap<String, String>,
) -> Option<String> {
    line.group
        .as_ref()
        .and_then(|group| group_labels.get(group))
        .cloned()
        .or_else(|| line.params.types().into_iter().next())
}

fn apply_line(
    card: &mut VCard,
    line: &Line,
    group_labels: &std::collections::BTreeMap<String, String>,
) -> Result<()> {
    let label = label_for(line, group_labels);
    let preferred = line.params.preferred();

    match line.name.to_ascii_uppercase().as_str() {
        "VERSION" => {}
        // Consumed into the label of the property sharing its group.
        "X-ABLABEL" if line.group.is_some() => {}
        "UID" => card.uid = Some(unescape_value(&line.value)),
        "FN" => card.formatted_name = Some(unescape_value(&line.value)),
        "N" => card.structured_name = Some(parse_structured_name(&line.value)),
        "NICKNAME" => card.nickname = Some(unescape_value(&line.value)),
        "ORG" => card.organization = Some(unescape_value(&line.value)),
        "TITLE" => card.title = Some(unescape_value(&line.value)),
        "NOTE" => card.note = Some(unescape_value(&line.value)),
        "BDAY" => card.birthday = parse_date(&line.value),
        "TEL" => card.phones.push(Telephone {
            number: unescape_value(&line.value),
            label,
            preferred,
        }),
        "EMAIL" => card.emails.push(Email {
            address: unescape_value(&line.value),
            label,
            preferred,
        }),
        "ADR" => card
            .addresses
            .push(parse_address(&line.value, label, preferred)),
        "URL" => card.urls.push(Url {
            url: unescape_value(&line.value),
            label,
            preferred,
        }),
        "X-SOCIALPROFILE" | "IMPP" => card
            .social_profiles
            .push(parse_social_profile(line, label, preferred)),
        "PHOTO" => card.photo = Some(parse_photo(line)?),
        name if name.starts_with("X-") => {
            let bag = card.extensions.get_or_insert_with(ExtensionBag::default);
            bag.properties
                .insert(name.to_owned(), unescape_value(&line.value));
        }
        _ => card.unknown.push(RawProperty {
            group: line.group.clone(),
            name: line.name.clone(),
            parameters: line.params.segments(),
            value: line.value.clone(),
        }),
    }
    Ok(())
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

fn parse_address(value: &str, label: Option<String>, preferred: bool) -> Address {
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

fn parse_social_profile(line: &Line, label: Option<String>, preferred: bool) -> SocialProfile {
    let url = unescape_value(&line.value);
    let service = line
        .params
        .first("X-SERVICE-TYPE")
        .map(unescape_param)
        .or_else(|| line.params.types().into_iter().next());
    let username = line.params.first("x-user").map(unescape_param).or_else(|| {
        url.rsplit('/')
            .next()
            .filter(|handle| !handle.is_empty())
            .map(str::to_owned)
    });

    SocialProfile {
        service,
        username,
        url: Some(url),
        label,
        preferred,
    }
}

/// Accept both the vCard 4 URI form and the vCard 3 `ENCODING=b` form.
fn parse_photo(line: &Line) -> Result<Photo> {
    let value = line.value.trim();
    let media_type = line
        .params
        .first("MEDIATYPE")
        .or_else(|| line.params.first("TYPE"))
        .map(normalize_media_type);

    let is_base64 = line
        .params
        .first("ENCODING")
        .is_some_and(|encoding| encoding.eq_ignore_ascii_case("b") || encoding == "BASE64")
        || line
            .params
            .entries
            .iter()
            .any(|(name, _)| name.eq_ignore_ascii_case("BASE64"));

    if let Some(rest) = value.strip_prefix("data:") {
        let (metadata, payload) = rest
            .split_once(',')
            .ok_or_else(|| Error::Parse("PHOTO data URI has no comma separator".to_owned()))?;
        let media_type = metadata
            .split(';')
            .next()
            .filter(|entry| !entry.is_empty())
            .map(str::to_owned)
            .or(media_type);
        let data = STANDARD
            .decode(payload.trim())
            .map_err(|error| Error::Parse(format!("invalid PHOTO data URI: {error}")))?;
        return Ok(Photo::Inline { data, media_type });
    }

    if is_base64 {
        let data = STANDARD
            .decode(value)
            .map_err(|error| Error::Parse(format!("invalid PHOTO base64: {error}")))?;
        return Ok(Photo::Inline { data, media_type });
    }

    Ok(Photo::Uri {
        uri: value.to_owned(),
    })
}

/// vCard 3 wrote bare image formats (`TYPE=JPEG`); vCard 4 wants a media type.
fn normalize_media_type(value: &str) -> String {
    let value = unescape_param(value);
    if value.contains('/') {
        value
    } else {
        format!("image/{}", value.to_ascii_lowercase())
    }
}

fn parse_date(value: &str) -> Option<DateOrDateTime> {
    if value.contains('T') {
        chrono::DateTime::parse_from_rfc3339(value)
            .ok()
            .map(|dt| DateOrDateTime::DateTime(dt.with_timezone(&Utc)))
    } else {
        NaiveDate::parse_from_str(value, "%Y%m%d")
            .ok()
            .or_else(|| NaiveDate::parse_from_str(value, "%Y-%m-%d").ok())
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
    value
        .trim_matches('"')
        .replace("\\\"", "\"")
        .replace("\\\\", "\\")
}

#[cfg(test)]
mod tests {
    use super::{parse_vcard, parse_vcards};

    #[test]
    fn parses_vcard_3_and_4() -> Result<(), Box<dyn std::error::Error>> {
        let v3 = "BEGIN:VCARD\r\nVERSION:3.0\r\nFN:Alice\r\nEND:VCARD\r\n";
        let card = parse_vcard(v3.as_bytes())?;
        assert_eq!(card.formatted_name.as_deref(), Some("Alice"));

        let v4 = "BEGIN:VCARD\nVERSION:4.0\nFN:Bob\nEND:VCARD\n";
        let card = parse_vcard(v4.as_bytes())?;
        assert_eq!(card.formatted_name.as_deref(), Some("Bob"));
        Ok(())
    }

    #[test]
    fn parses_multiple_cards() -> Result<(), Box<dyn std::error::Error>> {
        let input = "BEGIN:VCARD\nFN:One\nEND:VCARD\nBEGIN:VCARD\nFN:Two\nEND:VCARD\n";
        let cards = parse_vcards(input)?;
        assert_eq!(cards.len(), 2);
        Ok(())
    }

    #[test]
    fn preserves_x_properties() -> Result<(), Box<dyn std::error::Error>> {
        let input = "BEGIN:VCARD\nFN:Test\nX-CUSTOM:hello\nEND:VCARD\n";
        let card = parse_vcard(input.as_bytes())?;
        let extensions = card.extensions.ok_or("missing extensions")?;
        assert_eq!(
            extensions.properties.get("X-CUSTOM").map(String::as_str),
            Some("hello")
        );
        Ok(())
    }
}
