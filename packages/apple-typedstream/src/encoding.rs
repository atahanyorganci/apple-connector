use crate::{Error, Result};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Encoding {
    Bool,
    I8,
    U8,
    I16,
    U16,
    I32,
    U32,
    I64,
    U64,
    F32,
    F64,
    CString,
    Atom,
    Selector,
    Bytes,
    Class,
    Object,
    Ignored,
    Array(usize, Box<Encoding>),
    Struct(Option<String>, Vec<Encoding>),
}

pub(crate) fn parse_all(input: &[u8]) -> Result<Vec<Encoding>> {
    let mut offset = 0;
    let mut encodings = Vec::new();
    while offset < input.len() {
        encodings.push(parse_one(input, &mut offset)?);
    }
    if encodings.is_empty() {
        return Err(Error::syntax(0, "empty type encoding"));
    }
    Ok(encodings)
}

fn parse_one(input: &[u8], offset: &mut usize) -> Result<Encoding> {
    let start = *offset;
    let byte = *input
        .get(*offset)
        .ok_or_else(|| Error::syntax(start, "incomplete type encoding"))?;
    *offset += 1;

    let encoding = match byte {
        b'B' => Encoding::Bool,
        b'c' => Encoding::I8,
        b'C' => Encoding::U8,
        b's' => Encoding::I16,
        b'S' => Encoding::U16,
        b'i' | b'l' => Encoding::I32,
        b'I' | b'L' => Encoding::U32,
        b'q' => Encoding::I64,
        b'Q' => Encoding::U64,
        b'f' => Encoding::F32,
        b'd' => Encoding::F64,
        b'*' => Encoding::CString,
        b'%' => Encoding::Atom,
        b':' => Encoding::Selector,
        b'+' => Encoding::Bytes,
        b'#' => Encoding::Class,
        b'@' => Encoding::Object,
        b'!' => Encoding::Ignored,
        b'[' => {
            let digits_start = *offset;
            while input.get(*offset).is_some_and(u8::is_ascii_digit) {
                *offset += 1;
            }
            if digits_start == *offset {
                return Err(Error::syntax(start, "array encoding has no length"));
            }
            let length = std::str::from_utf8(&input[digits_start..*offset])
                .map_err(|_| Error::syntax(start, "invalid array length"))?
                .parse()
                .map_err(|_| Error::syntax(start, "array length overflows usize"))?;
            let element = parse_one(input, offset)?;
            if input.get(*offset) != Some(&b']') {
                return Err(Error::syntax(start, "unterminated array encoding"));
            }
            *offset += 1;
            Encoding::Array(length, Box::new(element))
        }
        b'{' => {
            let content_start = *offset;
            let mut equals = None;
            while let Some(&current) = input.get(*offset) {
                if current == b'=' {
                    equals = Some(*offset);
                    *offset += 1;
                    break;
                }
                if matches!(current, b'{' | b'}') {
                    break;
                }
                *offset += 1;
            }

            let name = equals.map(|end| String::from_utf8_lossy(&input[content_start..end]).into());
            if equals.is_none() {
                *offset = content_start;
            }

            let mut fields = Vec::new();
            while input.get(*offset) != Some(&b'}') {
                if *offset >= input.len() {
                    return Err(Error::syntax(start, "unterminated struct encoding"));
                }
                fields.push(parse_one(input, offset)?);
            }
            *offset += 1;
            Encoding::Struct(name, fields)
        }
        _ => {
            return Err(Error::syntax(
                start,
                format!("unsupported type encoding byte {byte:#04x}"),
            ));
        }
    };

    Ok(encoding)
}
