//! Serde [`Serializer`] and [`Deserializer`] for Apple/NeXT typedstreams.
//!
//! This crate reads and writes the binary format used by `NSArchiver`. Generic
//! Serde values are represented using Foundation objects such as `NSString`,
//! `NSNumber`, `NSArray`, `NSDictionary`, and `NSData`.

mod de;
mod encoding;
mod error;
mod parse;
mod ser;
mod value;

use std::{
    any::{Any, TypeId},
    io::{Read, Write},
};

pub use de::Deserializer;
pub use error::{Error, Result};
pub use ser::Serializer;
use serde::{Serialize, de::DeserializeOwned};
pub use value::{ArchivedObject, Class, Reference, ReferenceKind, StructValue, TypedValues, Value};

/// Deserialize a value from a typedstream byte slice.
pub fn from_slice<T>(bytes: &[u8]) -> Result<T>
where
    T: DeserializeOwned + 'static,
{
    let value = parse::parse(bytes)?;
    if TypeId::of::<T>() == TypeId::of::<Value>() {
        let value: Box<dyn Any> = Box::new(value);
        return Ok(*value
            .downcast::<T>()
            .expect("type id checked before downcast"));
    }
    de::from_value(value)
}

/// Deserialize a value from a reader containing one typedstream.
pub fn from_reader<R, T>(mut reader: R) -> Result<T>
where
    R: Read,
    T: DeserializeOwned + 'static,
{
    let mut bytes = Vec::new();
    reader.read_to_end(&mut bytes)?;
    from_slice(&bytes)
}

/// Serialize a value into a typedstream blob.
pub fn to_vec<T>(value: &T) -> Result<Vec<u8>>
where
    T: ?Sized + Serialize,
{
    let mut bytes = Vec::new();
    to_writer(&mut bytes, value)?;
    Ok(bytes)
}

/// Serialize a value to a typedstream writer.
pub fn to_writer<W, T>(writer: W, value: &T) -> Result<()>
where
    W: Write,
    T: ?Sized + Serialize,
{
    ser::to_writer(writer, value)
}
