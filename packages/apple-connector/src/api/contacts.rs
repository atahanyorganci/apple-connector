use std::sync::Arc;

use apple_contacts::ContactsStore;

use crate::{
    api::{dto::common::ContactsAuthStatusDto, error::ApiError, router::AppState},
    contacts::ContactsSources,
    db::is_pool_healthy,
};

pub(crate) fn require_contacts_sources(
    sources: &ContactsSources,
) -> Result<&ContactsSources, ApiError> {
    if sources.is_empty() {
        return Err(ApiError::new(
            crate::api::error::ErrorCode::ContactsDatabaseUnavailable,
        ));
    }
    Ok(sources)
}

pub(crate) fn require_contacts_store(
    store: &Option<Arc<ContactsStore>>,
) -> Result<Arc<ContactsStore>, ApiError> {
    store.clone().ok_or_else(ApiError::contacts_unavailable)
}

pub(crate) async fn require_contacts_access(
    state: &AppState,
) -> Result<Arc<ContactsStore>, ApiError> {
    let store = require_contacts_store(&state.contacts_store)?;
    match store.auth_status().await {
        apple_contacts::AuthStatus::Authorized | apple_contacts::AuthStatus::Limited => Ok(store),
        apple_contacts::AuthStatus::Denied | apple_contacts::AuthStatus::Restricted => {
            Err(ApiError::forbidden(
                "Contacts access denied; grant Contacts permission in System Settings",
            ))
        }
        apple_contacts::AuthStatus::NotDetermined => {
            store
                .ensure_contacts_access()
                .await
                .map_err(crate::api::contacts_convert::map_contacts_error)?;
            match store.auth_status().await {
                apple_contacts::AuthStatus::Authorized | apple_contacts::AuthStatus::Limited => {
                    Ok(store)
                }
                apple_contacts::AuthStatus::Denied | apple_contacts::AuthStatus::Restricted => {
                    Err(ApiError::forbidden(
                        "Contacts access denied; grant Contacts permission in System Settings",
                    ))
                }
                apple_contacts::AuthStatus::NotDetermined => {
                    Err(ApiError::service_unavailable("Contacts access not granted"))
                }
                apple_contacts::AuthStatus::Unavailable => Err(ApiError::contacts_unavailable()),
            }
        }
        apple_contacts::AuthStatus::Unavailable => Err(ApiError::contacts_unavailable()),
    }
}

pub(crate) async fn contacts_status(
    sources: &ContactsSources,
) -> crate::api::dto::common::HealthStatus {
    use crate::api::dto::common::HealthStatus;

    if sources.is_empty() {
        return HealthStatus::Unavailable;
    }

    for pool in sources.pools() {
        if !is_pool_healthy(pool).await {
            return HealthStatus::Unavailable;
        }
    }

    HealthStatus::Ok
}

pub(crate) async fn contacts_auth_status(
    store: &Option<Arc<ContactsStore>>,
) -> ContactsAuthStatusDto {
    match store {
        Some(store) => auth_status_to_dto(store.auth_status().await),
        None => ContactsAuthStatusDto::Unavailable,
    }
}

pub use super::contacts_convert::{
    contact_detail_carddav, contact_detail_vcard, contact_page_carddav, contact_page_vcard,
};

fn auth_status_to_dto(status: apple_contacts::AuthStatus) -> ContactsAuthStatusDto {
    match status {
        apple_contacts::AuthStatus::NotDetermined => ContactsAuthStatusDto::NotDetermined,
        apple_contacts::AuthStatus::Restricted => ContactsAuthStatusDto::Restricted,
        apple_contacts::AuthStatus::Denied => ContactsAuthStatusDto::Denied,
        apple_contacts::AuthStatus::Authorized => ContactsAuthStatusDto::Authorized,
        apple_contacts::AuthStatus::Limited => ContactsAuthStatusDto::Limited,
        apple_contacts::AuthStatus::Unavailable => ContactsAuthStatusDto::Unavailable,
    }
}
