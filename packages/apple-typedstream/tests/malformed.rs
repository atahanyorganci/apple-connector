use apple_typedstream::{Value, from_slice};

const NEW: u8 = 0x84;

fn header() -> Vec<u8> {
    vec![
        4, 11, b's', b't', b'r', b'e', b'a', b'm', b't', b'y', b'p', b'e', b'd', 0x81, 0xe8, 0x03,
    ]
}

fn typed(encoding: &[u8], payload: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut bytes = header();
    bytes.push(NEW);
    if encoding.len() <= 127 {
        bytes.push(
            u8::try_from(encoding.len())
                .map_err(|e| -> Box<dyn std::error::Error> { Box::new(e) })?,
        );
    } else {
        bytes.push(0x81);
        bytes.extend_from_slice(
            &u16::try_from(encoding.len())
                .map_err(|e| -> Box<dyn std::error::Error> { Box::new(e) })?
                .to_le_bytes(),
        );
    }
    bytes.extend_from_slice(encoding);
    bytes.extend_from_slice(payload);
    Ok(bytes)
}

#[test]
fn rejects_bad_header_fields() -> Result<(), Box<dyn std::error::Error>> {
    let mut bytes = header();
    bytes[0] = 2;
    assert!(
        from_slice::<Value>(&bytes)
            .err()
            .ok_or("expected version error")?
            .to_string()
            .contains("version")
    );

    let mut bytes = header();
    bytes[1] = 10;
    assert!(
        from_slice::<Value>(&bytes)
            .err()
            .ok_or("expected signature length error")?
            .to_string()
            .contains("signature length")
    );

    let mut bytes = header();
    bytes[2] = b'x';
    assert!(
        from_slice::<Value>(&bytes)
            .err()
            .ok_or("expected signature error")?
            .to_string()
            .contains("signature")
    );
    Ok(())
}

#[test]
fn rejects_invalid_references() -> Result<(), Box<dyn std::error::Error>> {
    let error = from_slice::<Value>(&typed(b"@", &[0x92])?)
        .err()
        .ok_or("expected out of bounds error")?;
    assert!(error.to_string().contains("out of bounds"));

    let mut bytes = header();
    bytes.push(0x92);
    let error = from_slice::<Value>(&bytes)
        .err()
        .ok_or("expected shared string reference error")?;
    assert!(error.to_string().contains("shared string reference"));
    Ok(())
}

#[test]
fn rejects_unsupported_and_malformed_encodings() -> Result<(), Box<dyn std::error::Error>> {
    let error = from_slice::<Value>(&typed(b"?", &[])?)
        .err()
        .ok_or("expected unsupported type encoding error")?;
    assert!(error.to_string().contains("unsupported type encoding"));

    let error = from_slice::<Value>(&typed(b"[2i", &[])?)
        .err()
        .ok_or("expected unterminated array error")?;
    assert!(error.to_string().contains("unterminated array"));

    let error = from_slice::<Value>(&typed(b"{Point=ii", &[])?)
        .err()
        .ok_or("expected unterminated struct error")?;
    assert!(error.to_string().contains("unterminated struct"));
    Ok(())
}

#[test]
fn rejects_invalid_boolean_and_class_name() -> Result<(), Box<dyn std::error::Error>> {
    let error = from_slice::<Value>(&typed(b"B", &[2])?)
        .err()
        .ok_or("expected boolean error")?;
    assert!(error.to_string().contains("boolean"));

    let payload = [
        NEW, // object
        NEW, // literal class
        NEW, 1, 0xff, // literal shared class name
        0,    // class version
        0x85, // nil superclass
        0x86, // end object
    ];
    let error = from_slice::<Value>(&typed(b"@", &payload)?)
        .err()
        .ok_or("expected class name error")?;
    assert!(error.to_string().contains("class name"));
    Ok(())
}

#[test]
fn enforces_blob_and_nesting_limits() -> Result<(), Box<dyn std::error::Error>> {
    let payload = [0x82, 0x01, 0x00, 0x00, 0x04];
    let error = from_slice::<Value>(&typed(b"+", &payload)?)
        .err()
        .ok_or("expected blob limit error")?;
    assert!(error.to_string().contains("exceeds limit"));

    let mut encoding = Vec::new();
    for _ in 0..130 {
        encoding.extend_from_slice(b"[1");
    }
    encoding.push(b'i');
    encoding.extend(std::iter::repeat_n(b']', 130));
    let error = from_slice::<Value>(&typed(&encoding, &[0])?)
        .err()
        .ok_or("expected nesting limit error")?;
    assert!(error.to_string().contains("nesting limit"));
    Ok(())
}
