use std::time::Duration;

use sqlx::SqlitePool;

use crate::{
    api::{
        dto::{
            calendar::EventDetailDto, calendar_convert::event_detail_to_dto,
            contacts::ContactDetailDto, contacts_convert::contact_detail_to_dto,
            reminder::ReminderDetailDto, reminder_convert::reminder_detail_to_dto,
        },
        error::ApiError,
    },
    apple_types::{ContactId, EventId, ReminderId},
    calendar::CalendarRepository,
    contacts::ContactsSources,
    reminders::ReminderRepository,
};

const HYDRATE_ATTEMPTS: usize = 5;
const HYDRATE_DELAY: Duration = Duration::from_millis(100);

#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct SyncPendingReminderDetailDto {
    pub id: ReminderId,
    pub sync_pending: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<ReminderDetailDto>,
}

#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct SyncPendingEventDetailDto {
    pub id: EventId,
    pub sync_pending: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<EventDetailDto>,
}

#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct SyncPendingContactDetailDto {
    pub id: ContactId,
    pub sync_pending: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<ContactDetailDto>,
}

pub async fn hydrate_reminder(
    pool: &SqlitePool,
    entity_ids: std::sync::Arc<crate::reminders::entities::EntityIds>,
    external_id: &str,
) -> Result<SyncPendingReminderDetailDto, ApiError> {
    let id = ReminderId::new(external_id.to_owned());
    for attempt in 0..HYDRATE_ATTEMPTS {
        if let Some(reminder) =
            ReminderRepository::with_entity_ids(pool, std::sync::Arc::clone(&entity_ids))
                .get_reminder(external_id)
                .await
                .map_err(|error| ApiError::internal(error.to_string()))?
        {
            return Ok(SyncPendingReminderDetailDto {
                id: reminder.id.clone(),
                detail: Some(reminder_detail_to_dto(&reminder)),
                sync_pending: false,
            });
        }
        if attempt + 1 < HYDRATE_ATTEMPTS {
            tokio::time::sleep(HYDRATE_DELAY).await;
        }
    }

    Ok(SyncPendingReminderDetailDto {
        id,
        detail: None,
        sync_pending: true,
    })
}

pub async fn hydrate_event(
    pool: &SqlitePool,
    external_id: &str,
) -> Result<SyncPendingEventDetailDto, ApiError> {
    let id = EventId::new(external_id.to_owned());
    for attempt in 0..HYDRATE_ATTEMPTS {
        if let Some(event) = CalendarRepository::new(pool)
            .get_event(external_id)
            .await
            .map_err(|error| ApiError::internal(error.to_string()))?
        {
            return Ok(SyncPendingEventDetailDto {
                id: event.summary.id.clone(),
                detail: Some(event_detail_to_dto(&event)),
                sync_pending: false,
            });
        }
        if attempt + 1 < HYDRATE_ATTEMPTS {
            tokio::time::sleep(HYDRATE_DELAY).await;
        }
    }

    Ok(SyncPendingEventDetailDto {
        id,
        detail: None,
        sync_pending: true,
    })
}

pub async fn hydrate_contact(
    sources: &ContactsSources,
    external_id: &str,
) -> Result<SyncPendingContactDetailDto, ApiError> {
    let api_id = crate::contacts::api_id_from_unique_id(external_id);
    let id = ContactId::new(api_id.clone());
    for attempt in 0..HYDRATE_ATTEMPTS {
        if let Some(contact) = sources
            .get_contact(&api_id)
            .await
            .map_err(|error| ApiError::internal(error.to_string()))?
        {
            return Ok(SyncPendingContactDetailDto {
                id: contact.id.clone(),
                detail: Some(contact_detail_to_dto(&contact)),
                sync_pending: false,
            });
        }
        if attempt + 1 < HYDRATE_ATTEMPTS {
            tokio::time::sleep(HYDRATE_DELAY).await;
        }
    }

    Ok(SyncPendingContactDetailDto {
        id,
        detail: None,
        sync_pending: true,
    })
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{HYDRATE_ATTEMPTS, HYDRATE_DELAY};

    #[test]
    fn hydrate_retry_defaults_are_documented() {
        assert_eq!(HYDRATE_ATTEMPTS, 5);
        assert_eq!(HYDRATE_DELAY, Duration::from_millis(100));
    }
}
