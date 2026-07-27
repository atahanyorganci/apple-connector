use std::time::Duration;

use sqlx::SqlitePool;

use crate::{
    api::{
        dto::{
            calendar::EventDetailDto, calendar_convert::event_detail_to_dto,
            reminder::ReminderDetailDto, reminder_convert::reminder_detail_to_dto,
        },
        error::ApiError,
    },
    calendar::CalendarRepository,
    reminders::ReminderRepository,
};

const HYDRATE_ATTEMPTS: usize = 5;
const HYDRATE_DELAY: Duration = Duration::from_millis(100);

#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct SyncPendingReminderDetailDto {
    #[serde(flatten)]
    pub detail: ReminderDetailDto,
    pub sync_pending: bool,
}

#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct SyncPendingEventDetailDto {
    #[serde(flatten)]
    pub detail: EventDetailDto,
    pub sync_pending: bool,
}

pub async fn hydrate_reminder(
    pool: &SqlitePool,
    external_id: &str,
) -> Result<SyncPendingReminderDetailDto, ApiError> {
    for attempt in 0..HYDRATE_ATTEMPTS {
        if let Some(reminder) = ReminderRepository::new(pool)
            .get_reminder(external_id)
            .await
            .map_err(|error| ApiError::internal(error.to_string()))?
        {
            return Ok(SyncPendingReminderDetailDto {
                detail: reminder_detail_to_dto(&reminder),
                sync_pending: false,
            });
        }
        if attempt + 1 < HYDRATE_ATTEMPTS {
            tokio::time::sleep(HYDRATE_DELAY).await;
        }
    }

    Ok(SyncPendingReminderDetailDto {
        detail: fallback_reminder_detail(external_id),
        sync_pending: true,
    })
}

pub async fn hydrate_event(
    pool: &SqlitePool,
    external_id: &str,
) -> Result<SyncPendingEventDetailDto, ApiError> {
    for attempt in 0..HYDRATE_ATTEMPTS {
        if let Some(event) = CalendarRepository::new(pool)
            .get_event(external_id)
            .await
            .map_err(|error| ApiError::internal(error.to_string()))?
        {
            return Ok(SyncPendingEventDetailDto {
                detail: event_detail_to_dto(&event),
                sync_pending: false,
            });
        }
        if attempt + 1 < HYDRATE_ATTEMPTS {
            tokio::time::sleep(HYDRATE_DELAY).await;
        }
    }

    Ok(SyncPendingEventDetailDto {
        detail: fallback_event_detail(external_id),
        sync_pending: true,
    })
}

fn fallback_reminder_detail(external_id: &str) -> ReminderDetailDto {
    use crate::{
        api::dto::reminder::ReminderDetailDto,
        apple_types::{ReminderId, ReminderListId},
    };

    ReminderDetailDto {
        id: ReminderId::new(external_id.to_owned()),
        row_id: 0,
        title: String::new(),
        notes: None,
        completed: false,
        flagged: false,
        priority: 0,
        list_id: ReminderListId::new(String::new()),
        list_row_id: 0,
        list_name: String::new(),
        parent_id: None,
        section_id: None,
        due: None,
        completion_at: None,
        created_at: None,
        last_modified_at: None,
        subtasks: Vec::new(),
        tags: Vec::new(),
        alarms: Vec::new(),
        recurrence: None,
        attachments: Vec::new(),
    }
}

fn fallback_event_detail(external_id: &str) -> EventDetailDto {
    use crate::{
        api::dto::calendar::{EventClassDto, EventStatusDto, EventSummaryDto},
        apple_types::{CalendarId, EventId},
    };

    EventDetailDto {
        summary: EventSummaryDto {
            id: EventId::new(external_id.to_owned()),
            row_id: 0,
            calendar_id: CalendarId::new(String::new()),
            calendar_row_id: 0,
            summary: None,
            start: None,
            end: None,
            all_day: false,
            status: EventStatusDto::Confirmed,
            hidden: false,
            is_recurring: false,
            occurrence_start: None,
            occurrence_end: None,
            event_class: EventClassDto::Standard,
        },
        description: None,
        url: None,
        location: None,
        organizer: None,
        attendees: Vec::new(),
        recurrence: None,
        exception_dates: Vec::new(),
        alarms: Vec::new(),
        attachments: Vec::new(),
        conference_url: None,
        travel_time_seconds: None,
        invitation_status: crate::api::dto::calendar::InvitationStatusDto::Unknown,
        availability: crate::api::dto::calendar::AvailabilityDto::Busy,
        privacy_level: crate::api::dto::calendar::PrivacyLevelDto::Default,
        series_id: None,
        series_row_id: None,
        original_start: None,
        last_modified: None,
        creation_date: None,
        has_structured_data: false,
        has_app_link: false,
    }
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
