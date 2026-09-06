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

const NIL: u8 = 0x85;
const END_OF_OBJECT: u8 = 0x86;

fn assert_limit(error: &apple_typedstream::Error, expected: &str) {
    assert!(
        matches!(
            error,
            apple_typedstream::Error::LimitExceeded { limit, .. } if *limit == expected
        ),
        "expected the {expected} limit, got: {error}"
    );
}

#[test]
fn enforces_the_blob_limit() -> Result<(), Box<dyn std::error::Error>> {
    // A declared length of 0x04000001, one byte past the 64 MiB ceiling.
    let payload = [0x82, 0x01, 0x00, 0x00, 0x04];
    let error = from_slice::<Value>(&typed(b"+", &payload)?)
        .err()
        .ok_or("expected blob limit error")?;
    assert_limit(&error, "blob length");
    Ok(())
}

#[test]
fn byte_arrays_cannot_bypass_the_blob_limit() -> Result<(), Box<dyn std::error::Error>> {
    // The `[N c]` fast path reads bytes directly and used to skip the ceiling
    // that `read_unshared_string` enforced for the same data.
    let error = from_slice::<Value>(&typed(b"[67108865c]", &[])?)
        .err()
        .ok_or("expected blob limit error")?;
    assert_limit(&error, "blob length");
    Ok(())
}

#[test]
fn declared_array_lengths_do_not_preallocate() -> Result<(), Box<dyn std::error::Error>> {
    // Fourteen bytes of encoding. Reserving from the declared length asked for
    // four billion values and aborted the process before reading anything.
    let error = from_slice::<Value>(&typed(b"[4294967295@]", &[])?)
        .err()
        .ok_or("expected an end-of-stream error")?;
    assert!(
        error.to_string().contains("unexpected end"),
        "unexpected error: {error}"
    );
    Ok(())
}

#[test]
fn enforces_the_encoding_nesting_limit() -> Result<(), Box<dyn std::error::Error>> {
    let mut encoding = Vec::new();
    for _ in 0..130 {
        encoding.extend_from_slice(b"[1");
    }
    encoding.push(b'i');
    encoding.extend(std::iter::repeat_n(b']', 130));
    let error = from_slice::<Value>(&typed(&encoding, &[0])?)
        .err()
        .ok_or("expected encoding nesting limit error")?;
    assert_limit(&error, "type encoding nesting depth");
    Ok(())
}

#[test]
fn enforces_the_encoding_item_limit() -> Result<(), Box<dyn std::error::Error>> {
    let encoding = vec![b'i'; 5000];
    let error = from_slice::<Value>(&typed(&encoding, &[])?)
        .err()
        .ok_or("expected encoding item limit error")?;
    assert_limit(&error, "type encoding item count");
    Ok(())
}

#[test]
fn enforces_the_class_chain_limit() -> Result<(), Box<dyn std::error::Error>> {
    let mut payload = vec![NEW];
    for _ in 0..300 {
        payload.extend_from_slice(&[NEW, NEW, 1, b'A', 0]);
    }
    let error = from_slice::<Value>(&typed(b"@", &payload)?)
        .err()
        .ok_or("expected class chain limit error")?;
    assert_limit(&error, "class chain length");
    Ok(())
}

#[test]
fn enforces_the_object_nesting_limit() -> Result<(), Box<dyn std::error::Error>> {
    // Each nested object costs three levels of reader depth, so sixty objects
    // clears the 128-level ceiling. The encoding stays one byte wide, which is
    // why the encoding-depth budget cannot catch this shape.
    // Shared string 0 is the stream's own "@" encoding, written by `typed`.
    // Object 1 is the class chain declared here; both are then referenced by
    // every nested level, which keeps each level three bytes wide.
    const ENCODING_REF: u8 = 0x92;
    const CLASS_REF: u8 = 0x93;

    let mut payload = vec![
        NEW, // object
        NEW, // literal class
        NEW, 1, b'A', // class name
        0,    // class version
        NIL,  // no superclass
    ];
    for _ in 0..60 {
        payload.push(ENCODING_REF);
        payload.push(NEW); // nested object
        payload.push(CLASS_REF);
    }
    payload.push(ENCODING_REF);
    payload.push(NIL);
    payload.extend(std::iter::repeat_n(END_OF_OBJECT, 61));

    let error = from_slice::<Value>(&typed(b"@", &payload)?)
        .err()
        .ok_or("expected value nesting limit error")?;
    assert_limit(&error, "value nesting depth");
    Ok(())
}
