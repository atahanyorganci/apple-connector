use std::{
    collections::{BTreeMap, HashMap},
    io::Write,
};

use serde::ser::{
    self, Serialize, SerializeMap, SerializeSeq, SerializeStruct, SerializeStructVariant,
    SerializeTuple, SerializeTupleStruct, SerializeTupleVariant,
};

use crate::{
    error::{Error, Result},
    value::Value,
};

const TAG_INTEGER_2: u8 = 0x81;
const TAG_INTEGER_4: u8 = 0x82;
const TAG_FLOATING_POINT: u8 = 0x83;
const TAG_NEW: u8 = 0x84;
const TAG_NIL: u8 = 0x85;
const TAG_END_OF_OBJECT: u8 = 0x86;
const FIRST_REFERENCE: i64 = -110;

fn collection_len(len: usize) -> Result<i64> {
    i64::try_from(len)
        .map_err(|_| Error::custom("collection length exceeds typedstream integer range"))
}

fn reference_index(index: usize) -> Result<i64> {
    collection_len(index).map(|index| FIRST_REFERENCE + index)
}

pub struct Serializer<W> {
    writer: W,
}

impl<W: Write> Serializer<W> {
    pub fn new(writer: W) -> Self {
        Self { writer }
    }

    pub fn serialize<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<()> {
        let value = value.serialize(ValueSerializer)?;
        Encoder::new(&mut self.writer).encode(&value)
    }

    pub fn into_inner(self) -> W {
        self.writer
    }
}

pub(crate) fn to_writer<W, T>(writer: W, value: &T) -> Result<()>
where
    W: Write,
    T: ?Sized + Serialize,
{
    Serializer::new(writer).serialize(value)
}

impl<W: Write> ser::Serializer for Serializer<W> {
    type Ok = ();
    type Error = Error;
    type SerializeSeq = RootSequence<W>;
    type SerializeTuple = RootSequence<W>;
    type SerializeTupleStruct = RootSequence<W>;
    type SerializeTupleVariant = RootTupleVariant<W>;
    type SerializeMap = RootMap<W>;
    type SerializeStruct = RootMap<W>;
    type SerializeStructVariant = RootStructVariant<W>;

    fn serialize_bool(self, value: bool) -> Result<()> {
        finish(self.writer, Value::Bool(value))
    }
    fn serialize_i8(self, value: i8) -> Result<()> {
        finish(self.writer, Value::I64(i64::from(value)))
    }
    fn serialize_i16(self, value: i16) -> Result<()> {
        finish(self.writer, Value::I64(i64::from(value)))
    }
    fn serialize_i32(self, value: i32) -> Result<()> {
        finish(self.writer, Value::I64(i64::from(value)))
    }
    fn serialize_i64(self, value: i64) -> Result<()> {
        finish(self.writer, Value::I64(value))
    }
    fn serialize_u8(self, value: u8) -> Result<()> {
        finish(self.writer, Value::U64(u64::from(value)))
    }
    fn serialize_u16(self, value: u16) -> Result<()> {
        finish(self.writer, Value::U64(u64::from(value)))
    }
    fn serialize_u32(self, value: u32) -> Result<()> {
        finish(self.writer, Value::U64(u64::from(value)))
    }
    fn serialize_u64(self, value: u64) -> Result<()> {
        finish(self.writer, Value::U64(value))
    }
    fn serialize_f32(self, value: f32) -> Result<()> {
        finish(self.writer, Value::F64(f64::from(value)))
    }
    fn serialize_f64(self, value: f64) -> Result<()> {
        finish(self.writer, Value::F64(value))
    }
    fn serialize_char(self, value: char) -> Result<()> {
        finish(self.writer, Value::String(value.to_string()))
    }
    fn serialize_str(self, value: &str) -> Result<()> {
        finish(self.writer, Value::String(value.to_owned()))
    }
    fn serialize_bytes(self, value: &[u8]) -> Result<()> {
        finish(self.writer, Value::Bytes(value.to_vec()))
    }
    fn serialize_none(self) -> Result<()> {
        finish(self.writer, Value::Null)
    }
    fn serialize_some<T: ?Sized + Serialize>(self, value: &T) -> Result<()> {
        finish(self.writer, value.serialize(ValueSerializer)?)
    }
    fn serialize_unit(self) -> Result<()> {
        finish(self.writer, Value::Null)
    }
    fn serialize_unit_struct(self, _: &'static str) -> Result<()> {
        finish(self.writer, Value::Null)
    }
    fn serialize_unit_variant(self, _: &'static str, _: u32, variant: &'static str) -> Result<()> {
        finish(self.writer, Value::String(variant.to_owned()))
    }
    fn serialize_newtype_struct<T: ?Sized + Serialize>(
        self,
        _: &'static str,
        value: &T,
    ) -> Result<()> {
        finish(self.writer, value.serialize(ValueSerializer)?)
    }
    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<()> {
        finish(
            self.writer,
            Value::Map(BTreeMap::from([(
                variant.to_owned(),
                value.serialize(ValueSerializer)?,
            )])),
        )
    }
    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq> {
        Ok(RootSequence {
            writer: self.writer,
            values: Vec::with_capacity(len.unwrap_or(0)),
        })
    }
    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple> {
        self.serialize_seq(Some(len))
    }
    fn serialize_tuple_struct(
        self,
        _: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        self.serialize_seq(Some(len))
    }
    fn serialize_tuple_variant(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        Ok(RootTupleVariant {
            writer: self.writer,
            variant: variant.to_owned(),
            values: Vec::with_capacity(len),
        })
    }
    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap> {
        Ok(RootMap {
            writer: self.writer,
            values: BTreeMap::new(),
            pending_key: None,
            _capacity: len.unwrap_or(0),
        })
    }
    fn serialize_struct(self, _: &'static str, len: usize) -> Result<Self::SerializeStruct> {
        self.serialize_map(Some(len))
    }
    fn serialize_struct_variant(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        Ok(RootStructVariant {
            variant: variant.to_owned(),
            map: RootMap {
                writer: self.writer,
                values: BTreeMap::new(),
                pending_key: None,
                _capacity: len,
            },
        })
    }
}

fn finish<W: Write>(writer: W, value: Value) -> Result<()> {
    Encoder::new(writer).encode(&value)
}

pub struct RootSequence<W> {
    writer: W,
    values: Vec<Value>,
}

impl<W: Write> RootSequence<W> {
    fn push<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<()> {
        self.values.push(value.serialize(ValueSerializer)?);
        Ok(())
    }
    fn finish(self) -> Result<()> {
        finish(self.writer, Value::Array(self.values))
    }
}
impl<W: Write> SerializeSeq for RootSequence<W> {
    type Ok = ();
    type Error = Error;
    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<()> {
        self.push(value)
    }
    fn end(self) -> Result<()> {
        self.finish()
    }
}
impl<W: Write> SerializeTuple for RootSequence<W> {
    type Ok = ();
    type Error = Error;
    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<()> {
        self.push(value)
    }
    fn end(self) -> Result<()> {
        self.finish()
    }
}
impl<W: Write> SerializeTupleStruct for RootSequence<W> {
    type Ok = ();
    type Error = Error;
    fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<()> {
        self.push(value)
    }
    fn end(self) -> Result<()> {
        self.finish()
    }
}

pub struct RootTupleVariant<W> {
    writer: W,
    variant: String,
    values: Vec<Value>,
}
impl<W: Write> SerializeTupleVariant for RootTupleVariant<W> {
    type Ok = ();
    type Error = Error;
    fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<()> {
        self.values.push(value.serialize(ValueSerializer)?);
        Ok(())
    }
    fn end(self) -> Result<()> {
        finish(
            self.writer,
            Value::Map(BTreeMap::from([(self.variant, Value::Array(self.values))])),
        )
    }
}

pub struct RootMap<W> {
    writer: W,
    values: BTreeMap<String, Value>,
    pending_key: Option<String>,
    _capacity: usize,
}
impl<W: Write> SerializeMap for RootMap<W> {
    type Ok = ();
    type Error = Error;
    fn serialize_key<T: ?Sized + Serialize>(&mut self, key: &T) -> Result<()> {
        self.pending_key = Some(key.serialize(KeySerializer)?);
        Ok(())
    }
    fn serialize_value<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<()> {
        let key = self
            .pending_key
            .take()
            .ok_or_else(|| Error::custom("map value has no key"))?;
        self.values.insert(key, value.serialize(ValueSerializer)?);
        Ok(())
    }
    fn end(self) -> Result<()> {
        finish(self.writer, Value::Map(self.values))
    }
}
impl<W: Write> SerializeStruct for RootMap<W> {
    type Ok = ();
    type Error = Error;
    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<()> {
        self.values
            .insert(key.to_owned(), value.serialize(ValueSerializer)?);
        Ok(())
    }
    fn end(self) -> Result<()> {
        finish(self.writer, Value::Map(self.values))
    }
}

pub struct RootStructVariant<W> {
    variant: String,
    map: RootMap<W>,
}
impl<W: Write> SerializeStructVariant for RootStructVariant<W> {
    type Ok = ();
    type Error = Error;
    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<()> {
        self.map
            .values
            .insert(key.to_owned(), value.serialize(ValueSerializer)?);
        Ok(())
    }
    fn end(self) -> Result<()> {
        finish(
            self.map.writer,
            Value::Map(BTreeMap::from([(
                self.variant,
                Value::Map(self.map.values),
            )])),
        )
    }
}

struct Encoder<W> {
    writer: W,
    shared_strings: HashMap<Vec<u8>, usize>,
    class_references: HashMap<Vec<(String, i64)>, usize>,
    c_string_references: HashMap<Vec<u8>, usize>,
    object_count: usize,
}

impl<W: Write> Encoder<W> {
    fn new(writer: W) -> Self {
        Self {
            writer,
            shared_strings: HashMap::new(),
            class_references: HashMap::new(),
            c_string_references: HashMap::new(),
            object_count: 0,
        }
    }

    fn encode(mut self, value: &Value) -> Result<()> {
        self.writer.write_all(&[4, 11])?;
        self.writer.write_all(b"streamtyped")?;
        self.write_integer(1000, false)?;
        self.write_typed_object(value)
    }

    fn write_typed_object(&mut self, value: &Value) -> Result<()> {
        self.write_shared_string(b"@")?;
        self.write_object(value)
    }

    fn write_object(&mut self, value: &Value) -> Result<()> {
        if matches!(value, Value::Null) {
            return self.writer.write_all(&[TAG_NIL]).map_err(Into::into);
        }
        self.writer.write_all(&[TAG_NEW])?;
        self.object_count += 1;
        match value {
            Value::Null => unreachable!(),
            Value::String(value) => {
                self.write_class_chain(&[("NSString", 1), ("NSObject", 0)])?;
                self.write_shared_string(b"+")?;
                self.write_unshared_string(value.as_bytes())?;
            }
            Value::Bytes(value) => {
                self.write_class_chain(&[("NSData", 0), ("NSObject", 0)])?;
                self.write_typed_integer(b"i", collection_len(value.len())?, true)?;
                let encoding = format!("[{}c]", value.len());
                self.write_shared_string(encoding.as_bytes())?;
                self.writer.write_all(value)?;
            }
            Value::Bool(value) => {
                self.write_number(b"B", |encoder| {
                    encoder.writer.write_all(&[u8::from(*value)])?;
                    Ok(())
                })?;
            }
            Value::I64(value) => {
                self.write_number(b"q", |encoder| encoder.write_integer(*value, true))?;
            }
            Value::U64(value) => {
                let value = i64::try_from(*value)
                    .map_err(|_| Error::custom("u64 value exceeds typedstream integer range"))?;
                self.write_number(b"Q", |encoder| encoder.write_integer(value, false))?;
            }
            Value::F64(value) => {
                self.write_number(b"d", |encoder| {
                    encoder.writer.write_all(&[TAG_FLOATING_POINT])?;
                    encoder.writer.write_all(&value.to_le_bytes())?;
                    Ok(())
                })?;
            }
            Value::Array(values) => {
                self.write_class_chain(&[("NSArray", 0), ("NSObject", 0)])?;
                self.write_typed_integer(b"i", collection_len(values.len())?, true)?;
                for value in values {
                    self.write_typed_object(value)?;
                }
            }
            Value::Map(values) => {
                self.write_class_chain(&[("NSDictionary", 0), ("NSObject", 0)])?;
                self.write_typed_integer(b"i", collection_len(values.len())?, true)?;
                for (key, value) in values {
                    self.write_typed_object(&Value::String(key.clone()))?;
                    self.write_typed_object(value)?;
                }
            }
            Value::Archived(_) | Value::Struct(_) | Value::Reference(_) => {
                return Err(Error::custom(
                    "raw archived objects, structs, and references cannot be emitted through generic Serde",
                ));
            }
        }
        self.writer.write_all(&[TAG_END_OF_OBJECT])?;
        Ok(())
    }

    fn write_number<F>(&mut self, encoding: &[u8], payload: F) -> Result<()>
    where
        F: FnOnce(&mut Self) -> Result<()>,
    {
        self.write_class_chain(&[("NSNumber", 0), ("NSValue", 0), ("NSObject", 0)])?;
        self.write_shared_string(b"*")?;
        self.write_c_string(encoding)?;
        self.write_shared_string(encoding)?;
        payload(self)
    }

    fn write_class_chain(&mut self, classes: &[(&str, i64)]) -> Result<()> {
        let key: Vec<(String, i64)> = classes
            .iter()
            .map(|(name, version)| ((*name).to_owned(), *version))
            .collect();
        if let Some(&index) = self.class_references.get(&key) {
            return self.write_integer(reference_index(index)?, true);
        }

        for (position, (name, version)) in classes.iter().enumerate() {
            let index = self.object_count;
            self.object_count += 1;
            let suffix = classes[position..]
                .iter()
                .map(|(name, version)| ((*name).to_owned(), *version))
                .collect();
            self.class_references.insert(suffix, index);
            self.writer.write_all(&[TAG_NEW])?;
            let mut c_name = name.as_bytes().to_vec();
            c_name.push(0);
            self.write_shared_string(&c_name)?;
            self.write_integer(*version, true)?;
        }
        self.writer.write_all(&[TAG_NIL])?;
        Ok(())
    }

    fn write_c_string(&mut self, value: &[u8]) -> Result<()> {
        if let Some(&index) = self.c_string_references.get(value) {
            return self.write_integer(reference_index(index)?, true);
        }
        let index = self.object_count;
        self.object_count += 1;
        self.c_string_references.insert(value.to_vec(), index);
        self.writer.write_all(&[TAG_NEW])?;
        self.write_shared_string(value)
    }

    fn write_typed_integer(&mut self, encoding: &[u8], value: i64, signed: bool) -> Result<()> {
        self.write_shared_string(encoding)?;
        self.write_integer(value, signed)
    }

    fn write_shared_string(&mut self, value: &[u8]) -> Result<()> {
        if let Some(&index) = self.shared_strings.get(value) {
            return self.write_integer(reference_index(index)?, true);
        }
        let index = self.shared_strings.len();
        self.shared_strings.insert(value.to_vec(), index);
        self.writer.write_all(&[TAG_NEW])?;
        self.write_unshared_string(value)
    }

    fn write_unshared_string(&mut self, value: &[u8]) -> Result<()> {
        self.write_integer(collection_len(value.len())?, false)?;
        self.writer.write_all(value)?;
        Ok(())
    }

    fn write_integer(&mut self, value: i64, signed: bool) -> Result<()> {
        if signed && (-128..=127).contains(&value) && !(-128..=-111).contains(&value) {
            let byte = i8::try_from(value).expect("single-byte signed integer range");
            self.writer.write_all(&[byte.cast_unsigned()])?;
        } else if !signed && (0..=255).contains(&value) && !(128..=145).contains(&value) {
            let byte = u8::try_from(value).expect("single-byte unsigned integer range");
            self.writer.write_all(&[byte])?;
        } else if signed {
            if let Ok(narrowed) = i16::try_from(value) {
                self.writer.write_all(&[TAG_INTEGER_2])?;
                self.writer.write_all(&narrowed.to_le_bytes())?;
            } else {
                let narrowed = i32::try_from(value)
                    .map_err(|_| Error::custom("signed integer exceeds typedstream i32 range"))?;
                self.writer.write_all(&[TAG_INTEGER_4])?;
                self.writer.write_all(&narrowed.to_le_bytes())?;
            }
        } else if let Ok(narrowed) = u16::try_from(value) {
            self.writer.write_all(&[TAG_INTEGER_2])?;
            self.writer.write_all(&narrowed.to_le_bytes())?;
        } else {
            let narrowed = u32::try_from(value)
                .map_err(|_| Error::custom("unsigned integer exceeds typedstream u32 range"))?;
            self.writer.write_all(&[TAG_INTEGER_4])?;
            self.writer.write_all(&narrowed.to_le_bytes())?;
        }
        Ok(())
    }
}

struct ValueSerializer;

impl ser::Serializer for ValueSerializer {
    type Ok = Value;
    type Error = Error;
    type SerializeSeq = SequenceSerializer;
    type SerializeTuple = SequenceSerializer;
    type SerializeTupleStruct = SequenceSerializer;
    type SerializeTupleVariant = TupleVariantSerializer;
    type SerializeMap = MapSerializer;
    type SerializeStruct = MapSerializer;
    type SerializeStructVariant = StructVariantSerializer;

    fn serialize_bool(self, value: bool) -> Result<Value> {
        Ok(Value::Bool(value))
    }
    fn serialize_i8(self, value: i8) -> Result<Value> {
        Ok(Value::I64(i64::from(value)))
    }
    fn serialize_i16(self, value: i16) -> Result<Value> {
        Ok(Value::I64(i64::from(value)))
    }
    fn serialize_i32(self, value: i32) -> Result<Value> {
        Ok(Value::I64(i64::from(value)))
    }
    fn serialize_i64(self, value: i64) -> Result<Value> {
        Ok(Value::I64(value))
    }
    fn serialize_u8(self, value: u8) -> Result<Value> {
        Ok(Value::U64(u64::from(value)))
    }
    fn serialize_u16(self, value: u16) -> Result<Value> {
        Ok(Value::U64(u64::from(value)))
    }
    fn serialize_u32(self, value: u32) -> Result<Value> {
        Ok(Value::U64(u64::from(value)))
    }
    fn serialize_u64(self, value: u64) -> Result<Value> {
        Ok(Value::U64(value))
    }
    fn serialize_f32(self, value: f32) -> Result<Value> {
        Ok(Value::F64(f64::from(value)))
    }
    fn serialize_f64(self, value: f64) -> Result<Value> {
        Ok(Value::F64(value))
    }
    fn serialize_char(self, value: char) -> Result<Value> {
        Ok(Value::String(value.to_string()))
    }
    fn serialize_str(self, value: &str) -> Result<Value> {
        Ok(Value::String(value.to_owned()))
    }
    fn serialize_bytes(self, value: &[u8]) -> Result<Value> {
        Ok(Value::Bytes(value.to_vec()))
    }
    fn serialize_none(self) -> Result<Value> {
        Ok(Value::Null)
    }
    fn serialize_some<T: ?Sized + Serialize>(self, value: &T) -> Result<Value> {
        value.serialize(self)
    }
    fn serialize_unit(self) -> Result<Value> {
        Ok(Value::Null)
    }
    fn serialize_unit_struct(self, _name: &'static str) -> Result<Value> {
        Ok(Value::Null)
    }
    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _index: u32,
        variant: &'static str,
    ) -> Result<Value> {
        Ok(Value::String(variant.to_owned()))
    }
    fn serialize_newtype_struct<T: ?Sized + Serialize>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Value> {
        value.serialize(self)
    }
    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        _name: &'static str,
        _index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Value> {
        Ok(Value::Map(BTreeMap::from([(
            variant.to_owned(),
            value.serialize(ValueSerializer)?,
        )])))
    }
    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq> {
        Ok(SequenceSerializer::new(len))
    }
    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple> {
        Ok(SequenceSerializer::new(Some(len)))
    }
    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        Ok(SequenceSerializer::new(Some(len)))
    }
    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        Ok(TupleVariantSerializer {
            variant: variant.to_owned(),
            values: Vec::with_capacity(len),
        })
    }
    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap> {
        Ok(MapSerializer::new(len))
    }
    fn serialize_struct(self, _name: &'static str, len: usize) -> Result<Self::SerializeStruct> {
        Ok(MapSerializer::new(Some(len)))
    }
    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        Ok(StructVariantSerializer {
            variant: variant.to_owned(),
            map: MapSerializer::new(Some(len)),
        })
    }
}

struct SequenceSerializer {
    values: Vec<Value>,
}

impl SequenceSerializer {
    fn new(len: Option<usize>) -> Self {
        Self {
            values: Vec::with_capacity(len.unwrap_or(0)),
        }
    }
    fn push<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<()> {
        self.values.push(value.serialize(ValueSerializer)?);
        Ok(())
    }
}

impl SerializeSeq for SequenceSerializer {
    type Ok = Value;
    type Error = Error;
    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<()> {
        self.push(value)
    }
    fn end(self) -> Result<Value> {
        Ok(Value::Array(self.values))
    }
}
impl SerializeTuple for SequenceSerializer {
    type Ok = Value;
    type Error = Error;
    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<()> {
        self.push(value)
    }
    fn end(self) -> Result<Value> {
        Ok(Value::Array(self.values))
    }
}
impl SerializeTupleStruct for SequenceSerializer {
    type Ok = Value;
    type Error = Error;
    fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<()> {
        self.push(value)
    }
    fn end(self) -> Result<Value> {
        Ok(Value::Array(self.values))
    }
}

struct TupleVariantSerializer {
    variant: String,
    values: Vec<Value>,
}
impl SerializeTupleVariant for TupleVariantSerializer {
    type Ok = Value;
    type Error = Error;
    fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<()> {
        self.values.push(value.serialize(ValueSerializer)?);
        Ok(())
    }
    fn end(self) -> Result<Value> {
        Ok(Value::Map(BTreeMap::from([(
            self.variant,
            Value::Array(self.values),
        )])))
    }
}

struct MapSerializer {
    values: BTreeMap<String, Value>,
    pending_key: Option<String>,
}
impl MapSerializer {
    fn new(_len: Option<usize>) -> Self {
        Self {
            values: BTreeMap::new(),
            pending_key: None,
        }
    }
}
impl SerializeMap for MapSerializer {
    type Ok = Value;
    type Error = Error;
    fn serialize_key<T: ?Sized + Serialize>(&mut self, key: &T) -> Result<()> {
        self.pending_key = Some(key.serialize(KeySerializer)?);
        Ok(())
    }
    fn serialize_value<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<()> {
        let key = self
            .pending_key
            .take()
            .ok_or_else(|| Error::custom("map value has no key"))?;
        self.values.insert(key, value.serialize(ValueSerializer)?);
        Ok(())
    }
    fn end(self) -> Result<Value> {
        Ok(Value::Map(self.values))
    }
}
impl SerializeStruct for MapSerializer {
    type Ok = Value;
    type Error = Error;
    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<()> {
        self.values
            .insert(key.to_owned(), value.serialize(ValueSerializer)?);
        Ok(())
    }
    fn end(self) -> Result<Value> {
        Ok(Value::Map(self.values))
    }
}

struct StructVariantSerializer {
    variant: String,
    map: MapSerializer,
}
impl SerializeStructVariant for StructVariantSerializer {
    type Ok = Value;
    type Error = Error;
    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<()> {
        SerializeStruct::serialize_field(&mut self.map, key, value)
    }
    fn end(self) -> Result<Value> {
        Ok(Value::Map(BTreeMap::from([(
            self.variant,
            Value::Map(self.map.values),
        )])))
    }
}

struct KeySerializer;
impl ser::Serializer for KeySerializer {
    type Ok = String;
    type Error = Error;
    type SerializeSeq = ser::Impossible<String, Error>;
    type SerializeTuple = ser::Impossible<String, Error>;
    type SerializeTupleStruct = ser::Impossible<String, Error>;
    type SerializeTupleVariant = ser::Impossible<String, Error>;
    type SerializeMap = ser::Impossible<String, Error>;
    type SerializeStruct = ser::Impossible<String, Error>;
    type SerializeStructVariant = ser::Impossible<String, Error>;
    fn serialize_str(self, value: &str) -> Result<String> {
        Ok(value.to_owned())
    }
    fn serialize_char(self, value: char) -> Result<String> {
        Ok(value.to_string())
    }
    fn serialize_bool(self, value: bool) -> Result<String> {
        Ok(value.to_string())
    }
    fn serialize_i8(self, value: i8) -> Result<String> {
        Ok(value.to_string())
    }
    fn serialize_i16(self, value: i16) -> Result<String> {
        Ok(value.to_string())
    }
    fn serialize_i32(self, value: i32) -> Result<String> {
        Ok(value.to_string())
    }
    fn serialize_i64(self, value: i64) -> Result<String> {
        Ok(value.to_string())
    }
    fn serialize_u8(self, value: u8) -> Result<String> {
        Ok(value.to_string())
    }
    fn serialize_u16(self, value: u16) -> Result<String> {
        Ok(value.to_string())
    }
    fn serialize_u32(self, value: u32) -> Result<String> {
        Ok(value.to_string())
    }
    fn serialize_u64(self, value: u64) -> Result<String> {
        Ok(value.to_string())
    }
    fn serialize_f32(self, value: f32) -> Result<String> {
        Ok(value.to_string())
    }
    fn serialize_f64(self, value: f64) -> Result<String> {
        Ok(value.to_string())
    }
    fn serialize_unit_variant(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
    ) -> Result<String> {
        Ok(variant.to_owned())
    }
    fn serialize_newtype_struct<T: ?Sized + Serialize>(
        self,
        _: &'static str,
        value: &T,
    ) -> Result<String> {
        value.serialize(self)
    }
    fn serialize_bytes(self, _: &[u8]) -> Result<String> {
        Err(Error::custom("map key must be scalar"))
    }
    fn serialize_none(self) -> Result<String> {
        Err(Error::custom("map key cannot be none"))
    }
    fn serialize_some<T: ?Sized + Serialize>(self, value: &T) -> Result<String> {
        value.serialize(self)
    }
    fn serialize_unit(self) -> Result<String> {
        Err(Error::custom("map key cannot be unit"))
    }
    fn serialize_unit_struct(self, _: &'static str) -> Result<String> {
        Err(Error::custom("map key cannot be unit"))
    }
    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: &T,
    ) -> Result<String> {
        Err(Error::custom("map key must be scalar"))
    }
    fn serialize_seq(self, _: Option<usize>) -> Result<Self::SerializeSeq> {
        Err(Error::custom("map key must be scalar"))
    }
    fn serialize_tuple(self, _: usize) -> Result<Self::SerializeTuple> {
        Err(Error::custom("map key must be scalar"))
    }
    fn serialize_tuple_struct(
        self,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        Err(Error::custom("map key must be scalar"))
    }
    fn serialize_tuple_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        Err(Error::custom("map key must be scalar"))
    }
    fn serialize_map(self, _: Option<usize>) -> Result<Self::SerializeMap> {
        Err(Error::custom("map key must be scalar"))
    }
    fn serialize_struct(self, _: &'static str, _: usize) -> Result<Self::SerializeStruct> {
        Err(Error::custom("map key must be scalar"))
    }
    fn serialize_struct_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeStructVariant> {
        Err(Error::custom("map key must be scalar"))
    }
}
