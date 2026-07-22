use std::collections::BTreeMap;

use serde::{
    Deserialize, Serialize, Serializer,
    ser::{SerializeMap, SerializeSeq},
};

/// A dynamically typed value represented by a typedstream archive.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum Value {
    Null,
    Bool(bool),
    I64(i64),
    U64(u64),
    F64(f64),
    String(String),
    Bytes(Vec<u8>),
    Array(Vec<Value>),
    Map(BTreeMap<String, Value>),
    Archived(ArchivedObject),
    Struct(StructValue),
    Reference(Reference),
}

impl Serialize for Value {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        match self {
            Self::Null => serializer.serialize_unit(),
            Self::Bool(value) => serializer.serialize_bool(*value),
            Self::I64(value) => serializer.serialize_i64(*value),
            Self::U64(value) => serializer.serialize_u64(*value),
            Self::F64(value) => serializer.serialize_f64(*value),
            Self::String(value) => serializer.serialize_str(value),
            Self::Bytes(value) => serializer.serialize_bytes(value),
            Self::Array(values) => {
                let mut sequence = serializer.serialize_seq(Some(values.len()))?;
                for value in values {
                    sequence.serialize_element(value)?;
                }
                sequence.end()
            }
            Self::Map(values) => {
                let mut map = serializer.serialize_map(Some(values.len()))?;
                for (key, value) in values {
                    map.serialize_entry(key, value)?;
                }
                map.end()
            }
            Self::Archived(value) => value.serialize(serializer),
            Self::Struct(value) => value.serialize(serializer),
            Self::Reference(value) => value.serialize(serializer),
        }
    }
}

/// An Objective-C object whose class-specific fields are not known.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ArchivedObject {
    pub classes: Vec<Class>,
    pub fields: Vec<TypedValues>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Class {
    pub name: String,
    pub version: i64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TypedValues {
    pub encoding: String,
    pub values: Vec<Value>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StructValue {
    pub name: Option<String>,
    pub fields: Vec<TypedValues>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reference {
    pub kind: ReferenceKind,
    pub index: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReferenceKind {
    CString,
    Class,
    Object,
}

impl Value {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[Value]> {
        match self {
            Self::Array(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_map(&self) -> Option<&BTreeMap<String, Value>> {
        match self {
            Self::Map(value) => Some(value),
            _ => None,
        }
    }
}
