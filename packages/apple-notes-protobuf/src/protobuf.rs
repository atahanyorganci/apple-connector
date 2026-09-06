use crate::DecodeError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WireType {
    Varint = 0,
    Fixed64 = 1,
    LengthDelimited = 2,
    Fixed32 = 5,
}

/// A single decoded protobuf field.
///
/// Length-delimited payloads borrow from the message being parsed, so nested
/// sub-messages never copy their bytes.
#[derive(Debug, Clone, Copy)]
pub struct Field<'a> {
    pub number: u32,
    pub varint: Option<u64>,
    pub bytes: Option<&'a [u8]>,
}

/// Decoding budget shared by every message parsed out of one note body.
///
/// Field counts accumulate across nested messages so a document cannot spend an
/// unbounded total by splitting itself into many small sub-messages.
pub struct Budget {
    max_fields: usize,
    max_depth: usize,
    fields_used: usize,
}

impl Budget {
    pub fn new(max_fields: usize, max_depth: usize) -> Self {
        Self {
            max_fields,
            max_depth,
            fields_used: 0,
        }
    }

    fn charge_field(&mut self) -> Result<(), DecodeError> {
        self.fields_used = self.fields_used.saturating_add(1);
        if self.fields_used > self.max_fields {
            return Err(DecodeError::LimitExceeded {
                limit: "protobuf field count",
                actual: self.fields_used,
                max: self.max_fields,
            });
        }
        Ok(())
    }

    fn check_depth(&self, depth: usize) -> Result<(), DecodeError> {
        if depth > self.max_depth {
            return Err(DecodeError::LimitExceeded {
                limit: "protobuf nesting depth",
                actual: depth,
                max: self.max_depth,
            });
        }
        Ok(())
    }
}

fn invalid(message: impl Into<String>) -> DecodeError {
    DecodeError::InvalidProtobuf(message.into())
}

pub fn parse_message<'a>(
    data: &'a [u8],
    budget: &mut Budget,
    depth: usize,
) -> Result<Vec<Field<'a>>, DecodeError> {
    budget.check_depth(depth)?;

    let mut fields = Vec::new();
    let mut index = 0;

    while index < data.len() {
        budget.charge_field()?;

        let (field_number, wire_type, next) = read_tag(data, index)?;
        index = next;

        let field = match wire_type {
            WireType::Varint => {
                let (value, next) = read_varint(data, index)?;
                index = next;
                Field {
                    number: field_number,
                    varint: Some(value),
                    bytes: None,
                }
            }
            WireType::Fixed64 => {
                index = advance(index, 8, data.len(), "truncated fixed64 field")?;
                Field {
                    number: field_number,
                    varint: None,
                    bytes: None,
                }
            }
            WireType::LengthDelimited => {
                let (length, next) = read_varint(data, index)?;
                index = next;
                let length = usize::try_from(length).map_err(|_| invalid("length overflow"))?;
                let end = advance(
                    index,
                    length,
                    data.len(),
                    "truncated length-delimited field",
                )?;
                let bytes = data
                    .get(index..end)
                    .ok_or_else(|| invalid("truncated length-delimited field"))?;
                index = end;
                Field {
                    number: field_number,
                    varint: None,
                    bytes: Some(bytes),
                }
            }
            WireType::Fixed32 => {
                index = advance(index, 4, data.len(), "truncated fixed32 field")?;
                Field {
                    number: field_number,
                    varint: None,
                    bytes: None,
                }
            }
        };

        fields.push(field);
    }

    Ok(fields)
}

fn advance(index: usize, length: usize, limit: usize, message: &str) -> Result<usize, DecodeError> {
    let end = index
        .checked_add(length)
        .ok_or_else(|| invalid("field length overflows the message"))?;
    if end > limit {
        return Err(invalid(message));
    }
    Ok(end)
}

pub fn read_tag(data: &[u8], index: usize) -> Result<(u32, WireType, usize), DecodeError> {
    let (tag, next) = read_varint(data, index)?;
    let wire_type = match tag & 0x07 {
        0 => WireType::Varint,
        1 => WireType::Fixed64,
        2 => WireType::LengthDelimited,
        5 => WireType::Fixed32,
        other => return Err(invalid(format!("unsupported wire type {other}"))),
    };
    let field_number =
        u32::try_from(tag >> 3).map_err(|_| invalid("field number exceeds u32 range"))?;
    if field_number == 0 {
        return Err(invalid("invalid field number 0"));
    }
    Ok((field_number, wire_type, next))
}

pub fn read_varint(data: &[u8], mut index: usize) -> Result<(u64, usize), DecodeError> {
    let mut value = 0_u64;
    let mut shift = 0;
    while let Some(&byte) = data.get(index) {
        index += 1;
        value |= u64::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return Ok((value, index));
        }
        shift += 7;
        if shift > 63 {
            return Err(invalid("varint overflow"));
        }
    }
    Err(invalid("truncated varint"))
}

pub fn fields_by_number<'a, 'b>(
    fields: &'b [Field<'a>],
    number: u32,
) -> impl Iterator<Item = &'b Field<'a>> {
    fields.iter().filter(move |field| field.number == number)
}

pub fn first_bytes<'a>(fields: &[Field<'a>], number: u32) -> Option<&'a [u8]> {
    fields_by_number(fields, number).find_map(|field| field.bytes)
}

pub fn all_bytes<'a>(fields: &[Field<'a>], number: u32) -> Vec<&'a [u8]> {
    fields_by_number(fields, number)
        .filter_map(|field| field.bytes)
        .collect()
}
