use std::sync::Mutex;

use objc2::rc::Retained;
use objc2_contacts::CNContactStore;
use tokio::task::JoinError;

use crate::{
    auth::{AuthSnapshot, ensure_contacts_access, request_pending_access},
    error::{ContactsError, ContactsResult},
};

const OPERATION_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

pub struct ContactsStore {
    pub(crate) inner: Mutex<Retained<CNContactStore>>,
    auth: AuthSnapshot,
}

// Contacts objects are only accessed while holding the mutex or on the main queue.
unsafe impl Send for ContactsStore {}
unsafe impl Sync for ContactsStore {}

impl ContactsStore {
    pub fn new() -> ContactsResult<Self> {
        let store = unsafe { CNContactStore::new() };
        Ok(Self {
            inner: Mutex::new(store),
            auth: AuthSnapshot::new(),
        })
    }

    pub(crate) fn with_store<F, T>(&self, f: F) -> ContactsResult<T>
    where
        F: FnOnce(&CNContactStore) -> ContactsResult<T>,
    {
        let store = self
            .inner
            .lock()
            .map_err(|_| ContactsError::Framework("Contacts store lock poisoned".into()))?;
        f(&store)
    }

    pub async fn auth_status(&self) -> crate::auth::AuthStatus {
        self.auth.status().await
    }

    pub async fn refresh_auth_status(&self) {
        self.auth.refresh().await;
    }

    async fn run_blocking<F, T>(&self, f: F) -> ContactsResult<T>
    where
        F: FnOnce(&CNContactStore) -> ContactsResult<T> + Send + 'static,
        T: Send + 'static,
    {
        let store_ptr = {
            let store = self
                .inner
                .lock()
                .map_err(|_| ContactsError::Framework("Contacts store lock poisoned".into()))?;
            Retained::as_ptr(&store) as usize
        };
        tokio::task::spawn_blocking(move || {
            let store = unsafe { &*(store_ptr as *const CNContactStore) };
            f(store)
        })
        .await
        .map_err(join_error)?
    }

    /// Prompt for Contacts access when status is `NotDetermined`.
    pub async fn request_access(&self) -> ContactsResult<()> {
        self.run_blocking(request_pending_access).await?;
        self.refresh_auth_status().await;
        Ok(())
    }

    pub async fn ensure_contacts_access(&self) -> ContactsResult<()> {
        self.run_blocking(ensure_contacts_access).await?;
        self.refresh_auth_status().await;
        Ok(())
    }

    pub(crate) fn ensure_contacts(&self) -> ContactsResult<()> {
        self.with_store(ensure_contacts_access)
    }

    pub(crate) async fn run_on_main<F, T>(&self, f: F) -> ContactsResult<T>
    where
        F: FnOnce(&CNContactStore) -> ContactsResult<T> + Send + 'static,
        T: Send + 'static,
    {
        tokio::time::timeout(OPERATION_TIMEOUT, self.run_blocking(f))
            .await
            .map_err(|_| ContactsError::Timeout)?
    }
}

fn join_error(_: JoinError) -> ContactsError {
    ContactsError::Framework("blocking task failed".into())
}

impl Clone for ContactsStore {
    fn clone(&self) -> Self {
        let store = match self.inner.lock() {
            Ok(guard) => guard.clone(),
            Err(poisoned) => poisoned.into_inner().clone(),
        };
        Self {
            inner: Mutex::new(store),
            auth: AuthSnapshot::new(),
        }
    }
}
