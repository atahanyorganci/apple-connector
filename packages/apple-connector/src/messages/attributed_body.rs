use std::collections::HashMap;

use apple_typedstream::{ArchivedObject, TypedValues, Value};

use super::model::{AttributedBodyDecodeError, AttributedRun, BodyAttribute, MessageBody};

pub fn decode(data: &[u8]) -> Result<MessageBody, AttributedBodyDecodeError> {
    let value = apple_typedstream::from_slice(data)
        .map_err(|_| AttributedBodyDecodeError::InvalidTypedStream)?;
    let Value::Archived(object) = value else {
        return Err(AttributedBodyDecodeError::NotAttributedString);
    };
    if !object
        .classes
        .iter()
        .any(|class| class.name == "NSAttributedString")
    {
        return Err(AttributedBodyDecodeError::NotAttributedString);
    }

    let text = attributed_string_text(&object).ok_or(AttributedBodyDecodeError::MissingText)?;
    let runs = parse_runs(&object.fields, &text);
    Ok(MessageBody {
        text: Some(text),
        runs,
        attributed_body_error: None,
    })
}

fn attributed_string_text(object: &ArchivedObject) -> Option<String> {
    let text_field = object.fields.first()?;
    if text_field.encoding != "@" {
        return None;
    }
    match text_field.values.first()? {
        Value::String(text) => Some(text.clone()),
        _ => None,
    }
}

fn parse_runs(fields: &[TypedValues], text: &str) -> Vec<AttributedRun> {
    let utf16_to_byte = build_utf16_to_byte_map(text);
    let mut runs = Vec::new();
    let mut cache: HashMap<i64, Vec<BodyAttribute>> = HashMap::new();
    let mut part_cache: HashMap<i64, Option<i64>> = HashMap::new();
    let mut utf16_pos = 0usize;
    let mut index = 1usize;

    while index < fields.len() {
        let Some((type_index, length)) = as_type_length_pair(&fields[index]) else {
            index += 1;
            continue;
        };
        index += 1;

        let (attributes, part) = if index < fields.len() {
            if let Some(map) = as_attribute_map(&fields[index]) {
                index += 1;
                let (attrs, part) = attributes_from_map(map);
                cache.insert(type_index, attrs.clone());
                part_cache.insert(type_index, part);
                (attrs, part)
            } else if let Some(attrs) = cache.get(&type_index) {
                (
                    attrs.clone(),
                    part_cache.get(&type_index).copied().flatten(),
                )
            } else {
                (Vec::new(), None)
            }
        } else if let Some(attrs) = cache.get(&type_index) {
            (
                attrs.clone(),
                part_cache.get(&type_index).copied().flatten(),
            )
        } else {
            (Vec::new(), None)
        };

        let start = utf16_idx(text, utf16_pos, &utf16_to_byte);
        utf16_pos = utf16_pos.saturating_add(usize::try_from(length).unwrap_or(usize::MAX));
        let end = utf16_idx(text, utf16_pos, &utf16_to_byte);

        runs.push(AttributedRun {
            start,
            end,
            part,
            attributes,
        });
    }

    runs
}

fn as_type_length_pair(field: &TypedValues) -> Option<(i64, u64)> {
    if field.encoding != "iI" || field.values.len() < 2 {
        return None;
    }
    Some((as_i64(&field.values[0])?, as_u64(&field.values[1])?))
}

fn as_attribute_map(field: &TypedValues) -> Option<&std::collections::BTreeMap<String, Value>> {
    if field.encoding != "@" {
        return None;
    }
    field.values.first()?.as_map()
}

fn attributes_from_map(
    map: &std::collections::BTreeMap<String, Value>,
) -> (Vec<BodyAttribute>, Option<i64>) {
    let part = map.get("__kIMMessagePartAttributeName").and_then(as_i64);
    let inline_sticker = map
        .get("__kIMEmojiImageAttributeName")
        .and_then(as_i64)
        .is_some_and(|value| value != 0);
    let is_rich_link = map
        .get("__kIMLinkIsRichLinkAttributeName")
        .and_then(as_i64)
        .is_some_and(|value| value != 0);

    let mut attributes = Vec::new();
    let mut handled_file_transfer = false;
    let mut handled_breadcrumb = false;

    for (key, value) in map {
        match key.as_str() {
            "__kIMMessagePartAttributeName"
            | "__kIMEmojiImageAttributeName"
            | "__kIMLinkIsRichLinkAttributeName" => {}
            "__kIMLinkAttributeName" => {
                if let Some(url) = extract_url(value) {
                    attributes.push(BodyAttribute::Link {
                        url,
                        is_rich: is_rich_link,
                    });
                }
            }
            "__kIMMentionConfirmedMention" => {
                if let Some(mention) = value.as_str() {
                    attributes.push(BodyAttribute::Mention(mention.to_owned()));
                }
            }
            "__kIMFileTransferGUIDAttributeName" => {
                if let Some(guid) = value.as_str() {
                    attributes.push(BodyAttribute::FileTransfer {
                        guid: guid.to_owned(),
                        inline_sticker,
                    });
                    handled_file_transfer = true;
                }
            }
            "__kIMPhoneNumberAttributeName" => attributes.push(BodyAttribute::PhoneNumber),
            "__kIMDataDetectedAttributeName" => attributes.push(BodyAttribute::DataDetected),
            "__kIMCalendarEventAttributeName" => attributes.push(BodyAttribute::CalendarEvent),
            "__kIMBreadcrumbTextMarkerAttributeName" | "__kIMBreadcrumbTextOptionFlags" => {
                if handled_breadcrumb {
                    continue;
                }
                attributes.push(BodyAttribute::Breadcrumb {
                    marker: map
                        .get("__kIMBreadcrumbTextMarkerAttributeName")
                        .and_then(Value::as_str)
                        .map(str::to_owned),
                    flags: map.get("__kIMBreadcrumbTextOptionFlags").and_then(as_i64),
                });
                handled_breadcrumb = true;
            }
            "__kIMFilenameAttributeName"
            | "__kIMInlineMediaHeightAttributeName"
            | "__kIMInlineMediaWidthAttributeName"
            | "IMAudioTranscription" => {
                // Attachment metadata companions; file-transfer GUID is the primary signal.
                if !handled_file_transfer {
                    attributes.push(BodyAttribute::Unknown(key.clone()));
                }
            }
            other => attributes.push(BodyAttribute::Unknown(other.to_owned())),
        }
    }

    (attributes, part)
}

fn extract_url(value: &Value) -> Option<String> {
    match value {
        Value::String(url) => Some(url.clone()),
        Value::Archived(object) if object.classes.iter().any(|class| class.name == "NSURL") => {
            object.fields.iter().find_map(|field| {
                if field.encoding != "@" {
                    return None;
                }
                field.values.iter().find_map(|nested| match nested {
                    Value::String(url) => Some(url.clone()),
                    _ => None,
                })
            })
        }
        _ => None,
    }
}

fn as_i64(value: &Value) -> Option<i64> {
    match value {
        Value::I64(value) => Some(*value),
        Value::U64(value) => i64::try_from(*value).ok(),
        _ => None,
    }
}

fn as_u64(value: &Value) -> Option<u64> {
    match value {
        Value::U64(value) => Some(*value),
        Value::I64(value) if *value >= 0 => Some(*value as u64),
        _ => None,
    }
}

fn build_utf16_to_byte_map(text: &str) -> Vec<usize> {
    let mut map = Vec::with_capacity(text.len() + 1);
    let mut byte = 0;
    for ch in text.chars() {
        let units = ch.len_utf16();
        for _ in 0..units {
            map.push(byte);
        }
        byte += ch.len_utf8();
    }
    map.push(byte);
    map
}

fn utf16_idx(text: &str, idx: usize, map: &[usize]) -> usize {
    *map.get(idx).unwrap_or(&text.len())
}

#[cfg(test)]
mod tests {
    use super::decode;
    use crate::messages::model::BodyAttribute;

    const HELLO_FIXTURE: &[u8] =
        include_bytes!("../../fixtures/messages/attributed-body-hello.bin");
    const LONG_FIXTURE: &[u8] = include_bytes!("../../fixtures/messages/attributed-body-long.bin");
    const SPACED_FIXTURE: &[u8] =
        include_bytes!("../../../apple-typedstream/fixtures/attributed-body-02-spaced.bin");
    const PHOTO_CAPTION_FIXTURE: &[u8] =
        include_bytes!("../../../apple-typedstream/fixtures/attributed-body-18-photo-caption.bin");
    const STICKER_FIXTURE: &[u8] =
        include_bytes!("../../../apple-typedstream/fixtures/attributed-body-19-sticker.bin");
    const EXPRESSIVE_FIXTURE: &[u8] =
        include_bytes!("../../../apple-typedstream/fixtures/attributed-body-20-expressive.bin");
    const PHONE_FIXTURE: &[u8] =
        include_bytes!("../../../apple-typedstream/fixtures/attributed-body-07-phone.bin");
    const TEXT_URL_FIXTURE: &[u8] =
        include_bytes!("../../../apple-typedstream/fixtures/attributed-body-06-text-url.bin");

    #[test]
    fn decodes_attributed_body_fixture() -> Result<(), Box<dyn std::error::Error>> {
        let body = decode(HELLO_FIXTURE)?;
        assert_eq!(body.text.as_deref(), Some("Noter test"));
        assert_eq!(body.runs.len(), 1);
        assert_eq!(body.runs[0].part, Some(0));
        assert!(body.runs[0].attributes.is_empty());
        Ok(())
    }

    #[test]
    fn decodes_i16_length_attributed_body_fixture() -> Result<(), Box<dyn std::error::Error>> {
        let body = decode(LONG_FIXTURE)?;
        let text = body.text.ok_or("missing text")?;
        assert!(
            text.starts_with("Sed nibh velit,"),
            "unexpected decoded text: {text:?}"
        );
        Ok(())
    }

    #[test]
    fn preserves_whitespace_and_object_placeholders() {
        assert_eq!(
            decode(SPACED_FIXTURE).ok().and_then(|body| body.text),
            Some("fixture: spaced  ".to_owned())
        );
        assert_eq!(
            decode(PHOTO_CAPTION_FIXTURE)
                .ok()
                .and_then(|body| body.text),
            Some("\u{fffc}fixture: photo caption".to_owned())
        );
        assert_eq!(
            decode(STICKER_FIXTURE).ok().and_then(|body| body.text),
            Some("\u{fffc}".to_owned())
        );
        assert_eq!(
            decode(EXPRESSIVE_FIXTURE).ok().and_then(|body| body.text),
            Some("\u{fffd}".to_owned())
        );
    }

    #[test]
    fn parses_photo_caption_file_transfer_run() -> Result<(), Box<dyn std::error::Error>> {
        let body = decode(PHOTO_CAPTION_FIXTURE)?;
        assert_eq!(body.runs.len(), 2);
        assert_eq!(
            body.runs[0].attributes,
            vec![BodyAttribute::FileTransfer {
                guid: "714A7477-1CA9-4EA8-8D65-C3FB7DEB0C39".to_owned(),
                inline_sticker: false,
            }]
        );
        assert_eq!(body.runs[0].part, Some(0));
        assert_eq!(
            &body.text.as_ref().ok_or("missing components")?[body.runs[0].start..body.runs[0].end],
            "\u{fffc}"
        );
        assert_eq!(body.runs[1].part, Some(1));
        assert_eq!(
            &body.text.as_ref().ok_or("missing components")?[body.runs[1].start..body.runs[1].end],
            "fixture: photo caption"
        );
        Ok(())
    }

    #[test]
    fn parses_sticker_inline_file_transfer() -> Result<(), Box<dyn std::error::Error>> {
        let body = decode(STICKER_FIXTURE)?;
        assert_eq!(
            body.runs[0].attributes,
            vec![BodyAttribute::FileTransfer {
                guid: "D400984E-62E5-45A9-AE69-BADBA5E69A5C".to_owned(),
                inline_sticker: true,
            }]
        );
        Ok(())
    }

    #[test]
    fn parses_phone_link_run() -> Result<(), Box<dyn std::error::Error>> {
        let body = decode(PHONE_FIXTURE)?;
        assert_eq!(body.runs.len(), 2);
        assert!(body.runs[0].attributes.is_empty());
        assert!(body.runs[1].attributes.iter().any(
            |attr| matches!(attr, BodyAttribute::Link { url, .. } if url.starts_with("tel:"))
        ));
        assert!(
            body.runs[1]
                .attributes
                .contains(&BodyAttribute::PhoneNumber)
        );
        assert_eq!(
            &body.text.as_ref().ok_or("missing components")?[body.runs[1].start..body.runs[1].end],
            "+1 (555) 123-4567"
        );
        Ok(())
    }

    #[test]
    fn reuses_attribute_cache_for_repeated_type_index() -> Result<(), Box<dyn std::error::Error>> {
        let body = decode(TEXT_URL_FIXTURE)?;
        let text = body.text.as_deref().ok_or("missing text")?;
        assert_eq!(text, "fixture: visit https://example.com today");
        assert_eq!(body.runs.len(), 4);
        assert_eq!(
            &text[body.runs[0].start..body.runs[0].end],
            "fixture: visit "
        );
        assert_eq!(
            &text[body.runs[1].start..body.runs[1].end],
            "https://example.com"
        );
        assert_eq!(&text[body.runs[2].start..body.runs[2].end], " ");
        assert_eq!(&text[body.runs[3].start..body.runs[3].end], "today");
        assert!(body.runs[0].attributes.is_empty());
        assert!(body.runs[2].attributes.is_empty());
        assert!(body.runs[1].attributes.iter().any(
            |attr| matches!(attr, BodyAttribute::Link { url, .. } if url == "https://example.com")
        ));
        assert!(
            body.runs[3]
                .attributes
                .contains(&BodyAttribute::CalendarEvent)
        );
        Ok(())
    }

    #[test]
    fn rejects_malformed_and_non_attributed_streams() -> Result<(), Box<dyn std::error::Error>> {
        use crate::messages::model::AttributedBodyDecodeError;

        assert_eq!(
            decode(b"not a typedstream"),
            Err(AttributedBodyDecodeError::InvalidTypedStream)
        );

        let string_stream = apple_typedstream::to_vec("plain NSString")?;
        assert_eq!(
            decode(&string_stream),
            Err(AttributedBodyDecodeError::NotAttributedString)
        );
        Ok(())
    }
}
