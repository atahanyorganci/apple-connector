use std::sync::{Mutex, mpsc};

use objc2_event_kit::{EKAuthorizationStatus, EKEntityType, EKEventStore};
use tokio::sync::RwLock;

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

impl AuthStatus {
    fn from_ek(status: EKAuthorizationStatus) -> Self {
        if status == EKAuthorizationStatus::NotDetermined {
            Self::NotDetermined
        } else if status == EKAuthorizationStatus::Restricted {
            Self::Restricted
        } else if status == EKAuthorizationStatus::Denied {
            Self::Denied
        } else if status == EKAuthorizationStatus::WriteOnly {
            Self::WriteOnly
        } else {
            Self::Authorized
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntityAuthStatus {
    pub reminders: AuthStatus,
    pub events: AuthStatus,
}

pub(crate) fn current_auth_status() -> EntityAuthStatus {
    let reminders = unsafe {
        AuthStatus::from_ek(EKEventStore::authorizationStatusForEntityType(
            EKEntityType::Reminder,
        ))
    };
    let events = unsafe {
        AuthStatus::from_ek(EKEventStore::authorizationStatusForEntityType(
            EKEntityType::Event,
        ))
    };
    EntityAuthStatus { reminders, events }
}

pub(crate) fn ensure_reminders_authorized(store: &EKEventStore) -> EventKitResult<()> {
    match current_auth_status().reminders {
        AuthStatus::Authorized | AuthStatus::WriteOnly => Ok(()),
        AuthStatus::Denied | AuthStatus::Restricted | AuthStatus::Unavailable => {
            Err(EventKitError::AccessDenied)
        }
        AuthStatus::NotDetermined => request_reminders_access(store),
    }
}

pub(crate) fn ensure_events_authorized(store: &EKEventStore) -> EventKitResult<()> {
    match current_auth_status().events {
        AuthStatus::Authorized | AuthStatus::WriteOnly => Ok(()),
        AuthStatus::Denied | AuthStatus::Restricted | AuthStatus::Unavailable => {
            Err(EventKitError::AccessDenied)
        }
        AuthStatus::NotDetermined => request_events_access(store),
    }
}

fn request_reminders_access(store: &EKEventStore) -> EventKitResult<()> {
    let (tx, rx) = mpsc::channel();
    let slot = Mutex::new(Some(tx));
    let block = block2::RcBlock::new(move |granted: objc2::runtime::Bool, _| {
        if let Some(tx) = slot.lock().expect("auth slot").take() {
            let _ = tx.send(granted.as_bool());
        }
    });
    unsafe {
        store.requestFullAccessToRemindersWithCompletion(block2::RcBlock::as_ptr(&block));
    }
    match rx.recv() {
        Ok(true) => Ok(()),
        Ok(false) => Err(EventKitError::AccessDenied),
        Err(_) => Err(EventKitError::Timeout),
    }
}

fn request_events_access(store: &EKEventStore) -> EventKitResult<()> {
    let (tx, rx) = mpsc::channel();
    let slot = Mutex::new(Some(tx));
    let block = block2::RcBlock::new(move |granted: objc2::runtime::Bool, _| {
        if let Some(tx) = slot.lock().expect("auth slot").take() {
            let _ = tx.send(granted.as_bool());
        }
    });
    unsafe {
        store.requestFullAccessToEventsWithCompletion(block2::RcBlock::as_ptr(&block));
    }
    match rx.recv() {
        Ok(true) => Ok(()),
        Ok(false) => Err(EventKitError::AccessDenied),
        Err(_) => Err(EventKitError::Timeout),
    }
}

pub(crate) struct AuthSnapshot(RwLock<EntityAuthStatus>);

impl AuthSnapshot {
    pub fn new() -> Self {
        Self(RwLock::new(current_auth_status()))
    }

    pub async fn refresh(&self) {
        *self.0.write().await = current_auth_status();
    }

    pub async fn status(&self) -> EntityAuthStatus {
        *self.0.read().await
    }
}

impl Default for AuthSnapshot {
    fn default() -> Self {
        Self::new()
    }
}
