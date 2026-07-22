const STREAMTYPED_MAGIC: &[u8] = b"streamtyped";
const STRING_MARKER: u8 = b'+';
const I_16: u8 = 0x81;

const METADATA_PREFIXES: &[&str] = &[
    "NS",
    "NSMutable",
    "__kIM",
    "NSDictionary",
    "NSNumber",
    "NSValue",
    "NSObject",
];

pub fn decode(data: &[u8]) -> Option<String> {
    if !looks_like_typedstream(data) {
        return None;
    }

    let mut best: Option<String> = None;

    for index in 0..data.len() {
        if data[index] != STRING_MARKER {
            continue;
        }

        let Some(text) = read_string_at(data, index) else {
            continue;
        };

        if !is_message_text(&text) {
            continue;
        }

        best = Some(match best {
            Some(current) if current.len() >= text.len() => current,
            _ => text,
        });
    }

    best
}

fn looks_like_typedstream(data: &[u8]) -> bool {
    data.len() >= 2 + STREAMTYPED_MAGIC.len()
        && &data[2..2 + STREAMTYPED_MAGIC.len()] == STREAMTYPED_MAGIC
}

fn read_string_at(data: &[u8], marker_index: usize) -> Option<String> {
    let payload = data.get(marker_index + 1..)?;
    let (length, header_len) = read_length(payload)?;
    let text_bytes = payload.get(header_len..header_len + length)?;
    let text = std::str::from_utf8(text_bytes).ok()?;
    Some(text.to_owned())
}

fn read_length(data: &[u8]) -> Option<(usize, usize)> {
    let first = *data.first()?;

    if first == I_16 {
        let bytes = data.get(1..3)?;
        let length = u16::from_le_bytes([bytes[0], bytes[1]]) as usize;
        return Some((length, 3));
    }

    if first <= 0x7f {
        return Some((usize::from(first), 1));
    }

    None
}

fn is_message_text(text: &str) -> bool {
    if text.is_empty() {
        return false;
    }

    if METADATA_PREFIXES
        .iter()
        .any(|prefix| text.starts_with(prefix))
    {
        return false;
    }

    text.chars()
        .any(|character| !character.is_ascii_control() || character == '\n' || character == '\t')
}

#[cfg(test)]
mod tests {
    use super::decode;

    const HELLO_FIXTURE: &[u8] =
        include_bytes!("../../fixtures/messages/attributed-body-hello.bin");
    const LONG_FIXTURE: &[u8] = include_bytes!("../../fixtures/messages/attributed-body-long.bin");

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
}
