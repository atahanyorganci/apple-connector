use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("serialize error: {0}")]
    Serialize(String),
    #[error("parse error: {0}")]
    Parse(String),
    /// An input budget was exhausted. `limit` names the budget so callers can
    /// tell which one tripped without matching on message text.
    #[error("{limit} limit exceeded: {actual} exceeds maximum {max}")]
    LimitExceeded {
        limit: &'static str,
        actual: usize,
        max: usize,
    },
    /// A parse failure tied to the line and property it came from, so callers
    /// see where in the vCard the problem is rather than a bare message.
    #[error("parse error on line {line} ({property}): {message}")]
    Property {
        line: usize,
        property: String,
        message: String,
    },
}

impl Error {
    pub(crate) fn property(
        line: usize,
        property: impl Into<String>,
        message: impl std::fmt::Display,
    ) -> Self {
        Self::Property {
            line,
            property: property.into(),
            message: message.to_string(),
        }
    }
}
