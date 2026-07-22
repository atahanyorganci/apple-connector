use apple_typedstream::Value;

pub fn decode(data: &[u8]) -> Option<String> {
    let value = apple_typedstream::from_slice(data).ok()?;
    let Value::Archived(object) = value else {
        return None;
    };
    if !object
        .classes
        .iter()
        .any(|class| class.name == "NSAttributedString")
    {
        return None;
    }

    let text_field = object.fields.first()?;
    if text_field.encoding != "@" {
        return None;
    }

    match text_field.values.first()? {
        Value::String(text) => Some(text.clone()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::decode;

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

    #[test]
    fn decodes_attributed_body_fixture() {
        let decoded = decode(HELLO_FIXTURE).expect("decode attributed body fixture");

        assert_eq!(decoded, "Noter test");
    }

    #[test]
    fn decodes_i16_length_attributed_body_fixture() {
        let decoded = decode(LONG_FIXTURE).expect("decode long attributed body fixture");

        assert!(
            decoded.starts_with("Sed nibh velit,"),
            "unexpected decoded text: {decoded:?}"
        );
    }

    #[test]
    fn preserves_whitespace_and_object_placeholders() {
        assert_eq!(decode(SPACED_FIXTURE).as_deref(), Some("fixture: spaced  "));
        assert_eq!(
            decode(PHOTO_CAPTION_FIXTURE).as_deref(),
            Some("\u{fffc}fixture: photo caption")
        );
        assert_eq!(decode(STICKER_FIXTURE).as_deref(), Some("\u{fffc}"));
        assert_eq!(decode(EXPRESSIVE_FIXTURE).as_deref(), Some("\u{fffd}"));
    }

    #[test]
    fn rejects_malformed_and_non_attributed_streams() {
        assert_eq!(decode(b"not a typedstream"), None);

        let string_stream = apple_typedstream::to_vec("plain NSString").expect("encode NSString");
        assert_eq!(decode(&string_stream), None);
    }
}
