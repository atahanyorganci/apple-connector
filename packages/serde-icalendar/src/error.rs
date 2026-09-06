use std::fmt;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Parse(String),
    Serialize(String),
    Custom(String),
    /// An input budget was exhausted. `limit` names the budget so callers can
    /// tell which one tripped without matching on message text.
    LimitExceeded {
        limit: &'static str,
        actual: usize,
        max: usize,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse(message) | Self::Serialize(message) | Self::Custom(message) => {
                f.write_str(message)
            }
            Self::LimitExceeded { limit, actual, max } => {
                write!(f, "{limit} limit exceeded: {actual} exceeds maximum {max}")
            }
        }
    }
}

impl std::error::Error for Error {}

impl serde::ser::Error for Error {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Self::Custom(msg.to_string())
    }
}

impl serde::de::Error for Error {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Self::Custom(msg.to_string())
    }
}
