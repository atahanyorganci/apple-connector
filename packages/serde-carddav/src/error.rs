use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("serialize error: {0}")]
    Serialize(String),
    #[error("parse error: {0}")]
    Parse(String),
}
