use std::{fmt, io};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    InvalidHeader { offset: usize, message: String },
    UnexpectedEof { offset: usize, needed: usize },
    Syntax { offset: usize, message: String },
    Io(io::Error),
    Custom(String),
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidHeader { offset, message } => {
                write!(
                    formatter,
                    "invalid typedstream header at byte {offset}: {message}"
                )
            }
            Self::UnexpectedEof { offset, needed } => write!(
                formatter,
                "unexpected end of typedstream at byte {offset} (needed {needed} bytes)"
            ),
            Self::Syntax { offset, message } => {
                write!(
                    formatter,
                    "typedstream syntax error at byte {offset}: {message}"
                )
            }
            Self::Io(error) => error.fmt(formatter),
            Self::Custom(message) => write!(formatter, "{message}"),
        }
    }
}

impl Error {
    pub fn custom(message: impl fmt::Display) -> Self {
        Self::Custom(message.to_string())
    }

    pub(crate) fn syntax(offset: usize, message: impl fmt::Display) -> Self {
        Self::Syntax {
            offset,
            message: message.to_string(),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            _ => None,
        }
    }
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl serde::de::Error for Error {
    fn custom<T>(message: T) -> Self
    where
        T: fmt::Display,
    {
        Self::Custom(message.to_string())
    }
}

impl serde::ser::Error for Error {
    fn custom<T>(message: T) -> Self
    where
        T: fmt::Display,
    {
        Self::Custom(message.to_string())
    }
}
