use sqlx::SqlitePool;

use crate::{
    api::{
        dto::{
            calendar::EventDetailDto,
            calendar_convert::event_detail_to_dto,
            contacts::{ContactDetailDto, GroupDetailDto},
            contacts_convert::{contact_detail_to_dto, group_detail_to_dto},
            reminder::ReminderDetailDto,
            reminder_convert::reminder_detail_to_dto,
        },
        error::ApiError,
    },
    apple_types::{ContactId, EventId, GroupId, ReminderId},
    calendar::CalendarRepository,
    contacts::ContactsSources,
    reminders::ReminderRepository,
};

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

#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct SyncPendingGroupDetailDto {
    pub id: GroupId,
    pub sync_pending: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<GroupDetailDto>,
}

/// Return status for create/update mutations that may still be awaiting SQLite sync.
pub fn mutation_status(sync_pending: bool, created: bool) -> axum::http::StatusCode {
    if sync_pending {
        axum::http::StatusCode::ACCEPTED
    } else if created {
        axum::http::StatusCode::CREATED
    } else {
        axum::http::StatusCode::OK
    }
}

/// Single non-blocking SQLite read after a write. Never sleeps on the request path.
pub async fn hydrate_reminder(
    pool: &SqlitePool,
    entity_ids: std::sync::Arc<crate::reminders::entities::EntityIds>,
    external_id: &str,
) -> Result<SyncPendingReminderDetailDto, ApiError> {
    let id = ReminderId::new(external_id.to_owned());
    if let Some(reminder) = ReminderRepository::with_entity_ids(pool, entity_ids)
        .get_reminder(external_id)
        .await
        .map_err(ApiError::from_sqlx)?
    {
        return Ok(SyncPendingReminderDetailDto {
            id: reminder.id.clone(),
            detail: Some(reminder_detail_to_dto(&reminder)),
            sync_pending: false,
        });
    }

    Ok(SyncPendingReminderDetailDto {
        id,
        detail: None,
        sync_pending: true,
    })
}

/// Single non-blocking SQLite read after a write. Never sleeps on the request path.
pub async fn hydrate_event(
    pool: &SqlitePool,
    external_id: &str,
) -> Result<SyncPendingEventDetailDto, ApiError> {
    let id = EventId::new(external_id.to_owned());
    if let Some(event) = CalendarRepository::new(pool)
        .get_event(external_id)
        .await
        .map_err(ApiError::from_sqlx)?
    {
        return Ok(SyncPendingEventDetailDto {
            id: event.summary.id.clone(),
            detail: Some(event_detail_to_dto(&event)),
            sync_pending: false,
        });
    }

    Ok(SyncPendingEventDetailDto {
        id,
        detail: None,
        sync_pending: true,
    })
}

/// Single non-blocking SQLite read after a write. Never sleeps on the request path.
pub async fn hydrate_contact(
    sources: &ContactsSources,
    external_id: &str,
) -> Result<SyncPendingContactDetailDto, ApiError> {
    let api_id = crate::contacts::api_id_from_unique_id(external_id);
    let id = ContactId::new(api_id.clone());
    if let Some(contact) = sources
        .get_contact(&api_id)
        .await
        .map_err(ApiError::from_sqlx)?
    {
        return Ok(SyncPendingContactDetailDto {
            id: contact.id.clone(),
            detail: Some(contact_detail_to_dto(&contact)),
            sync_pending: false,
        });
    }

    Ok(SyncPendingContactDetailDto {
        id,
        detail: None,
        sync_pending: true,
    })
}

/// Single non-blocking SQLite read after a write. Never sleeps on the request path.
pub async fn hydrate_group(
    sources: &ContactsSources,
    external_id: &str,
) -> Result<SyncPendingGroupDetailDto, ApiError> {
    let api_id = crate::contacts::api_id_from_unique_id(external_id);
    let id = GroupId::new(api_id.clone());
    if let Some(group) = sources
        .get_group(&api_id)
        .await
        .map_err(ApiError::from_sqlx)?
    {
        return Ok(SyncPendingGroupDetailDto {
            id: group.id.clone(),
            detail: Some(group_detail_to_dto(&group)),
            sync_pending: false,
        });
    }

    Ok(SyncPendingGroupDetailDto {
        id,
        detail: None,
        sync_pending: true,
    })
}

#[cfg(test)]
mod tests {
    use axum::http::StatusCode;

    use super::mutation_status;

    #[test]
    fn mutation_status_returns_accepted_when_sync_pending() {
        assert_eq!(
            mutation_status(true, true),
            StatusCode::ACCEPTED,
            "create pending"
        );
        assert_eq!(
            mutation_status(true, false),
            StatusCode::ACCEPTED,
            "update pending"
        );
        assert_eq!(
            mutation_status(false, true),
            StatusCode::CREATED,
            "create hydrated"
        );
        assert_eq!(
            mutation_status(false, false),
            StatusCode::OK,
            "update hydrated"
        );
    }
}
