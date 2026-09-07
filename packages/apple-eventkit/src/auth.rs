use std::{sync::mpsc, time::Duration};

use objc2_event_kit::{EKAuthorizationStatus, EKEntityType, EKEventStore};
use tokio::sync::RwLock;

use crate::error::{EventKitError, EventKitResult};

/// How long one system prompt may stay unanswered.
const PROMPT_TIMEOUT: Duration = Duration::from_secs(120);

/// Budget for a worker job that may show both prompts back to back, with slack.
pub(crate) const PROMPT_JOB_BUDGET: Duration = Duration::from_secs(400);

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

/// What one access request actually did.
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
    /// The entity is not available on this system.
    Unavailable,
    /// EventKit refused the request for some other reason.
    Failed,
}

impl AuthOutcome {
    pub fn is_granted(self) -> bool {
        matches!(self, Self::AlreadyGranted | Self::Granted)
    }
}

/// Outcome of requesting access to both entities.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccessRequestOutcome {
    pub reminders: AuthOutcome,
    pub events: AuthOutcome,
}

/// Outcome implied by the current status, or `None` when a prompt is still needed.
fn outcome_for_status(status: AuthStatus) -> Option<AuthOutcome> {
    match status {
        AuthStatus::Authorized | AuthStatus::WriteOnly => Some(AuthOutcome::AlreadyGranted),
        AuthStatus::Denied => Some(AuthOutcome::Denied),
        AuthStatus::Restricted => Some(AuthOutcome::Restricted),
        AuthStatus::Unavailable => Some(AuthOutcome::Unavailable),
        AuthStatus::NotDetermined => None,
    }
}

/// Outcome of a prompt that has run to completion.
fn outcome_for_request(result: EventKitResult<()>) -> AuthOutcome {
    match result {
        Ok(()) => AuthOutcome::Granted,
        Err(EventKitError::AccessDenied) => AuthOutcome::Denied,
        Err(EventKitError::Timeout) => AuthOutcome::TimedOut,
        Err(EventKitError::UnsupportedPlatform) => AuthOutcome::Unavailable,
        Err(_) => AuthOutcome::Failed,
    }
}

/// Requests whatever access is still undecided and reports what happened to each entity.
///
/// Both entities are always resolved, so a refusal for one cannot hide the other's result.
pub(crate) fn request_pending_access(store: &EKEventStore) -> EventKitResult<AccessRequestOutcome> {
    let status = current_auth_status();
    let reminders = outcome_for_status(status.reminders)
        .unwrap_or_else(|| outcome_for_request(request_reminders_access(store)));
    let events = outcome_for_status(status.events)
        .unwrap_or_else(|| outcome_for_request(request_events_access(store)));
    Ok(AccessRequestOutcome { reminders, events })
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
    let (tx, rx) = mpsc::sync_channel(1);
    // `SyncSender` needs no lock: the channel has room for exactly one value, so a second
    // completion callback is discarded by `try_send` instead of being lost behind a poisoned
    // mutex — which previously stalled the caller for the whole authorization timeout.
    let block = block2::RcBlock::new(move |granted: objc2::runtime::Bool, _| {
        let _ = tx.try_send(granted.as_bool());
    });
    unsafe {
        store.requestFullAccessToRemindersWithCompletion(block2::RcBlock::as_ptr(&block));
    }
    wait_for_auth(rx)
}

fn request_events_access(store: &EKEventStore) -> EventKitResult<()> {
    let (tx, rx) = mpsc::sync_channel(1);
    // `SyncSender` needs no lock: the channel has room for exactly one value, so a second
    // completion callback is discarded by `try_send` instead of being lost behind a poisoned
    // mutex — which previously stalled the caller for the whole authorization timeout.
    let block = block2::RcBlock::new(move |granted: objc2::runtime::Bool, _| {
        let _ = tx.try_send(granted.as_bool());
    });
    unsafe {
        store.requestFullAccessToEventsWithCompletion(block2::RcBlock::as_ptr(&block));
    }
    wait_for_auth(rx)
}

fn wait_for_auth(rx: mpsc::Receiver<bool>) -> EventKitResult<()> {
    // EventKit delivers the completion handler on its own queue, so the worker thread can block
    // here. It previously polled while pumping `NSRunLoop::currentRunLoop`, which returns
    // immediately on a thread with no input sources — a full-CPU spin for as long as the prompt
    // stayed on screen.
    match rx.recv_timeout(PROMPT_TIMEOUT) {
        Ok(true) => Ok(()),
        Ok(false) => Err(EventKitError::AccessDenied),
        Err(mpsc::RecvTimeoutError::Timeout) => Err(EventKitError::Timeout),
        Err(mpsc::RecvTimeoutError::Disconnected) => Err(EventKitError::Framework(
            "EventKit auth callback dropped".into(),
        )),
    }
}

/// Last observed authorization status.
///
/// The snapshot is only ever written with a value read on the worker thread, so it cannot drift
/// from what the framework reports.
pub(crate) struct AuthSnapshot(RwLock<EntityAuthStatus>);

impl AuthSnapshot {
    pub fn new(initial: EntityAuthStatus) -> Self {
        Self(RwLock::new(initial))
    }

    pub async fn store(&self, status: EntityAuthStatus) {
        *self.0.write().await = status;
    }

    pub async fn status(&self) -> EntityAuthStatus {
        *self.0.read().await
    }
}

#[cfg(test)]
mod tests {
    use super::{AuthOutcome, AuthStatus, outcome_for_request, outcome_for_status};
    use crate::error::EventKitError;

    #[test]
    fn decided_statuses_need_no_prompt() {
        assert_eq!(
            outcome_for_status(AuthStatus::Authorized),
            Some(AuthOutcome::AlreadyGranted)
        );
        assert_eq!(
            outcome_for_status(AuthStatus::WriteOnly),
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
    }

    #[test]
    fn undecided_status_asks_for_a_prompt() {
        assert_eq!(outcome_for_status(AuthStatus::NotDetermined), None);
    }

    #[test]
    fn a_prompt_result_keeps_denial_and_timeout_apart() {
        assert_eq!(outcome_for_request(Ok(())), AuthOutcome::Granted);
        assert_eq!(
            outcome_for_request(Err(EventKitError::AccessDenied)),
            AuthOutcome::Denied
        );
        assert_eq!(
            outcome_for_request(Err(EventKitError::Timeout)),
            AuthOutcome::TimedOut
        );
        assert_eq!(
            outcome_for_request(Err(EventKitError::UnsupportedPlatform)),
            AuthOutcome::Unavailable
        );
        assert_eq!(
            outcome_for_request(Err(EventKitError::Framework("boom".into()))),
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
