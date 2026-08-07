use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum EventStatus {
    #[default]
    Confirmed,
    Tentative,
    Cancelled,
    Unknown(i64),
}

impl EventStatus {
    pub fn from_raw(value: Option<i64>) -> Self {
        match value {
            None | Some(0) => Self::Confirmed,
            Some(1) => Self::Tentative,
            Some(2) => Self::Cancelled,
            Some(code) => Self::Unknown(code),
        }
    }

    #[must_use]
    #[allow(dead_code)] // used in tests; public round-trip API
    pub fn raw_code(self) -> i64 {
        match self {
            Self::Confirmed => 0,
            Self::Tentative => 1,
            Self::Cancelled => 2,
            Self::Unknown(code) => code,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum InvitationStatus {
    #[default]
    Unknown,
    Accepted,
    Declined,
    Tentative,
    NeedsAction,
    Raw(i64),
}

impl InvitationStatus {
    pub fn from_raw(value: Option<i64>) -> Self {
        match value {
            None | Some(0) => Self::Unknown,
            Some(1) => Self::Accepted,
            Some(2) => Self::Declined,
            Some(3) => Self::Tentative,
            Some(4) => Self::NeedsAction,
            Some(code) => Self::Raw(code),
        }
    }

    #[must_use]
    #[allow(dead_code)] // used in tests; public round-trip API
    pub fn raw_code(self) -> i64 {
        match self {
            Self::Unknown => 0,
            Self::Accepted => 1,
            Self::Declined => 2,
            Self::Tentative => 3,
            Self::NeedsAction => 4,
            Self::Raw(code) => code,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Availability {
    #[default]
    Busy,
    Free,
    Tentative,
    Unavailable,
    Unknown(i64),
}

impl Availability {
    pub fn from_raw(value: Option<i64>) -> Self {
        match value {
            None | Some(0) => Self::Busy,
            Some(1) => Self::Free,
            Some(2) => Self::Tentative,
            Some(3) => Self::Unavailable,
            Some(code) => Self::Unknown(code),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PrivacyLevel {
    #[default]
    Default,
    Public,
    Private,
    Unknown(i64),
}

impl PrivacyLevel {
    pub fn from_raw(value: Option<i64>) -> Self {
        match value {
            None | Some(0) => Self::Default,
            Some(1) => Self::Public,
            Some(2) => Self::Private,
            Some(code) => Self::Unknown(code),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StoreType {
    #[default]
    Local,
    CalDav,
    Exchange,
    Subscription,
    Birthday,
    Unknown(i64),
}

impl StoreType {
    pub fn from_raw(value: Option<i64>) -> Self {
        match value {
            None | Some(0) => Self::Local,
            Some(1) => Self::CalDav,
            Some(2) => Self::Exchange,
            Some(3) => Self::Subscription,
            Some(4) => Self::Birthday,
            Some(code) => Self::Unknown(code),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EventClass {
    #[default]
    Standard,
    Birthday,
    SpecialDay,
    Unknown(i64),
}

impl EventClass {
    pub fn from_row(
        entity_type: Option<i64>,
        birthday_id: Option<i64>,
        special_day: Option<&str>,
    ) -> Self {
        if birthday_id.is_some_and(|id| id > 0) {
            return Self::Birthday;
        }
        if special_day.is_some_and(|s| !s.is_empty()) {
            return Self::SpecialDay;
        }
        match entity_type {
            None | Some(0) => Self::Standard,
            Some(1) => Self::Birthday,
            Some(2) => Self::SpecialDay,
            Some(code) => Self::Unknown(code),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{EventStatus, InvitationStatus};

    #[test]
    fn event_status_round_trips_known_codes() {
        for code in [0, 1, 2] {
            let status = EventStatus::from_raw(Some(code));
            assert_eq!(status.raw_code(), code);
        }
    }

    #[test]
    fn invitation_status_round_trips_known_codes() {
        for code in [0, 1, 2, 3, 4] {
            let status = InvitationStatus::from_raw(Some(code));
            assert_eq!(status.raw_code(), code);
        }
    }
}
