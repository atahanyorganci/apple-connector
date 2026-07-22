use apple_typedstream::{Value, from_slice};

const NEW: u8 = 0x84;

fn header() -> Vec<u8> {
    vec![
        4, 11, b's', b't', b'r', b'e', b'a', b'm', b't', b'y', b'p', b'e', b'd', 0x81, 0xe8, 0x03,
    ]
}

fn typed(encoding: &[u8], payload: &[u8]) -> Vec<u8> {
    let mut bytes = header();
    bytes.push(NEW);
    if encoding.len() <= 127 {
        bytes.push(encoding.len() as u8);
    } else {
        bytes.push(0x81);
        bytes.extend_from_slice(&(encoding.len() as u16).to_le_bytes());
    }
    bytes.extend_from_slice(encoding);
    bytes.extend_from_slice(payload);
    bytes
}

#[test]
fn rejects_bad_header_fields() {
    let mut bytes = header();
    bytes[0] = 2;
    assert!(
        from_slice::<Value>(&bytes)
            .unwrap_err()
            .to_string()
            .contains("version")
    );

    let mut bytes = header();
    bytes[1] = 10;
    assert!(
        from_slice::<Value>(&bytes)
            .unwrap_err()
            .to_string()
            .contains("signature length")
    );

    let mut bytes = header();
    bytes[2] = b'x';
    assert!(
        from_slice::<Value>(&bytes)
            .unwrap_err()
            .to_string()
            .contains("signature")
    );
}

#[test]
fn rejects_invalid_references() {
    let error = from_slice::<Value>(&typed(b"@", &[0x92])).unwrap_err();
    assert!(error.to_string().contains("out of bounds"));

    let mut bytes = header();
    bytes.push(0x92);
    let error = from_slice::<Value>(&bytes).unwrap_err();
    assert!(error.to_string().contains("shared string reference"));
}

#[test]
fn rejects_unsupported_and_malformed_encodings() {
    let error = from_slice::<Value>(&typed(b"?", &[])).unwrap_err();
    assert!(error.to_string().contains("unsupported type encoding"));

    let error = from_slice::<Value>(&typed(b"[2i", &[])).unwrap_err();
    assert!(error.to_string().contains("unterminated array"));

    let error = from_slice::<Value>(&typed(b"{Point=ii", &[])).unwrap_err();
    assert!(error.to_string().contains("unterminated struct"));
}

#[test]
fn rejects_invalid_boolean_and_class_name() {
    let error = from_slice::<Value>(&typed(b"B", &[2])).unwrap_err();
    assert!(error.to_string().contains("boolean"));

    let payload = [
        NEW, // object
        NEW, // literal class
        NEW, 1, 0xff, // literal shared class name
        0,    // class version
        0x85, // nil superclass
        0x86, // end object
    ];
    let error = from_slice::<Value>(&typed(b"@", &payload)).unwrap_err();
    assert!(error.to_string().contains("class name"));
}

#[test]
fn enforces_blob_and_nesting_limits() {
    let payload = [0x82, 0x01, 0x00, 0x00, 0x04];
    let error = from_slice::<Value>(&typed(b"+", &payload)).unwrap_err();
    assert!(error.to_string().contains("exceeds limit"));

    let mut encoding = Vec::new();
    for _ in 0..130 {
        encoding.extend_from_slice(b"[1");
    }
    encoding.push(b'i');
    encoding.extend(std::iter::repeat_n(b']', 130));
    let error = from_slice::<Value>(&typed(&encoding, &[0])).unwrap_err();
    assert!(error.to_string().contains("nesting limit"));
}
