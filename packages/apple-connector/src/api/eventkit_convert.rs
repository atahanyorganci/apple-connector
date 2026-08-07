use apple_eventkit::{
    AlarmInput, AlarmKind, CalendarResolveHint, CalendarStoreType, CreateEventInput,
    CreateReminderInput, DeleteEventInput, DueInput, EventKitError, EventSpan, EventStatusInput,
    LocationInput, RecurrenceFrequency, RecurrenceInput, ReminderListResolveHint, UpdateEventInput,
    UpdateReminderInput,
};

use crate::{
    api::{
        dto::{
            calendar::{CreateEventRequest, EventSpanDto, EventStatusInputDto, UpdateEventRequest},
            reminder::{
                AlarmInputDto, AlarmKindDto, CreateReminderRequest, DueInputDto, LocationInputDto,
                RecurrenceFrequencyDto, RecurrenceInputDto, UpdateReminderRequest,
            },
        },
        error::ApiError,
    },
    apple_types::ReminderPriority,
    calendar::CalendarResolveMetadata,
    reminders::ReminderListResolveMetadata,
};

pub fn map_eventkit_error(error: EventKitError) -> ApiError {
    match error {
        EventKitError::NotFound => ApiError::not_found("EventKit item not found"),
        EventKitError::AccessDenied => ApiError::forbidden("EventKit access denied"),
        EventKitError::ReadOnlyCalendar => {
            ApiError::forbidden("target calendar or list is read-only")
        }
        EventKitError::ValidationFailed(message) => ApiError::unprocessable(message),
        EventKitError::AmbiguousMatch(message) => ApiError::conflict(message),
        EventKitError::UnsupportedPlatform => ApiError::eventkit_unavailable(),
        EventKitError::Framework(message) => ApiError::internal(message),
        EventKitError::Timeout => ApiError::internal("EventKit operation timed out"),
    }
}

pub fn validate_create_reminder(request: &CreateReminderRequest) -> Result<(), ApiError> {
    validate_reminder_priority(request.priority)?;
    reject_unsupported_reminder_fields(
        request.section_id.is_some(),
        request.parent_id.is_some(),
        !request.tags.is_empty(),
        !request.attachments.is_empty(),
        request.flagged.is_some(),
    )
}

pub fn validate_update_reminder(request: &UpdateReminderRequest) -> Result<(), ApiError> {
    validate_reminder_priority(request.priority)?;
    reject_unsupported_reminder_fields(
        request.section_id.is_some(),
        request.parent_id.is_some(),
        !request.tags.is_empty(),
        !request.attachments.is_empty(),
        request.flagged.is_some(),
    )
}

fn validate_reminder_priority(priority: Option<i64>) -> Result<(), ApiError> {
    if let Some(value) = priority {
        ReminderPriority::try_new(value)
            .map_err(|error| ApiError::unprocessable(error.to_string()))?;
    }
    Ok(())
}

fn reject_unsupported_reminder_fields(
    section: bool,
    parent: bool,
    tags: bool,
    attachments: bool,
    flagged: bool,
) -> Result<(), ApiError> {
    if section {
        return Err(ApiError::unprocessable_with_details(
            "unsupported reminder field",
            serde_json::json!({ "field": "section_id" }),
        ));
    }
    if parent {
        return Err(ApiError::unprocessable_with_details(
            "unsupported reminder field",
            serde_json::json!({ "field": "parent_id" }),
        ));
    }
    if tags {
        return Err(ApiError::unprocessable_with_details(
            "unsupported reminder field",
            serde_json::json!({ "field": "tags" }),
        ));
    }
    if attachments {
        return Err(ApiError::unprocessable_with_details(
            "unsupported reminder field",
            serde_json::json!({ "field": "attachments" }),
        ));
    }
    if flagged {
        return Err(ApiError::unprocessable_with_details(
            "unsupported reminder field",
            serde_json::json!({ "field": "flagged" }),
        ));
    }
    Ok(())
}

pub fn reminder_list_hint(metadata: ReminderListResolveMetadata) -> ReminderListResolveHint {
    ReminderListResolveHint {
        api_id: metadata.api_id,
        external_id: metadata.external_id,
        title: metadata.title,
        is_smart_list: metadata.is_smart_list,
    }
}

pub fn calendar_hint(metadata: CalendarResolveMetadata) -> CalendarResolveHint {
    CalendarResolveHint {
        api_id: metadata.api_id,
        external_id: metadata.external_id,
        title: metadata.title,
        store_type: match metadata.store_type {
            1 => CalendarStoreType::CalDav,
            2 => CalendarStoreType::Exchange,
            3 => CalendarStoreType::Subscription,
            4 => CalendarStoreType::Birthday,
            _ => CalendarStoreType::Local,
        },
    }
}

pub fn create_reminder_input(request: CreateReminderRequest) -> CreateReminderInput {
    CreateReminderInput {
        title: request.title,
        notes: request.notes,
        due: request.due.map(due_input),
        completed: request.completed,
        priority: request.priority,
        url: request.url,
        location: request.location.map(location_input),
        alarms: request.alarms.into_iter().map(alarm_input).collect(),
        recurrence: request.recurrence.map(recurrence_input),
    }
}

pub fn update_reminder_input(
    request: UpdateReminderRequest,
    list_hint: Option<ReminderListResolveHint>,
) -> UpdateReminderInput {
    UpdateReminderInput {
        title: request.title,
        notes: request.notes,
        due: request.due.map(|due| due.map(due_input)),
        completed: request.completed,
        priority: request.priority,
        url: request.url,
        list_hint,
        location: request
            .location
            .map(|location| location.map(location_input)),
        alarms: request
            .alarms
            .map(|alarms| alarms.into_iter().map(alarm_input).collect()),
        recurrence: request
            .recurrence
            .map(|recurrence| recurrence.map(recurrence_input)),
    }
}

pub fn create_event_input(request: CreateEventRequest) -> CreateEventInput {
    CreateEventInput {
        summary: request.summary,
        description: request.description,
        start: request.start.seconds(),
        end: request.end.seconds(),
        all_day: request.all_day,
        url: request.url,
        status: request.status.map(event_status_input),
        location: request.location.map(location_input),
        alarms: request.alarms.into_iter().map(alarm_input).collect(),
        recurrence: request.recurrence.map(recurrence_input),
    }
}

pub fn update_event_input(
    request: UpdateEventRequest,
    calendar_hint: Option<CalendarResolveHint>,
    span: EventSpanDto,
) -> UpdateEventInput {
    UpdateEventInput {
        summary: request.summary,
        description: request.description,
        start: request.start.map(|value| value.seconds()),
        end: request.end.map(|value| value.seconds()),
        all_day: request.all_day,
        url: request.url,
        status: request.status.map(event_status_input),
        calendar_hint,
        location: request
            .location
            .map(|location| location.map(location_input)),
        alarms: request
            .alarms
            .map(|alarms| alarms.into_iter().map(alarm_input).collect()),
        recurrence: request
            .recurrence
            .map(|recurrence| recurrence.map(recurrence_input)),
        span: span.into(),
    }
}

pub fn delete_event_input(span: EventSpanDto, occurrence_start: Option<i64>) -> DeleteEventInput {
    DeleteEventInput {
        span: span.into(),
        occurrence_start,
    }
}

impl From<EventSpanDto> for EventSpan {
    fn from(value: EventSpanDto) -> Self {
        match value {
            EventSpanDto::This => Self::This,
            EventSpanDto::Future => Self::Future,
            EventSpanDto::All => Self::All,
        }
    }
}

fn due_input(due: DueInputDto) -> DueInput {
    DueInput {
        at: due.at.seconds(),
        all_day: due.all_day,
    }
}

fn location_input(location: LocationInputDto) -> LocationInput {
    LocationInput {
        title: location.title,
        latitude: location.latitude,
        longitude: location.longitude,
    }
}

fn alarm_input(alarm: AlarmInputDto) -> AlarmInput {
    AlarmInput {
        kind: match alarm.kind {
            AlarmKindDto::Absolute => AlarmKind::Absolute,
            AlarmKindDto::Relative => AlarmKind::Relative,
            AlarmKindDto::Location | AlarmKindDto::Unknown => AlarmKind::Relative,
        },
        at: alarm.at.map(|value| value.seconds()),
        offset_seconds: alarm.offset_seconds,
    }
}

fn recurrence_input(recurrence: RecurrenceInputDto) -> RecurrenceInput {
    RecurrenceInput {
        frequency: match recurrence.frequency {
            RecurrenceFrequencyDto::Daily => RecurrenceFrequency::Daily,
            RecurrenceFrequencyDto::Weekly => RecurrenceFrequency::Weekly,
            RecurrenceFrequencyDto::Monthly => RecurrenceFrequency::Monthly,
            RecurrenceFrequencyDto::Yearly => RecurrenceFrequency::Yearly,
        },
        interval: recurrence.interval,
        count: recurrence.count,
        end_date: recurrence.end_date.map(|value| value.seconds()),
    }
}

fn event_status_input(status: EventStatusInputDto) -> EventStatusInput {
    match status {
        EventStatusInputDto::Confirmed => EventStatusInput::Confirmed,
        EventStatusInputDto::Tentative => EventStatusInput::Tentative,
        EventStatusInputDto::Cancelled => EventStatusInput::Cancelled,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        api::dto::reminder::{CreateReminderRequest, UpdateReminderRequest},
        apple_types::SectionId,
    };

    #[test]
    fn validate_create_reminder_rejects_unsupported_section_id()
    -> Result<(), Box<dyn std::error::Error>> {
        let request = CreateReminderRequest {
            title: "Test".into(),
            notes: None,
            due: None,
            completed: None,
            priority: None,
            url: None,
            location: None,
            alarms: Vec::new(),
            recurrence: None,
            section_id: Some(SectionId::new("00000000-0000-0000-0000-000000000001")),
            parent_id: None,
            tags: Vec::new(),
            attachments: Vec::new(),
            flagged: None,
        };
        let error = validate_create_reminder(&request)
            .err()
            .ok_or("expected create reminder validation error")?;
        assert_eq!(error.status(), axum::http::StatusCode::UNPROCESSABLE_ENTITY);
        Ok(())
    }

    #[test]
    fn validate_update_reminder_rejects_unsupported_tags() -> Result<(), Box<dyn std::error::Error>>
    {
        let request = UpdateReminderRequest {
            title: None,
            notes: None,
            due: None,
            completed: None,
            priority: None,
            url: None,
            list_id: None,
            location: None,
            alarms: None,
            recurrence: None,
            section_id: None,
            parent_id: None,
            tags: vec!["work".into()],
            attachments: Vec::new(),
            flagged: None,
        };
        let error = validate_update_reminder(&request)
            .err()
            .ok_or("expected update reminder validation error")?;
        assert_eq!(error.status(), axum::http::StatusCode::UNPROCESSABLE_ENTITY);
        Ok(())
    }

    #[test]
    fn map_eventkit_read_only_to_forbidden() {
        let error = map_eventkit_error(EventKitError::ReadOnlyCalendar);
        assert_eq!(error.status(), axum::http::StatusCode::FORBIDDEN);
    }

    #[test]
    fn map_eventkit_not_found() {
        let error = map_eventkit_error(EventKitError::NotFound);
        assert_eq!(error.status(), axum::http::StatusCode::NOT_FOUND);
    }
}
