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

use std::io::{Read, Write};

pub use de::Deserializer;
pub use error::{Error, Result};
pub use ser::Serializer;
use serde::{Serialize, de::DeserializeOwned};
pub use value::{ArchivedObject, Class, Reference, ReferenceKind, StructValue, TypedValues, Value};

/// Maximum number of bytes [`from_reader`] and [`value_from_reader`] will read.
pub const MAX_INPUT_BYTES: usize = 64 * 1024 * 1024;

/// Parse a typedstream into its dynamically typed [`Value`] representation.
///
/// This is the faithful view of the archive. Going through [`from_slice`]
/// instead would round-trip it through Serde and flatten archived objects into
/// plain maps.
pub fn value_from_slice(bytes: &[u8]) -> Result<Value> {
    parse::parse(bytes)
}

/// Parse a typedstream [`Value`] from a reader, reading at most
/// [`MAX_INPUT_BYTES`].
pub fn value_from_reader<R: Read>(reader: R) -> Result<Value> {
    value_from_slice(&read_bounded(reader, MAX_INPUT_BYTES)?)
}

/// Deserialize a value from a typedstream byte slice.
pub fn from_slice<T>(bytes: &[u8]) -> Result<T>
where
    T: DeserializeOwned,
{
    de::from_value(parse::parse(bytes)?)
}

/// Deserialize a value from a reader containing one typedstream.
///
/// The reader is capped at [`MAX_INPUT_BYTES`]; use [`from_reader_with_limit`]
/// to choose a different ceiling.
pub fn from_reader<R, T>(reader: R) -> Result<T>
where
    R: Read,
    T: DeserializeOwned,
{
    from_reader_with_limit(reader, MAX_INPUT_BYTES)
}

/// Deserialize from a reader, reading at most `limit` bytes.
pub fn from_reader_with_limit<R, T>(reader: R, limit: usize) -> Result<T>
where
    R: Read,
    T: DeserializeOwned,
{
    from_slice(&read_bounded(reader, limit)?)
}

/// Read at most `limit` bytes, failing rather than growing without bound.
fn read_bounded<R: Read>(reader: R, limit: usize) -> Result<Vec<u8>> {
    let ceiling = u64::try_from(limit).unwrap_or(u64::MAX).saturating_add(1);
    let mut bytes = Vec::new();
    reader.take(ceiling).read_to_end(&mut bytes)?;
    if bytes.len() > limit {
        return Err(Error::limit(bytes.len(), "input size", bytes.len(), limit));
    }
    Ok(bytes)
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
