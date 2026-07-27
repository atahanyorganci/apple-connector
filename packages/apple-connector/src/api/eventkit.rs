use std::sync::Arc;

use apple_eventkit::{AuthStatus, EventKitStore};

use crate::api::{
    dto::common::EventKitAuthStatusDto, error::ApiError, eventkit_convert::map_eventkit_error,
    router::AppState,
};

pub(crate) fn require_eventkit(
    store: &Option<Arc<EventKitStore>>,
) -> Result<Arc<EventKitStore>, ApiError> {
    store.clone().ok_or_else(ApiError::eventkit_unavailable)
}

pub(crate) async fn require_eventkit_reminders(
    state: &AppState,
) -> Result<Arc<EventKitStore>, ApiError> {
    let store = require_eventkit(&state.eventkit)?;
    match store.auth_status().await.reminders {
        AuthStatus::Authorized | AuthStatus::WriteOnly => Ok(store),
        AuthStatus::Denied | AuthStatus::Restricted => Err(ApiError::forbidden(
            "Reminders access denied; grant Reminders permission in System Settings",
        )),
        AuthStatus::NotDetermined => {
            store
                .ensure_reminders_access()
                .await
                .map_err(map_eventkit_error)?;
            match store.auth_status().await.reminders {
                AuthStatus::Authorized | AuthStatus::WriteOnly => Ok(store),
                AuthStatus::Denied | AuthStatus::Restricted => Err(ApiError::forbidden(
                    "Reminders access denied; grant Reminders permission in System Settings",
                )),
                AuthStatus::NotDetermined => Err(ApiError::service_unavailable(
                    "Reminders access not granted",
                )),
                AuthStatus::Unavailable => Err(ApiError::eventkit_unavailable()),
            }
        }
        AuthStatus::Unavailable => Err(ApiError::eventkit_unavailable()),
    }
}

pub(crate) async fn require_eventkit_events(
    state: &AppState,
) -> Result<Arc<EventKitStore>, ApiError> {
    let store = require_eventkit(&state.eventkit)?;
    match store.auth_status().await.events {
        AuthStatus::Authorized | AuthStatus::WriteOnly => Ok(store),
        AuthStatus::Denied | AuthStatus::Restricted => Err(ApiError::forbidden(
            "Calendar access denied; grant Calendars permission in System Settings",
        )),
        AuthStatus::NotDetermined => {
            store
                .ensure_events_access()
                .await
                .map_err(map_eventkit_error)?;
            match store.auth_status().await.events {
                AuthStatus::Authorized | AuthStatus::WriteOnly => Ok(store),
                AuthStatus::Denied | AuthStatus::Restricted => Err(ApiError::forbidden(
                    "Calendar access denied; grant Calendars permission in System Settings",
                )),
                AuthStatus::NotDetermined => {
                    Err(ApiError::service_unavailable("Calendar access not granted"))
                }
                AuthStatus::Unavailable => Err(ApiError::eventkit_unavailable()),
            }
        }
        AuthStatus::Unavailable => Err(ApiError::eventkit_unavailable()),
    }
}

pub(crate) async fn eventkit_reminders_status(
    store: &Option<Arc<EventKitStore>>,
) -> EventKitAuthStatusDto {
    match store {
        Some(store) => auth_status_to_dto(store.auth_status().await.reminders),
        None => EventKitAuthStatusDto::Unavailable,
    }
}

pub(crate) async fn eventkit_events_status(
    store: &Option<Arc<EventKitStore>>,
) -> EventKitAuthStatusDto {
    match store {
        Some(store) => auth_status_to_dto(store.auth_status().await.events),
        None => EventKitAuthStatusDto::Unavailable,
    }
}

fn auth_status_to_dto(status: AuthStatus) -> EventKitAuthStatusDto {
    match status {
        AuthStatus::NotDetermined => EventKitAuthStatusDto::NotDetermined,
        AuthStatus::Restricted => EventKitAuthStatusDto::Restricted,
        AuthStatus::Denied => EventKitAuthStatusDto::Denied,
        AuthStatus::Authorized => EventKitAuthStatusDto::Authorized,
        AuthStatus::WriteOnly => EventKitAuthStatusDto::WriteOnly,
        AuthStatus::Unavailable => EventKitAuthStatusDto::Unavailable,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_denied_auth_status() {
        assert_eq!(
            auth_status_to_dto(AuthStatus::Denied),
            EventKitAuthStatusDto::Denied
        );
    }
}
