use std::sync::{Mutex, mpsc};

use objc2::rc::Retained;
use objc2_event_kit::EKEventStore;

use crate::{
    auth::{AuthSnapshot, EntityAuthStatus, ensure_events_authorized, ensure_reminders_authorized},
    error::{EventKitError, EventKitResult},
};

const OPERATION_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

pub struct EventKitStore {
    pub(crate) inner: Mutex<Retained<EKEventStore>>,
    auth: AuthSnapshot,
}

// EventKit objects are only accessed while holding the mutex or on the main queue.
unsafe impl Send for EventKitStore {}
unsafe impl Sync for EventKitStore {}

impl EventKitStore {
    pub fn new() -> EventKitResult<Self> {
        let store = unsafe { EKEventStore::new() };
        Ok(Self {
            inner: Mutex::new(store),
            auth: AuthSnapshot::new(),
        })
    }

    pub(crate) fn with_store<F, T>(&self, f: F) -> EventKitResult<T>
    where
        F: FnOnce(&EKEventStore) -> EventKitResult<T>,
    {
        let store = self
            .inner
            .lock()
            .map_err(|_| EventKitError::Framework("EventKit store lock poisoned".into()))?;
        f(&store)
    }

    pub async fn auth_status(&self) -> EntityAuthStatus {
        self.auth.status().await
    }

    pub async fn refresh_auth_status(&self) {
        self.auth.refresh().await;
    }

    pub(crate) fn ensure_reminders(&self) -> EventKitResult<()> {
        self.with_store(ensure_reminders_authorized)
    }

    pub(crate) fn ensure_events(&self) -> EventKitResult<()> {
        self.with_store(ensure_events_authorized)
    }

    pub(crate) async fn run_on_main<F, T>(&self, f: F) -> EventKitResult<T>
    where
        F: FnOnce(&EKEventStore) -> EventKitResult<T> + Send + 'static,
        T: Send + 'static,
    {
        let store_ptr = {
            let store = self
                .inner
                .lock()
                .map_err(|_| EventKitError::Framework("EventKit store lock poisoned".into()))?;
            Retained::as_ptr(&store) as usize
        };
        tokio::time::timeout(
            OPERATION_TIMEOUT,
            tokio::task::spawn_blocking(move || {
                let (tx, rx) = mpsc::sync_channel(1);
                dispatch2::DispatchQueue::main().exec_async(move || {
                    let store = unsafe { &*(store_ptr as *const EKEventStore) };
                    let _ = tx.send(f(store));
                });
                rx.recv()
                    .map_err(|_| EventKitError::Framework("main queue callback dropped".into()))?
            }),
        )
        .await
        .map_err(|_| EventKitError::Timeout)?
        .map_err(|_| EventKitError::Framework("blocking task failed".into()))?
    }
}

impl Clone for EventKitStore {
    fn clone(&self) -> Self {
        let store = self.inner.lock().expect("eventkit lock");
        Self {
            inner: Mutex::new(store.clone()),
            auth: AuthSnapshot::new(),
        }
    }
}
