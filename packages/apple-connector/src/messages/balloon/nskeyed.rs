//! Decode `NSKeyedArchiver` property lists used by Messages app balloons.
//!
//! Archives store an object table under `$objects` and point `$top.root` at the
//! root object via UID references. Binary plists use native UID values; XML
//! plists encode the same refs as `{ CF$UID: <integer> }` dictionaries. This
//! module resolves both into a nested [`plist::Value`] graph that balloon
//! parsers can read with ordinary dictionary lookups.

use plist::{Dictionary, Value};

const MAX_DEPTH: usize = 256;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NsKeyedError {
    NotADictionary,
    MissingObjects,
    MissingRoot,
    BadUid,
    RecursionLimit,
    KeyValueLengthMismatch,
}

/// Decode a binary or XML plist and expand `NSKeyedArchiver` UID references.
pub fn decode_ns_keyed_archiver(bytes: &[u8]) -> Result<Value, NsKeyedError> {
    let plist = Value::from_reader(std::io::Cursor::new(bytes))
        .map_err(|_| NsKeyedError::NotADictionary)?;
    expand_ns_keyed_archiver(&plist)
}

pub fn expand_ns_keyed_archiver(plist: &Value) -> Result<Value, NsKeyedError> {
    let root = plist.as_dictionary().ok_or(NsKeyedError::NotADictionary)?;
    let objects = root
        .get("$objects")
        .and_then(Value::as_array)
        .ok_or(NsKeyedError::MissingObjects)?;
    let top = root
        .get("$top")
        .and_then(Value::as_dictionary)
        .ok_or(NsKeyedError::MissingRoot)?;
    let root_uid = top.get("root").ok_or(NsKeyedError::MissingRoot)?;
    resolve(objects, root_uid, 0)
}

fn resolve(objects: &[Value], value: &Value, depth: usize) -> Result<Value, NsKeyedError> {
    if depth >= MAX_DEPTH {
        return Err(NsKeyedError::RecursionLimit);
    }

    if let Some(index) = uid_ref(value)? {
        let object = objects.get(index).ok_or(NsKeyedError::BadUid)?;
        return resolve_object(objects, object, depth + 1);
    }

    resolve_object(objects, value, depth)
}

fn resolve_object(objects: &[Value], object: &Value, depth: usize) -> Result<Value, NsKeyedError> {
    if depth >= MAX_DEPTH {
        return Err(NsKeyedError::RecursionLimit);
    }

    if let Some(index) = uid_ref(object)? {
        let nested = objects.get(index).ok_or(NsKeyedError::BadUid)?;
        return resolve_object(objects, nested, depth + 1);
    }

    match object {
        Value::Array(items) => {
            let mut out = Vec::with_capacity(items.len());
            for item in items {
                out.push(resolve(objects, item, depth + 1)?);
            }
            Ok(Value::Array(out))
        }
        Value::Dictionary(dict) => resolve_dictionary(objects, dict, depth),
        other => Ok(other.clone()),
    }
}

fn resolve_dictionary(
    objects: &[Value],
    dict: &Dictionary,
    depth: usize,
) -> Result<Value, NsKeyedError> {
    // NSURL / path-style objects store the string in NS.relative.
    if let Some(relative) = dict.get("NS.relative") {
        return resolve(objects, relative, depth + 1);
    }

    // NSDictionary / NSMutableDictionary
    if dict.contains_key("NS.keys") && dict.contains_key("NS.objects") {
        let keys = dict
            .get("NS.keys")
            .and_then(Value::as_array)
            .ok_or(NsKeyedError::NotADictionary)?;
        let values = dict
            .get("NS.objects")
            .and_then(Value::as_array)
            .ok_or(NsKeyedError::NotADictionary)?;
        if keys.len() != values.len() {
            return Err(NsKeyedError::KeyValueLengthMismatch);
        }

        let mut out = Dictionary::new();
        for (key_ref, value_ref) in keys.iter().zip(values.iter()) {
            let key = resolve(objects, key_ref, depth + 1)?;
            let value = resolve(objects, value_ref, depth + 1)?;
            out.insert(value_as_key(&key), value);
        }
        return Ok(Value::Dictionary(out));
    }

    let mut out = Dictionary::new();
    for (key, value) in dict {
        if key == "$class" {
            continue;
        }
        out.insert(key.clone(), resolve(objects, value, depth + 1)?);
    }
    Ok(Value::Dictionary(out))
}

/// Binary plists use [`Value::Uid`]; XML plists encode the same pointer as
/// `{ "CF$UID": <integer> }`.
fn uid_ref(value: &Value) -> Result<Option<usize>, NsKeyedError> {
    if let Some(uid) = value.as_uid() {
        return usize::try_from(uid.get())
            .map(Some)
            .map_err(|_| NsKeyedError::BadUid);
    }

    let Some(dict) = value.as_dictionary() else {
        return Ok(None);
    };
    if dict.len() != 1 {
        return Ok(None);
    }
    let Some(index) = dict.get("CF$UID").and_then(Value::as_unsigned_integer) else {
        return Ok(None);
    };
    usize::try_from(index)
        .map(Some)
        .map_err(|_| NsKeyedError::BadUid)
}

fn value_as_key(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Integer(integer) => integer.to_string(),
        Value::Boolean(flag) => flag.to_string(),
        Value::Real(number) => number.to_string(),
        _ => "key".to_owned(),
    }
}

pub fn dict_string(dict: &Dictionary, key: &str) -> Option<String> {
    match dict.get(key)? {
        Value::String(text) => Some(text.clone()),
        Value::Dictionary(nested) => nested
            .get("URL")
            .and_then(Value::as_string)
            .map(str::to_owned)
            .or_else(|| {
                nested
                    .values()
                    .find_map(Value::as_string)
                    .map(str::to_owned)
            }),
        _ => None,
    }
}

pub fn as_dictionary(value: &Value) -> Option<&Dictionary> {
    value.as_dictionary()
}
