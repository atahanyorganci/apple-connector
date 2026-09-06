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
}
