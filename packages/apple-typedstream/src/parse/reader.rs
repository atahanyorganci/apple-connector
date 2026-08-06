use std::collections::BTreeMap;

use crate::{
    encoding::{self, Encoding},
    error::{Error, Result},
    value::{ArchivedObject, Class, Reference, ReferenceKind, StructValue, TypedValues, Value},
};

const TAG_INTEGER_2: i8 = -127;
const TAG_INTEGER_4: i8 = -126;
const TAG_FLOATING_POINT: i8 = -125;
const TAG_NEW: i8 = -124;
const TAG_NIL: i8 = -123;
const TAG_END_OF_OBJECT: i8 = -122;
const FIRST_TAG: i8 = -128;
const LAST_TAG: i8 = -111;
const FIRST_REFERENCE: i64 = -110;
const MAX_DEPTH: usize = 128;
const MAX_BLOB_LENGTH: usize = 64 * 1024 * 1024;

#[derive(Clone)]
enum CacheEntry {
    CString(Value),
    Class(Vec<Class>),
    Object(Value),
}

#[derive(Clone, Copy)]
enum ByteOrder {
    Little,
    Big,
}

pub(crate) fn parse(bytes: &[u8]) -> Result<Value> {
    Reader::new(bytes)?.read_root()
}

struct Reader<'a> {
    bytes: &'a [u8],
    offset: usize,
    byte_order: ByteOrder,
    shared_strings: Vec<Vec<u8>>,
    objects: Vec<CacheEntry>,
}

impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8]) -> Result<Self> {
        let mut reader = Self {
            bytes,
            offset: 0,
            byte_order: ByteOrder::Little,
            shared_strings: Vec::new(),
            objects: Vec::new(),
        };
        reader.read_header()?;
        Ok(reader)
    }

    fn read_header(&mut self) -> Result<()> {
        let version = self.read_u8()?;
        if version != 4 {
            return Err(Error::InvalidHeader {
                offset: 0,
                message: format!("unsupported streamer version {version}"),
            });
        }

        let signature_length = usize::from(self.read_u8()?);
        if signature_length != 11 {
            return Err(Error::InvalidHeader {
                offset: 1,
                message: format!("signature length is {signature_length}, expected 11"),
            });
        }
        let signature = self.read_exact(signature_length)?;
        self.byte_order = match signature {
            b"streamtyped" => ByteOrder::Little,
            b"typedstream" => ByteOrder::Big,
            _ => {
                return Err(Error::InvalidHeader {
                    offset: 2,
                    message: "invalid byte-order signature".to_owned(),
                });
            }
        };

        let _system_version = self.read_integer(false)?;
        Ok(())
    }

    fn read_root(&mut self) -> Result<Value> {
        let mut roots = Vec::new();
        while self.offset < self.bytes.len() {
            let group = self.read_typed_values(0)?;
            roots.extend(group.values);
        }

        match roots.len() {
            0 => Err(Error::syntax(self.offset, "stream contains no values")),
            1 => Ok(roots.pop().expect("length checked")),
            _ => Ok(Value::Array(roots)),
        }
    }

    fn read_typed_values(&mut self, depth: usize) -> Result<TypedValues> {
        self.check_depth(depth)?;
        let encoding_bytes = self
            .read_shared_string()?
            .ok_or_else(|| Error::syntax(self.offset, "nil type encoding"))?;
        let encodings = encoding::parse_all(&encoding_bytes)?;
        let mut values = Vec::with_capacity(encodings.len());
        for encoding in &encodings {
            values.push(self.read_value(encoding, depth + 1)?);
        }
        Ok(TypedValues {
            encoding: String::from_utf8_lossy(&encoding_bytes).into_owned(),
            values,
        })
    }

    fn read_value(&mut self, encoding: &Encoding, depth: usize) -> Result<Value> {
        self.check_depth(depth)?;
        match encoding {
            Encoding::Bool => match self.read_u8()? {
                0 => Ok(Value::Bool(false)),
                1 => Ok(Value::Bool(true)),
                value => Err(Error::syntax(
                    self.offset - 1,
                    format!("invalid boolean byte {value}"),
                )),
            },
            Encoding::I8 => Ok(Value::I64(i64::from(self.read_i8()?))),
            Encoding::U8 => Ok(Value::U64(u64::from(self.read_u8()?))),
            Encoding::I16 | Encoding::I32 | Encoding::I64 => {
                Ok(Value::I64(self.read_integer(true)?))
            }
            Encoding::U16 | Encoding::U32 | Encoding::U64 => {
                Ok(Value::U64(self.read_integer(false)? as u64))
            }
            Encoding::F32 => Ok(Value::F64(f64::from(self.read_float()?))),
            Encoding::F64 => Ok(Value::F64(self.read_double()?)),
            Encoding::CString => self.read_c_string(),
            Encoding::Atom | Encoding::Selector => Ok(match self.read_shared_string()? {
                Some(value) => bytes_to_value(value),
                None => Value::Null,
            }),
            Encoding::Bytes => Ok(match self.read_unshared_string()? {
                Some(value) => Value::Bytes(value),
                None => Value::Null,
            }),
            Encoding::Class => {
                let classes = self.read_class_chain()?;
                Ok(Value::Archived(ArchivedObject {
                    classes,
                    fields: Vec::new(),
                }))
            }
            Encoding::Object => self.read_object(depth + 1),
            Encoding::Ignored => Ok(Value::Null),
            Encoding::Array(length, element) => {
                if matches!(element.as_ref(), Encoding::I8 | Encoding::U8) {
                    return Ok(Value::Bytes(self.read_exact(*length)?.to_vec()));
                }
                let mut values = Vec::with_capacity(*length);
                for _ in 0..*length {
                    values.push(self.read_value(element, depth + 1)?);
                }
                Ok(Value::Array(values))
            }
            Encoding::Struct(name, fields) => {
                let mut values = Vec::with_capacity(fields.len());
                for field in fields {
                    values.push(TypedValues {
                        encoding: encoding_name(field),
                        values: vec![self.read_value(field, depth + 1)?],
                    });
                }
                Ok(Value::Struct(StructValue {
                    name: name.clone(),
                    fields: values,
                }))
            }
        }
    }

    fn read_object(&mut self, depth: usize) -> Result<Value> {
        let head = self.read_i8()?;
        if head == TAG_NIL {
            return Ok(Value::Null);
        }
        if head != TAG_NEW {
            return self.read_reference(ReferenceKind::Object, head);
        }

        let object_index = self.objects.len();
        self.objects
            .push(CacheEntry::Object(Value::Reference(Reference {
                kind: ReferenceKind::Object,
                index: object_index,
            })));

        let classes = self.read_class_chain()?;
        let mut fields = Vec::new();
        while self.peek_i8()? != TAG_END_OF_OBJECT {
            fields.push(self.read_typed_values(depth + 1)?);
        }
        self.offset += 1;

        let value = normalize_object(ArchivedObject { classes, fields });
        self.objects[object_index] = CacheEntry::Object(value.clone());
        Ok(value)
    }

    fn read_class_chain(&mut self) -> Result<Vec<Class>> {
        let mut literals = Vec::new();
        let mut head = self.read_i8()?;
        while head == TAG_NEW {
            let name = self
                .read_shared_string()?
                .ok_or_else(|| Error::syntax(self.offset, "nil class name"))?;
            let name = String::from_utf8(name)
                .map_err(|_| Error::syntax(self.offset, "class name is not UTF-8"))?;
            let name = name.trim_end_matches('\0').to_owned();
            let version = self.read_integer(true)?;
            literals.push(Class { name, version });
            head = self.read_i8()?;
        }

        let inherited = if head == TAG_NIL {
            Vec::new()
        } else {
            let index = self.decode_reference(head)?;
            match self.objects.get(index) {
                Some(CacheEntry::Class(classes)) => classes.clone(),
                Some(_) => {
                    return Err(Error::syntax(
                        self.offset - 1,
                        format!("reference {index} is not a class"),
                    ));
                }
                None => {
                    return Err(Error::syntax(
                        self.offset - 1,
                        format!("class reference {index} is out of bounds"),
                    ));
                }
            }
        };

        let mut full = literals.clone();
        full.extend(inherited);
        for index in 0..literals.len() {
            let mut chain = literals[index..].to_vec();
            chain.extend(full[literals.len()..].iter().cloned());
            self.objects.push(CacheEntry::Class(chain));
        }
        Ok(full)
    }

    fn read_c_string(&mut self) -> Result<Value> {
        let head = self.read_i8()?;
        if head == TAG_NIL {
            return Ok(Value::Null);
        }
        if head != TAG_NEW {
            return self.read_reference(ReferenceKind::CString, head);
        }
        let bytes = self
            .read_shared_string()?
            .ok_or_else(|| Error::syntax(self.offset, "nil literal C string"))?;
        if bytes.contains(&0) {
            return Err(Error::syntax(self.offset, "C string contains a zero byte"));
        }
        let value = bytes_to_value(bytes);
        self.objects.push(CacheEntry::CString(value.clone()));
        Ok(value)
    }

    fn read_reference(&mut self, kind: ReferenceKind, head: i8) -> Result<Value> {
        let index = self.decode_reference(head)?;
        let entry = self.objects.get(index).ok_or_else(|| {
            Error::syntax(
                self.offset - 1,
                format!("object reference {index} is out of bounds"),
            )
        })?;
        match (kind, entry) {
            (ReferenceKind::CString, CacheEntry::CString(value))
            | (ReferenceKind::Object, CacheEntry::Object(value)) => Ok(value.clone()),
            _ => Err(Error::syntax(
                self.offset - 1,
                format!("reference {index} has the wrong kind"),
            )),
        }
    }

    fn decode_reference(&mut self, head: i8) -> Result<usize> {
        let encoded = self.read_integer_with_head(head, true)?;
        let decoded = encoded - FIRST_REFERENCE;
        usize::try_from(decoded)
            .map_err(|_| Error::syntax(self.offset - 1, "negative reference index"))
    }

    fn read_unshared_string(&mut self) -> Result<Option<Vec<u8>>> {
        let head = self.read_i8()?;
        if head == TAG_NIL {
            return Ok(None);
        }
        let length = self.read_integer_with_head(head, false)?;
        let length = usize::try_from(length)
            .map_err(|_| Error::syntax(self.offset - 1, "negative string length"))?;
        if length > MAX_BLOB_LENGTH {
            return Err(Error::syntax(
                self.offset - 1,
                format!("blob length {length} exceeds limit"),
            ));
        }
        Ok(Some(self.read_exact(length)?.to_vec()))
    }

    fn read_shared_string(&mut self) -> Result<Option<Vec<u8>>> {
        let head = self.read_i8()?;
        if head == TAG_NIL {
            return Ok(None);
        }
        if head == TAG_NEW {
            let value = self
                .read_unshared_string()?
                .ok_or_else(|| Error::syntax(self.offset, "nil literal shared string"))?;
            self.shared_strings.push(value.clone());
            return Ok(Some(value));
        }
        let index = self.decode_reference(head)?;
        self.shared_strings
            .get(index)
            .cloned()
            .map(Some)
            .ok_or_else(|| {
                Error::syntax(
                    self.offset - 1,
                    format!("shared string reference {index} is out of bounds"),
                )
            })
    }

    fn read_integer(&mut self, signed: bool) -> Result<i64> {
        let head = self.read_i8()?;
        self.read_integer_with_head(head, signed)
    }

    fn read_integer_with_head(&mut self, head: i8, signed: bool) -> Result<i64> {
        if !(FIRST_TAG..=LAST_TAG).contains(&head) {
            return Ok(if signed {
                i64::from(head)
            } else {
                i64::from(head as u8)
            });
        }
        match head {
            TAG_INTEGER_2 => {
                let bytes: [u8; 2] = self.read_exact(2)?.try_into().expect("length checked");
                Ok(if signed {
                    i64::from(match self.byte_order {
                        ByteOrder::Little => i16::from_le_bytes(bytes),
                        ByteOrder::Big => i16::from_be_bytes(bytes),
                    })
                } else {
                    i64::from(match self.byte_order {
                        ByteOrder::Little => u16::from_le_bytes(bytes),
                        ByteOrder::Big => u16::from_be_bytes(bytes),
                    })
                })
            }
            TAG_INTEGER_4 => {
                let bytes: [u8; 4] = self.read_exact(4)?.try_into().expect("length checked");
                Ok(if signed {
                    i64::from(match self.byte_order {
                        ByteOrder::Little => i32::from_le_bytes(bytes),
                        ByteOrder::Big => i32::from_be_bytes(bytes),
                    })
                } else {
                    i64::from(match self.byte_order {
                        ByteOrder::Little => u32::from_le_bytes(bytes),
                        ByteOrder::Big => u32::from_be_bytes(bytes),
                    })
                })
            }
            _ => Err(Error::syntax(
                self.offset - 1,
                format!("invalid integer head {:#04x}", head as u8),
            )),
        }
    }

    fn read_float(&mut self) -> Result<f32> {
        let head = self.read_i8()?;
        if head != TAG_FLOATING_POINT {
            return Ok(self.read_integer_with_head(head, true)? as f32);
        }
        let bytes: [u8; 4] = self.read_exact(4)?.try_into().expect("length checked");
        Ok(match self.byte_order {
            ByteOrder::Little => f32::from_le_bytes(bytes),
            ByteOrder::Big => f32::from_be_bytes(bytes),
        })
    }

    fn read_double(&mut self) -> Result<f64> {
        let head = self.read_i8()?;
        if head != TAG_FLOATING_POINT {
            return Ok(self.read_integer_with_head(head, true)? as f64);
        }
        let bytes: [u8; 8] = self.read_exact(8)?.try_into().expect("length checked");
        Ok(match self.byte_order {
            ByteOrder::Little => f64::from_le_bytes(bytes),
            ByteOrder::Big => f64::from_be_bytes(bytes),
        })
    }

    fn check_depth(&self, depth: usize) -> Result<()> {
        if depth > MAX_DEPTH {
            Err(Error::syntax(self.offset, "nesting limit exceeded"))
        } else {
            Ok(())
        }
    }

    fn peek_i8(&self) -> Result<i8> {
        self.bytes
            .get(self.offset)
            .copied()
            .map(|byte| byte.cast_signed())
            .ok_or(Error::UnexpectedEof {
                offset: self.offset,
                needed: 1,
            })
    }

    fn read_i8(&mut self) -> Result<i8> {
        self.read_u8().map(|byte| byte.cast_signed())
    }

    fn read_u8(&mut self) -> Result<u8> {
        Ok(self.read_exact(1)?[0])
    }

    fn read_exact(&mut self, length: usize) -> Result<&'a [u8]> {
        let end = self
            .offset
            .checked_add(length)
            .ok_or_else(|| Error::syntax(self.offset, "offset overflow"))?;
        let value = self
            .bytes
            .get(self.offset..end)
            .ok_or(Error::UnexpectedEof {
                offset: self.offset,
                needed: length,
            })?;
        self.offset = end;
        Ok(value)
    }
}

fn bytes_to_value(bytes: Vec<u8>) -> Value {
    match String::from_utf8(bytes) {
        Ok(value) => Value::String(value),
        Err(error) => Value::Bytes(error.into_bytes()),
    }
}

fn normalize_object(object: ArchivedObject) -> Value {
    let class = object.classes.first().map(|value| value.name.as_str());
    let flattened: Vec<Value> = object
        .fields
        .iter()
        .flat_map(|field| field.values.iter().cloned())
        .collect();

    match class {
        Some("NSString" | "NSMutableString") => flattened
            .into_iter()
            .find_map(|value| match value {
                Value::Bytes(bytes) => Some(bytes_to_value(bytes)),
                Value::String(_) => Some(value),
                _ => None,
            })
            .unwrap_or(Value::Archived(object)),
        Some("NSArray" | "NSMutableArray" | "NSSet" | "NSMutableSet") => {
            Value::Array(flattened.into_iter().skip(1).collect())
        }
        Some("NSDictionary" | "NSMutableDictionary") => {
            let mut values = flattened.into_iter().skip(1);
            let mut map = BTreeMap::new();
            while let (Some(key), Some(value)) = (values.next(), values.next()) {
                let key = match key {
                    Value::String(value) => value,
                    other => format!("{other:?}"),
                };
                map.insert(key, value);
            }
            Value::Map(map)
        }
        Some("NSNumber" | "NSValue") => flattened
            .into_iter()
            .rev()
            .find(|value| {
                matches!(
                    value,
                    Value::Bool(_) | Value::I64(_) | Value::U64(_) | Value::F64(_)
                )
            })
            .unwrap_or(Value::Archived(object)),
        Some("NSData" | "NSMutableData") => flattened
            .into_iter()
            .find(|value| matches!(value, Value::Bytes(_)))
            .unwrap_or(Value::Archived(object)),
        _ => Value::Archived(object),
    }
}

fn encoding_name(encoding: &Encoding) -> String {
    match encoding {
        Encoding::Bool => "B",
        Encoding::I8 => "c",
        Encoding::U8 => "C",
        Encoding::I16 => "s",
        Encoding::U16 => "S",
        Encoding::I32 => "i",
        Encoding::U32 => "I",
        Encoding::I64 => "q",
        Encoding::U64 => "Q",
        Encoding::F32 => "f",
        Encoding::F64 => "d",
        Encoding::CString => "*",
        Encoding::Atom => "%",
        Encoding::Selector => ":",
        Encoding::Bytes => "+",
        Encoding::Class => "#",
        Encoding::Object => "@",
        Encoding::Ignored => "!",
        Encoding::Array(_, _) => "[]",
        Encoding::Struct(_, _) => "{}",
    }
    .to_owned()
}
