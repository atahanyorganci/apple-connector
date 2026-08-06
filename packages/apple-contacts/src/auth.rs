use std::{
    sync::{Mutex, mpsc},
    time::{Duration, Instant},
};

use objc2_contacts::{CNAuthorizationStatus, CNContactStore, CNEntityType};
use objc2_foundation::{NSDate, NSDefaultRunLoopMode, NSRunLoop};
use tokio::sync::RwLock;

use crate::error::{ContactsError, ContactsResult};

const AUTH_TIMEOUT: Duration = Duration::from_secs(120);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthStatus {
    NotDetermined,
    Restricted,
    Denied,
    Authorized,
    Limited,
    Unavailable,
}

impl AuthStatus {
    fn from_cn(status: CNAuthorizationStatus) -> Self {
        if status == CNAuthorizationStatus::NotDetermined {
            Self::NotDetermined
        } else if status == CNAuthorizationStatus::Restricted {
            Self::Restricted
        } else if status == CNAuthorizationStatus::Denied {
            Self::Denied
        } else if status == CNAuthorizationStatus::Limited {
            Self::Limited
        } else {
            Self::Authorized
        }
    }
}

pub(crate) fn current_auth_status() -> AuthStatus {
    unsafe {
        AuthStatus::from_cn(CNContactStore::authorizationStatusForEntityType(
            CNEntityType::Contacts,
        ))
    }
}

pub(crate) fn request_pending_access(_store: &CNContactStore) -> ContactsResult<()> {
    if current_auth_status() == AuthStatus::NotDetermined {
        let _ = request_contacts_access(_store);
    }
    Ok(())
}

pub(crate) fn ensure_contacts_access(_store: &CNContactStore) -> ContactsResult<()> {
    match current_auth_status() {
        AuthStatus::Authorized | AuthStatus::Limited => Ok(()),
        AuthStatus::Denied | AuthStatus::Restricted | AuthStatus::Unavailable => {
            Err(ContactsError::AccessDenied)
        }
        AuthStatus::NotDetermined => request_contacts_access(_store),
    }
}

fn request_contacts_access(store: &CNContactStore) -> ContactsResult<()> {
    let (tx, rx) = mpsc::sync_channel(1);
    let slot = Mutex::new(Some(tx));
    let block = block2::RcBlock::new(move |granted: objc2::runtime::Bool, _| {
        if let Ok(mut guard) = slot.lock()
            && let Some(tx) = guard.take()
        {
            let _ = tx.send(granted.as_bool());
        }
    });
    unsafe {
        store.requestAccessForEntityType_completionHandler(CNEntityType::Contacts, &block);
    }
    wait_for_auth(rx)
}

fn wait_for_auth(rx: mpsc::Receiver<bool>) -> ContactsResult<()> {
    let deadline = Instant::now() + AUTH_TIMEOUT;
    loop {
        match rx.try_recv() {
            Ok(true) => return Ok(()),
            Ok(false) => return Err(ContactsError::AccessDenied),
            Err(mpsc::TryRecvError::Empty) => {}
            Err(mpsc::TryRecvError::Disconnected) => {
                return Err(ContactsError::Framework(
                    "Contacts auth callback dropped".into(),
                ));
            }
        }
        if Instant::now() >= deadline {
            return Err(ContactsError::Timeout);
        }
        let until = NSDate::dateWithTimeIntervalSinceNow(0.05);
        unsafe {
            NSRunLoop::currentRunLoop().runMode_beforeDate(NSDefaultRunLoopMode, &until);
        }
    }
}

pub(crate) struct AuthSnapshot(RwLock<AuthStatus>);

impl AuthSnapshot {
    pub fn new() -> Self {
        Self(RwLock::new(current_auth_status()))
    }

    pub async fn refresh(&self) {
        *self.0.write().await = current_auth_status();
    }

    pub async fn status(&self) -> AuthStatus {
        *self.0.read().await
    }
}

impl Default for AuthSnapshot {
    fn default() -> Self {
        Self::new()
    }
}
