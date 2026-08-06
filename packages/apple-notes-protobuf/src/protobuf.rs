#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WireType {
    Varint = 0,
    Fixed64 = 1,
    LengthDelimited = 2,
    Fixed32 = 5,
}

#[derive(Debug, Clone)]
pub struct Field {
    pub number: u32,
    pub varint: Option<u64>,
    pub bytes: Option<Vec<u8>>,
}

pub fn parse_message(data: &[u8]) -> Result<Vec<Field>, String> {
    let mut fields = Vec::new();
    let mut index = 0;

    while index < data.len() {
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
                if index + 8 > data.len() {
                    return Err("truncated fixed64 field".to_owned());
                }
                index += 8;
                Field {
                    number: field_number,
                    varint: None,
                    bytes: None,
                }
            }
            WireType::LengthDelimited => {
                let (length, next) = read_varint(data, index)?;
                index = next;
                let length = usize::try_from(length).map_err(|_| "length overflow".to_owned())?;
                if index + length > data.len() {
                    return Err("truncated length-delimited field".to_owned());
                }
                let bytes = data[index..index + length].to_vec();
                index += length;
                Field {
                    number: field_number,
                    varint: None,
                    bytes: Some(bytes),
                }
            }
            WireType::Fixed32 => {
                if index + 4 > data.len() {
                    return Err("truncated fixed32 field".to_owned());
                }
                index += 4;
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

pub fn read_tag(data: &[u8], index: usize) -> Result<(u32, WireType, usize), String> {
    let (tag, next) = read_varint(data, index)?;
    let wire_type = match tag & 0x07 {
        0 => WireType::Varint,
        1 => WireType::Fixed64,
        2 => WireType::LengthDelimited,
        5 => WireType::Fixed32,
        other => return Err(format!("unsupported wire type {other}")),
    };
    let field_number =
        u32::try_from(tag >> 3).map_err(|_| "field number exceeds u32 range".to_owned())?;
    if field_number == 0 {
        return Err("invalid field number 0".to_owned());
    }
    Ok((field_number, wire_type, next))
}

pub fn read_varint(data: &[u8], mut index: usize) -> Result<(u64, usize), String> {
    let mut value = 0_u64;
    let mut shift = 0;
    while index < data.len() {
        let byte = data[index];
        index += 1;
        value |= u64::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return Ok((value, index));
        }
        shift += 7;
        if shift > 63 {
            return Err("varint overflow".to_owned());
        }
    }
    Err("truncated varint".to_owned())
}

pub fn fields_by_number(fields: &[Field], number: u32) -> impl Iterator<Item = &Field> {
    fields.iter().filter(move |field| field.number == number)
}

pub fn first_bytes(fields: &[Field], number: u32) -> Option<&[u8]> {
    fields_by_number(fields, number).find_map(|field| field.bytes.as_deref())
}

pub fn all_bytes(fields: &[Field], number: u32) -> Vec<&[u8]> {
    fields_by_number(fields, number)
        .filter_map(|field| field.bytes.as_deref())
        .collect()
}
