use crate::error::{EventKitError, EventKitResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthStatus {
    NotDetermined,
    Restricted,
    Denied,
    Authorized,
    WriteOnly,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntityAuthStatus {
    pub reminders: AuthStatus,
    pub events: AuthStatus,
}

pub struct EventKitStore;

impl EventKitStore {
    pub fn new() -> EventKitResult<Self> {
        Err(EventKitError::UnsupportedPlatform)
    }

    pub async fn auth_status(&self) -> EntityAuthStatus {
        EntityAuthStatus {
            reminders: AuthStatus::Unavailable,
            events: AuthStatus::Unavailable,
        }
    }

    pub async fn refresh_auth_status(&self) {}

    pub async fn request_access(&self) -> EventKitResult<()> {
        Ok(())
    }

    pub async fn ensure_reminders_access(&self) -> EventKitResult<()> {
        Ok(())
    }

    pub async fn ensure_events_access(&self) -> EventKitResult<()> {
        Ok(())
    }
}
