use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum EventStatus {
    #[default]
    Confirmed,
    Tentative,
    Cancelled,
}

impl EventStatus {
    pub fn from_raw(value: Option<i64>) -> Self {
        match value.unwrap_or(0) {
            1 => Self::Tentative,
            2 => Self::Cancelled,
            _ => Self::Confirmed,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum InvitationStatus {
    #[default]
    Unknown,
    Accepted,
    Declined,
    Tentative,
    NeedsAction,
}

impl InvitationStatus {
    pub fn from_raw(value: Option<i64>) -> Self {
        match value.unwrap_or(0) {
            1 => Self::Accepted,
            2 => Self::Declined,
            3 => Self::Tentative,
            4 => Self::NeedsAction,
            _ => Self::Unknown,
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
}

impl Availability {
    pub fn from_raw(value: Option<i64>) -> Self {
        match value.unwrap_or(0) {
            1 => Self::Free,
            2 => Self::Tentative,
            3 => Self::Unavailable,
            _ => Self::Busy,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PrivacyLevel {
    #[default]
    Default,
    Public,
    Private,
}

impl PrivacyLevel {
    pub fn from_raw(value: Option<i64>) -> Self {
        match value.unwrap_or(0) {
            1 => Self::Public,
            2 => Self::Private,
            _ => Self::Default,
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
}

impl StoreType {
    pub fn from_raw(value: Option<i64>) -> Self {
        match value.unwrap_or(0) {
            1 => Self::CalDav,
            2 => Self::Exchange,
            3 => Self::Subscription,
            4 => Self::Birthday,
            _ => Self::Local,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EventClass {
    #[default]
    Standard,
    Birthday,
    SpecialDay,
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
        match entity_type.unwrap_or(0) {
            1 => Self::Birthday,
            2 => Self::SpecialDay,
            _ => Self::Standard,
        }
    }
}
