use std::{sync::mpsc, time::Duration};

use objc2_contacts::{CNAuthorizationStatus, CNContactStore, CNEntityType};
use tokio::sync::RwLock;

use crate::error::{ContactsError, ContactsResult};

/// How long the system prompt may stay unanswered.
const PROMPT_TIMEOUT: Duration = Duration::from_secs(120);

/// Budget for a worker job that may show the prompt, with slack.
pub(crate) const PROMPT_JOB_BUDGET: Duration = Duration::from_secs(200);

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

/// What an access request actually did.
///
/// Startup needs to tell these apart: a denial, an unanswered prompt, and an entity the system
/// does not offer all leave the status undecided or refused, but call for different messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthOutcome {
    /// Access was already granted before this request, so no prompt was shown.
    AlreadyGranted,
    /// The prompt was shown and answered affirmatively.
    Granted,
    /// Access is refused — the prompt was declined, or it had been declined earlier.
    Denied,
    /// Device policy forbids access and it cannot be requested.
    Restricted,
    /// The prompt was not answered within the authorization budget.
    TimedOut,
    /// Contacts is not available on this system.
    Unavailable,
    /// The Contacts framework refused the request for some other reason.
    Failed,
}

impl AuthOutcome {
    pub fn is_granted(self) -> bool {
        matches!(self, Self::AlreadyGranted | Self::Granted)
    }
}

/// Outcome implied by the current status, or `None` when a prompt is still needed.
fn outcome_for_status(status: AuthStatus) -> Option<AuthOutcome> {
    match status {
        AuthStatus::Authorized | AuthStatus::Limited => Some(AuthOutcome::AlreadyGranted),
        AuthStatus::Denied => Some(AuthOutcome::Denied),
        AuthStatus::Restricted => Some(AuthOutcome::Restricted),
        AuthStatus::Unavailable => Some(AuthOutcome::Unavailable),
        AuthStatus::NotDetermined => None,
    }
}

/// Outcome of a prompt that has run to completion.
fn outcome_for_request(result: ContactsResult<()>) -> AuthOutcome {
    match result {
        Ok(()) => AuthOutcome::Granted,
        Err(ContactsError::AccessDenied) => AuthOutcome::Denied,
        Err(ContactsError::Timeout) => AuthOutcome::TimedOut,
        Err(ContactsError::UnsupportedPlatform) => AuthOutcome::Unavailable,
        Err(_) => AuthOutcome::Failed,
    }
}

/// Requests access when it is still undecided and reports what happened.
pub(crate) fn request_pending_access(store: &CNContactStore) -> ContactsResult<AuthOutcome> {
    Ok(outcome_for_status(current_auth_status())
        .unwrap_or_else(|| outcome_for_request(request_contacts_access(store))))
}

pub(crate) fn ensure_contacts_access(store: &CNContactStore) -> ContactsResult<()> {
    match current_auth_status() {
        AuthStatus::Authorized | AuthStatus::Limited => Ok(()),
        AuthStatus::Denied | AuthStatus::Restricted | AuthStatus::Unavailable => {
            Err(ContactsError::AccessDenied)
        }
        AuthStatus::NotDetermined => request_contacts_access(store),
    }
}

fn request_contacts_access(store: &CNContactStore) -> ContactsResult<()> {
    let (tx, rx) = mpsc::sync_channel(1);
    // `SyncSender` needs no lock: the channel has room for exactly one value, so a second
    // completion callback is discarded by `try_send` instead of being lost behind a poisoned
    // mutex — which previously stalled the caller for the whole authorization timeout.
    let block = block2::RcBlock::new(move |granted: objc2::runtime::Bool, _| {
        let _ = tx.try_send(granted.as_bool());
    });
    unsafe {
        store.requestAccessForEntityType_completionHandler(CNEntityType::Contacts, &block);
    }
    wait_for_auth(rx)
}

fn wait_for_auth(rx: mpsc::Receiver<bool>) -> ContactsResult<()> {
    // The Contacts framework delivers the completion handler on its own queue, so the worker
    // thread can block here. It previously polled while pumping `NSRunLoop::currentRunLoop`,
    // which returns immediately on a thread with no input sources — a full-CPU spin for as long
    // as the prompt stayed on screen.
    match rx.recv_timeout(PROMPT_TIMEOUT) {
        Ok(true) => Ok(()),
        Ok(false) => Err(ContactsError::AccessDenied),
        Err(mpsc::RecvTimeoutError::Timeout) => Err(ContactsError::Timeout),
        Err(mpsc::RecvTimeoutError::Disconnected) => Err(ContactsError::Framework(
            "Contacts auth callback dropped".into(),
        )),
    }
}

/// Last observed authorization status.
///
/// The snapshot is only ever written with a value read on the worker thread, so it cannot drift
/// from what the framework reports.
pub(crate) struct AuthSnapshot(RwLock<AuthStatus>);

impl AuthSnapshot {
    pub fn new(initial: AuthStatus) -> Self {
        Self(RwLock::new(initial))
    }

    pub async fn store(&self, status: AuthStatus) {
        *self.0.write().await = status;
    }

    pub async fn status(&self) -> AuthStatus {
        *self.0.read().await
    }
}

#[cfg(test)]
mod tests {
    use super::{AuthOutcome, AuthStatus, outcome_for_request, outcome_for_status};
    use crate::error::ContactsError;

    #[test]
    fn decided_statuses_need_no_prompt() {
        assert_eq!(
            outcome_for_status(AuthStatus::Authorized),
            Some(AuthOutcome::AlreadyGranted)
        );
        assert_eq!(
            outcome_for_status(AuthStatus::Limited),
            Some(AuthOutcome::AlreadyGranted)
        );
        assert_eq!(
            outcome_for_status(AuthStatus::Denied),
            Some(AuthOutcome::Denied)
        );
        assert_eq!(
            outcome_for_status(AuthStatus::Restricted),
            Some(AuthOutcome::Restricted)
        );
        assert_eq!(
            outcome_for_status(AuthStatus::Unavailable),
            Some(AuthOutcome::Unavailable)
        );
        assert_eq!(outcome_for_status(AuthStatus::NotDetermined), None);
    }

    #[test]
    fn a_prompt_result_keeps_denial_and_timeout_apart() {
        assert_eq!(outcome_for_request(Ok(())), AuthOutcome::Granted);
        assert_eq!(
            outcome_for_request(Err(ContactsError::AccessDenied)),
            AuthOutcome::Denied
        );
        assert_eq!(
            outcome_for_request(Err(ContactsError::Timeout)),
            AuthOutcome::TimedOut
        );
        assert_eq!(
            outcome_for_request(Err(ContactsError::UnsupportedPlatform)),
            AuthOutcome::Unavailable
        );
        assert_eq!(
            outcome_for_request(Err(ContactsError::Framework("boom".into()))),
            AuthOutcome::Failed
        );
    }

    #[test]
    fn only_granted_outcomes_report_access() {
        assert!(AuthOutcome::Granted.is_granted());
        assert!(AuthOutcome::AlreadyGranted.is_granted());
        for outcome in [
            AuthOutcome::Denied,
            AuthOutcome::Restricted,
            AuthOutcome::TimedOut,
            AuthOutcome::Unavailable,
            AuthOutcome::Failed,
        ] {
            assert!(!outcome.is_granted());
        }
    }
}
