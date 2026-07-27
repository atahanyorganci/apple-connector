use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum EventKitError {
    #[error("item not found")]
    NotFound,
    #[error("EventKit access denied")]
    AccessDenied,
    #[error("calendar is read-only")]
    ReadOnlyCalendar,
    #[error("validation failed: {0}")]
    ValidationFailed(String),
    #[error("ambiguous match: {0}")]
    AmbiguousMatch(String),
    #[error("EventKit is unavailable on this platform")]
    UnsupportedPlatform,
    #[error("EventKit framework error: {0}")]
    Framework(String),
    #[error("EventKit operation timed out")]
    Timeout,
}

pub type EventKitResult<T> = Result<T, EventKitError>;
