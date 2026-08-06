use std::{collections::BTreeMap, io::Cursor};

use apple_typedstream::{
    Deserializer, Serializer, Value, from_reader, from_slice, to_vec, to_writer,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Sample {
    title: String,
    enabled: bool,
    signed: i32,
    unsigned: u32,
    ratio: f64,
    optional: Option<String>,
    values: Vec<i16>,
    labels: BTreeMap<String, String>,
    #[serde(with = "serde_bytes")]
    payload: Vec<u8>,
    state: State,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
enum State {
    Ready,
    Count(i32),
    Point { x: i32, y: i32 },
}

fn sample(state: State) -> Sample {
    Sample {
        title: "Hello, typedstream! 👋".to_owned(),
        enabled: true,
        signed: -32_000,
        unsigned: 65_000,
        ratio: 3.25,
        optional: None,
        values: vec![-2, 0, 9],
        labels: BTreeMap::from([
            ("first".to_owned(), "one".to_owned()),
            ("second".to_owned(), "two".to_owned()),
        ]),
        payload: vec![0, 1, 2, 127, 128, 255],
        state,
    }
}

#[test]
fn round_trips_nested_serde_data() -> Result<(), Box<dyn std::error::Error>> {
    for state in [State::Ready, State::Count(42), State::Point { x: -4, y: 8 }] {
        let expected = sample(state);
        let bytes = to_vec(&expected)?;
        let actual: Sample = from_slice(&bytes)?;
        assert_eq!(actual, expected);
    }
    Ok(())
}

#[test]
fn reader_and_writer_apis_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let expected = vec!["one".to_owned(), "two".to_owned()];
    let mut bytes = Vec::new();
    to_writer(&mut bytes, &expected)?;
    let actual: Vec<String> = from_reader(Cursor::new(bytes))?;
    assert_eq!(actual, expected);
    Ok(())
}

#[test]
fn dynamic_byte_value_uses_nsdata() -> Result<(), Box<dyn std::error::Error>> {
    let expected = Value::Bytes(vec![0, 1, 127, 128, 255]);
    let bytes = to_vec(&expected)?;
    let actual: serde_bytes::ByteBuf = from_slice(&bytes)?;
    assert_eq!(actual.as_ref(), &[0, 1, 127, 128, 255]);
    Ok(())
}

#[test]
fn serializer_and_deserializer_implement_serde_traits() -> Result<(), Box<dyn std::error::Error>> {
    let expected = sample(State::Count(7));
    let mut bytes = Vec::new();
    expected.serialize(Serializer::new(&mut bytes))?;
    let actual = Sample::deserialize(Deserializer::from_slice(&bytes)?)?;
    assert_eq!(actual, expected);
    Ok(())
}

#[test]
fn parses_real_fixture_as_dynamic_value() -> Result<(), Box<dyn std::error::Error>> {
    let bytes = include_bytes!("../fixtures/attributed-body-01-plain-short.bin");
    let value: Value = from_slice(bytes)?;
    let Value::Archived(object) = value else {
        panic!("expected archived object");
    };
    assert_eq!(object.classes[0].name, "NSAttributedString");
    assert_eq!(object.classes[1].name, "NSObject");
    assert!(!object.fields.is_empty());
    Ok(())
}

#[test]
fn rejects_invalid_header_and_truncated_values() -> Result<(), Box<dyn std::error::Error>> {
    let error = from_slice::<Value>(b"not a typedstream")
        .err()
        .ok_or("expected header error")?;
    assert!(error.to_string().contains("header"));

    let bytes = to_vec(&"truncated")?;
    let error = from_slice::<String>(&bytes[..bytes.len() - 1])
        .err()
        .ok_or("expected truncated stream error")?;
    assert!(error.to_string().contains("end of typedstream"));
    Ok(())
}
