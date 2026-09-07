use std::{sync::Arc, time::Duration};

use objc2::rc::Retained;
use objc2_contacts::CNContactStore;

use crate::{
    auth::{
        AuthOutcome, AuthSnapshot, AuthStatus, PROMPT_JOB_BUDGET, current_auth_status,
        ensure_contacts_access,
    },
    error::{ContactsError, ContactsResult},
    worker::{Worker, WorkerError},
};

/// Budget for a single framework operation.
const OPERATION_TIMEOUT: Duration = Duration::from_secs(30);

struct Inner {
    worker: Worker<Retained<CNContactStore>>,
    auth: AuthSnapshot,
}

/// Handle to the process-wide Contacts store.
///
/// The `CNContactStore` lives on the worker thread and is never shared: callers submit closures
/// that borrow it for one job, and only `Send` results come back. Cloning shares the same worker
/// and the same authorization snapshot, so a clone cannot forget that access was granted.
#[derive(Clone)]
pub struct ContactsStore {
    inner: Arc<Inner>,
}

impl ContactsStore {
    pub fn new() -> ContactsResult<Self> {
        let (worker, initial) = Worker::spawn("apple-contacts", || {
            let store = unsafe { CNContactStore::new() };
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

    pub async fn auth_status(&self) -> AuthStatus {
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

    /// Prompts for Contacts access when the status is `NotDetermined`, and reports what
    /// happened.
    pub async fn request_access(&self) -> ContactsResult<AuthOutcome> {
        let outcome = self
            .run_with_timeout(PROMPT_JOB_BUDGET, crate::auth::request_pending_access)
            .await;
        self.refresh_auth_status().await;
        outcome
    }

    pub async fn ensure_contacts_access(&self) -> ContactsResult<()> {
        let outcome = self
            .run_with_timeout(PROMPT_JOB_BUDGET, ensure_contacts_access)
            .await;
        self.refresh_auth_status().await;
        outcome
    }

    pub(crate) async fn ensure_contacts(&self) -> ContactsResult<()> {
        self.run_with_timeout(PROMPT_JOB_BUDGET, ensure_contacts_access)
            .await
    }

    /// Runs `f` against the store on the worker thread.
    pub(crate) async fn run<F, T>(&self, f: F) -> ContactsResult<T>
    where
        F: FnOnce(&CNContactStore) -> ContactsResult<T> + Send + 'static,
        T: Send + 'static,
    {
        self.run_with_timeout(OPERATION_TIMEOUT, f).await
    }

    async fn run_with_timeout<F, T>(&self, budget: Duration, f: F) -> ContactsResult<T>
    where
        F: FnOnce(&CNContactStore) -> ContactsResult<T> + Send + 'static,
        T: Send + 'static,
    {
        self.inner
            .worker
            .run(budget, move |store: &Retained<CNContactStore>| f(store))
            .await
            .map_err(worker_error)?
    }
}

fn worker_error(error: WorkerError) -> ContactsError {
    match error {
        WorkerError::Stopped => ContactsError::Framework("Contacts worker is not running".into()),
        WorkerError::Lost => {
            ContactsError::Framework("Contacts worker dropped the operation".into())
        }
        WorkerError::TimedOut => ContactsError::Timeout,
    }
}
