use std::{collections::BTreeMap, io::Read};

use serde::de::{
    self, DeserializeOwned, EnumAccess, IntoDeserializer, MapAccess, SeqAccess, VariantAccess,
    Visitor,
};

use crate::{
    error::{Error, Result},
    value::{ArchivedObject, Class, TypedValues, Value},
};

pub struct Deserializer {
    value: Value,
}

impl Deserializer {
    pub fn from_slice(bytes: &[u8]) -> Result<Self> {
        Ok(Self::from_value(crate::parse::parse(bytes)?))
    }

    pub fn from_reader<R: Read>(mut reader: R) -> Result<Self> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes)?;
        Self::from_slice(&bytes)
    }

    pub fn from_value(value: Value) -> Self {
        Self { value }
    }
}

pub(crate) fn from_value<T>(value: Value) -> Result<T>
where
    T: DeserializeOwned,
{
    T::deserialize(Deserializer::from_value(value))
}

impl<'de> de::Deserializer<'de> for Deserializer {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::Null => visitor.visit_unit(),
            Value::Bool(value) => visitor.visit_bool(value),
            Value::I64(value) => visitor.visit_i64(value),
            Value::U64(value) => visitor.visit_u64(value),
            Value::F64(value) => visitor.visit_f64(value),
            Value::String(value) => visitor.visit_string(value),
            Value::Bytes(value) => visitor.visit_byte_buf(value),
            Value::Array(values) => visitor.visit_seq(SeqAccessIterator {
                values: values.into_iter(),
            }),
            Value::Map(values) => visitor.visit_map(MapAccessIterator {
                entries: values.into_iter(),
                next_value: None,
            }),
            Value::Archived(value) => {
                let object = archived_to_map(value);
                visitor.visit_map(MapAccessIterator {
                    entries: object.into_iter(),
                    next_value: None,
                })
            }
            Value::Struct(value) => visitor.visit_seq(SeqAccessIterator {
                values: value
                    .fields
                    .into_iter()
                    .flat_map(|field| field.values)
                    .collect::<Vec<_>>()
                    .into_iter(),
            }),
            Value::Reference(value) => visitor.visit_u64(value.index as u64),
        }
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::Null => visitor.visit_none(),
            value => visitor.visit_some(Deserializer::from_value(value)),
        }
    }

    fn deserialize_enum<V>(
        self,
        _name: &str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::String(value) => visitor.visit_enum(value.into_deserializer()),
            Value::Map(mut values) if values.len() == 1 => {
                let (variant, value) = values
                    .pop_first()
                    .ok_or_else(|| Error::custom("enum map variant missing after length check"))?;
                visitor.visit_enum(ValueEnumAccess {
                    variant,
                    value: Some(value),
                })
            }
            value => Err(unexpected(&value)),
        }
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::Bytes(value) => visitor.visit_bytes(&value),
            Value::String(value) => visitor.visit_bytes(value.as_bytes()),
            value => Err(unexpected(&value)),
        }
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::Bytes(value) => visitor.visit_byte_buf(value),
            Value::String(value) => visitor.visit_byte_buf(value.into_bytes()),
            value => Err(unexpected(&value)),
        }
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string unit
        unit_struct newtype_struct seq tuple tuple_struct map struct identifier
        ignored_any
    }
}

struct SeqAccessIterator {
    values: std::vec::IntoIter<Value>,
}

impl<'de> SeqAccess<'de> for SeqAccessIterator {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: de::DeserializeSeed<'de>,
    {
        match self.values.next() {
            Some(value) => seed.deserialize(Deserializer::from_value(value)).map(Some),
            None => Ok(None),
        }
    }
}

struct MapAccessIterator {
    entries: std::collections::btree_map::IntoIter<String, Value>,
    next_value: Option<Value>,
}

impl<'de> MapAccess<'de> for MapAccessIterator {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: de::DeserializeSeed<'de>,
    {
        match self.entries.next() {
            Some((key, value)) => {
                self.next_value = Some(value);
                seed.deserialize(key.into_deserializer()).map(Some)
            }
            None => Ok(None),
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: de::DeserializeSeed<'de>,
    {
        let value = self
            .next_value
            .take()
            .ok_or_else(|| Error::custom("missing map value"))?;
        seed.deserialize(Deserializer::from_value(value))
    }
}

fn unexpected(value: &Value) -> Error {
    Error::custom(format!("unexpected typedstream value: {value:?}"))
}

struct ValueEnumAccess {
    variant: String,
    value: Option<Value>,
}

impl<'de> EnumAccess<'de> for ValueEnumAccess {
    type Error = Error;
    type Variant = ValueVariantAccess;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant)>
    where
        V: de::DeserializeSeed<'de>,
    {
        let variant = seed.deserialize(serde::de::value::StringDeserializer::<Error>::new(
            self.variant,
        ))?;
        Ok((variant, ValueVariantAccess { value: self.value }))
    }
}

struct ValueVariantAccess {
    value: Option<Value>,
}

impl<'de> VariantAccess<'de> for ValueVariantAccess {
    type Error = Error;

    fn unit_variant(self) -> Result<()> {
        match self.value {
            None | Some(Value::Null) => Ok(()),
            Some(value) => Err(unexpected(&value)),
        }
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
    where
        T: de::DeserializeSeed<'de>,
    {
        seed.deserialize(Deserializer::from_value(self.value.unwrap_or(Value::Null)))
    }

    fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        de::Deserializer::deserialize_seq(
            Deserializer::from_value(self.value.unwrap_or(Value::Array(Vec::new()))),
            visitor,
        )
    }

    fn struct_variant<V>(self, _fields: &'static [&'static str], visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        de::Deserializer::deserialize_map(
            Deserializer::from_value(self.value.unwrap_or(Value::Map(BTreeMap::new()))),
            visitor,
        )
    }
}

fn archived_to_map(value: ArchivedObject) -> BTreeMap<String, Value> {
    BTreeMap::from([
        (
            "$classes".to_owned(),
            Value::Array(value.classes.into_iter().map(class_to_value).collect()),
        ),
        (
            "$fields".to_owned(),
            Value::Array(value.fields.into_iter().map(field_to_value).collect()),
        ),
    ])
}

fn class_to_value(value: Class) -> Value {
    Value::Map(BTreeMap::from([
        ("name".to_owned(), Value::String(value.name)),
        ("version".to_owned(), Value::I64(value.version)),
    ]))
}

fn field_to_value(value: TypedValues) -> Value {
    Value::Map(BTreeMap::from([
        ("encoding".to_owned(), Value::String(value.encoding)),
        ("values".to_owned(), Value::Array(value.values)),
    ]))
}
