use std::{sync::Arc, time::Duration};

use objc2::rc::Retained;
use objc2_event_kit::EKEventStore;

use crate::{
    auth::{
        AuthSnapshot, EntityAuthStatus, current_auth_status, ensure_events_authorized,
        ensure_reminders_authorized,
    },
    error::{EventKitError, EventKitResult},
    worker::{Worker, WorkerError},
};

/// Budget for a single framework operation.
const OPERATION_TIMEOUT: Duration = Duration::from_secs(30);
/// Authorization waits on a person answering a system prompt, so it gets its own budget.
const AUTH_TIMEOUT: Duration = Duration::from_secs(180);

struct Inner {
    worker: Worker<Retained<EKEventStore>>,
    auth: AuthSnapshot,
}

/// Handle to the process-wide EventKit store.
///
/// The `EKEventStore` lives on the worker thread and is never shared: callers submit closures
/// that borrow it for one job, and only `Send` results come back. Cloning shares the same worker
/// and the same authorization snapshot, so a clone cannot forget that access was granted.
#[derive(Clone)]
pub struct EventKitStore {
    inner: Arc<Inner>,
}

impl EventKitStore {
    pub fn new() -> EventKitResult<Self> {
        let (worker, initial) = Worker::spawn("apple-eventkit", || {
            let store = unsafe { EKEventStore::new() };
            let status = current_auth_status();
            (store, status)
        })
        .map_err(worker_error)?;

        Ok(Self {
            inner: Arc::new(Inner {
                worker,
                auth: AuthSnapshot::new(initial),
            }),
        })
    }

    pub async fn auth_status(&self) -> EntityAuthStatus {
        self.inner.auth.status().await
    }

    pub async fn refresh_auth_status(&self) {
        if let Ok(status) = self
            .inner
            .worker
            .run(OPERATION_TIMEOUT, |_| current_auth_status())
            .await
        {
            self.inner.auth.store(status).await;
        }
    }

    /// Prompt for Reminders and Calendar access when status is `NotDetermined`.
    pub async fn request_access(&self) -> EventKitResult<()> {
        let outcome = self
            .run_with_timeout(AUTH_TIMEOUT, crate::auth::request_pending_access)
            .await;
        self.refresh_auth_status().await;
        outcome
    }

    pub async fn ensure_reminders_access(&self) -> EventKitResult<()> {
        let outcome = self
            .run_with_timeout(AUTH_TIMEOUT, ensure_reminders_authorized)
            .await;
        self.refresh_auth_status().await;
        outcome
    }

    pub async fn ensure_events_access(&self) -> EventKitResult<()> {
        let outcome = self
            .run_with_timeout(AUTH_TIMEOUT, ensure_events_authorized)
            .await;
        self.refresh_auth_status().await;
        outcome
    }

    pub(crate) async fn ensure_reminders(&self) -> EventKitResult<()> {
        self.run_with_timeout(AUTH_TIMEOUT, ensure_reminders_authorized)
            .await
    }

    pub(crate) async fn ensure_events(&self) -> EventKitResult<()> {
        self.run_with_timeout(AUTH_TIMEOUT, ensure_events_authorized)
            .await
    }

    /// Runs `f` against the store on the worker thread.
    pub(crate) async fn run<F, T>(&self, f: F) -> EventKitResult<T>
    where
        F: FnOnce(&EKEventStore) -> EventKitResult<T> + Send + 'static,
        T: Send + 'static,
    {
        self.run_with_timeout(OPERATION_TIMEOUT, f).await
    }

    async fn run_with_timeout<F, T>(&self, budget: Duration, f: F) -> EventKitResult<T>
    where
        F: FnOnce(&EKEventStore) -> EventKitResult<T> + Send + 'static,
        T: Send + 'static,
    {
        self.inner
            .worker
            .run(budget, move |store: &Retained<EKEventStore>| f(store))
            .await
            .map_err(worker_error)?
    }
}

fn worker_error(error: WorkerError) -> EventKitError {
    match error {
        WorkerError::Stopped => EventKitError::Framework("EventKit worker is not running".into()),
        WorkerError::Lost => {
            EventKitError::Framework("EventKit worker dropped the operation".into())
        }
        WorkerError::TimedOut => EventKitError::Timeout,
    }
}
