use apple_eventkit::{
    AlarmInput, AlarmKind, CalendarResolveHint, CalendarStoreType, CreateEventInput,
    CreateReminderInput, DeleteEventInput, DueInput, EventKitError, EventSpan, LocationInput,
    RecurrenceFrequency, RecurrenceInput, ReminderListResolveHint, ReminderLocationInput,
    UpdateEventInput, UpdateReminderInput,
};

use crate::{
    api::{
        dto::{
            calendar::{CreateEventRequest, EventSpanDto, UpdateEventRequest},
            reminder::{
                AlarmInputDto, AlarmKindDto, CreateReminderRequest, DueInputDto, LocationInputDto,
                RecurrenceFrequencyDto, RecurrenceInputDto, UpdateReminderRequest,
            },
        },
        error::{ApiError, ErrorCode},
    },
    apple_types::ReminderPriority,
    calendar::CalendarResolveMetadata,
    reminders::ReminderListResolveMetadata,
};

pub fn map_eventkit_error(error: EventKitError) -> ApiError {
    match error {
        EventKitError::NotFound => ApiError::new(ErrorCode::ResourceNotFound),
        EventKitError::AccessDenied => ApiError::new(ErrorCode::EventkitAccessDenied),
        EventKitError::ReadOnlyCalendar => ApiError::new(ErrorCode::CalendarReadOnly),
        EventKitError::ValidationFailed(message) => {
            ApiError::with_message(ErrorCode::UnprocessableEntity, message)
        }
        EventKitError::EndBeforeStart => ApiError::new(ErrorCode::EventEndBeforeStart),
        EventKitError::AmbiguousMatch(message) => {
            ApiError::with_message(ErrorCode::AmbiguousEventKitMatch, message)
        }
        EventKitError::UnsupportedPlatform => ApiError::eventkit_unavailable(),
        EventKitError::Framework(_message) => ApiError::new(ErrorCode::InternalError),
        EventKitError::Timeout => ApiError::new(ErrorCode::GatewayTimeout),
    }
}

/// Request-level checks for event creation, run before any store is consulted.
pub fn validate_create_event(request: &CreateEventRequest) -> Result<(), ApiError> {
    reject_event_status(request.status.is_some())?;
    if request.end.seconds() < request.start.seconds() {
        return Err(ApiError::new(ErrorCode::EventEndBeforeStart));
    }
    Ok(())
}

/// Request-level checks for an event update.
///
/// The date range is deliberately not checked here: a partial update only inverts against the
/// stored event, so that check belongs behind the framework boundary (see `merged_range`).
pub fn validate_update_event(request: &UpdateEventRequest) -> Result<(), ApiError> {
    reject_event_status(request.status.is_some())
}

pub fn validate_create_reminder(request: &CreateReminderRequest) -> Result<(), ApiError> {
    validate_reminder_priority(request.priority)?;
    reject_reminder_coordinates(request.location.as_ref())?;
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
    reject_reminder_coordinates(request.location.as_ref().and_then(Option::as_ref))?;
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
        ReminderPriority::try_new(value).map_err(|error| {
            ApiError::with_message(ErrorCode::UnprocessableEntity, error.to_string())
        })?;
    }
    Ok(())
}

/// EventKit cannot store a coordinate on a reminder: `structuredLocation` belongs to `EKEvent`
/// and `EKAlarm`, and `EKReminder` inherits only a plain location string.
///
/// Accepting the coordinate and writing just the text left the caller believing a geofence had
/// been saved, so the field is refused instead. A geofenced reminder needs a proximity alarm,
/// which this API does not yet expose.
fn reject_reminder_coordinates(location: Option<&LocationInputDto>) -> Result<(), ApiError> {
    let Some(location) = location else {
        return Ok(());
    };
    for (present, field) in [
        (location.latitude.is_some(), "location.latitude"),
        (location.longitude.is_some(), "location.longitude"),
    ] {
        if present {
            return Err(ApiError::with_details(
                ErrorCode::UnsupportedReminderField,
                "EventKit cannot store coordinates on a reminder; only the location text is kept",
                serde_json::json!({ "field": field }),
            ));
        }
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
        return Err(ApiError::with_details(
            ErrorCode::UnsupportedReminderField,
            "unsupported reminder field",
            serde_json::json!({ "field": "section_id" }),
        ));
    }
    if parent {
        return Err(ApiError::with_details(
            ErrorCode::UnsupportedReminderField,
            "unsupported reminder field",
            serde_json::json!({ "field": "parent_id" }),
        ));
    }
    if tags {
        return Err(ApiError::with_details(
            ErrorCode::UnsupportedReminderField,
            "unsupported reminder field",
            serde_json::json!({ "field": "tags" }),
        ));
    }
    if attachments {
        return Err(ApiError::with_details(
            ErrorCode::UnsupportedReminderField,
            "unsupported reminder field",
            serde_json::json!({ "field": "attachments" }),
        ));
    }
    if flagged {
        return Err(ApiError::with_details(
            ErrorCode::UnsupportedReminderField,
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

pub fn create_reminder_input(
    request: CreateReminderRequest,
) -> Result<CreateReminderInput, ApiError> {
    reject_reminder_coordinates(request.location.as_ref())?;
    Ok(CreateReminderInput {
        title: request.title,
        notes: request.notes,
        due: request.due.map(due_input),
        completed: request.completed,
        priority: request.priority,
        url: request.url,
        location: request.location.map(reminder_location_input),
        alarms: request
            .alarms
            .into_iter()
            .map(alarm_input)
            .collect::<Result<Vec<_>, _>>()?,
        recurrence: request.recurrence.map(recurrence_input),
    })
}

pub fn update_reminder_input(
    request: UpdateReminderRequest,
    list_hint: Option<ReminderListResolveHint>,
) -> Result<UpdateReminderInput, ApiError> {
    reject_reminder_coordinates(request.location.as_ref().and_then(Option::as_ref))?;
    Ok(UpdateReminderInput {
        title: request.title,
        notes: request.notes,
        due: request.due.map(|due| due.map(due_input)),
        completed: request.completed,
        priority: request.priority,
        url: request.url,
        list_hint,
        location: request
            .location
            .map(|location| location.map(reminder_location_input)),
        alarms: request
            .alarms
            .map(|alarms| {
                alarms
                    .into_iter()
                    .map(alarm_input)
                    .collect::<Result<Vec<_>, _>>()
            })
            .transpose()?,
        recurrence: request
            .recurrence
            .map(|recurrence| recurrence.map(recurrence_input)),
    })
}

pub fn create_event_input(request: CreateEventRequest) -> Result<CreateEventInput, ApiError> {
    reject_event_status(request.status.is_some())?;
    Ok(CreateEventInput {
        summary: request.summary,
        description: request.description,
        start: request.start.seconds(),
        end: request.end.seconds(),
        all_day: request.all_day,
        url: request.url,
        location: request.location.map(location_input),
        alarms: request
            .alarms
            .into_iter()
            .map(alarm_input)
            .collect::<Result<Vec<_>, _>>()?,
        recurrence: request.recurrence.map(recurrence_input),
    })
}

pub fn update_event_input(
    request: UpdateEventRequest,
    calendar_hint: Option<CalendarResolveHint>,
    span: EventSpanDto,
) -> Result<UpdateEventInput, ApiError> {
    reject_event_status(request.status.is_some())?;
    Ok(UpdateEventInput {
        summary: request.summary,
        description: request.description,
        start: request.start.map(|value| value.seconds()),
        end: request.end.map(|value| value.seconds()),
        all_day: request.all_day,
        url: request.url,
        calendar_hint,
        location: request
            .location
            .map(|location| location.map(location_input)),
        alarms: request
            .alarms
            .map(|alarms| {
                alarms
                    .into_iter()
                    .map(alarm_input)
                    .collect::<Result<Vec<_>, _>>()
            })
            .transpose()?,
        recurrence: request
            .recurrence
            .map(|recurrence| recurrence.map(recurrence_input)),
        span: span.into(),
    })
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
        }
    }
}

fn due_input(due: DueInputDto) -> DueInput {
    DueInput {
        at: due.at.seconds(),
        all_day: due.all_day,
    }
}

fn reminder_location_input(location: LocationInputDto) -> ReminderLocationInput {
    ReminderLocationInput {
        title: location.title,
    }
}

fn location_input(location: LocationInputDto) -> LocationInput {
    LocationInput {
        title: location.title,
        latitude: location.latitude,
        longitude: location.longitude,
    }
}

fn alarm_input(alarm: AlarmInputDto) -> Result<AlarmInput, ApiError> {
    let kind = match alarm.kind {
        AlarmKindDto::Absolute => AlarmKind::Absolute,
        AlarmKindDto::Relative => AlarmKind::Relative,
        AlarmKindDto::Location => {
            return Err(ApiError::with_details(
                ErrorCode::UnsupportedAlarmKind,
                "unsupported alarm kind",
                serde_json::json!({
                    "field": "alarms.kind",
                    "kind": "location",
                }),
            ));
        }
        AlarmKindDto::Unknown => {
            return Err(ApiError::with_details(
                ErrorCode::UnsupportedAlarmKind,
                "unsupported alarm kind",
                serde_json::json!({
                    "field": "alarms.kind",
                    "kind": "unknown",
                }),
            ));
        }
    };
    Ok(AlarmInput {
        kind,
        at: alarm.at.map(|value| value.seconds()),
        offset_seconds: alarm.offset_seconds,
    })
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

/// EventKit exposes `EKEvent.status` as read-only, so there is no supported status to apply.
///
/// Accepting the field and dropping it left clients believing a status had been set, which is why
/// this rejects rather than ignores. Cancelling an event means deleting it.
fn reject_event_status(present: bool) -> Result<(), ApiError> {
    if present {
        return Err(ApiError::with_details(
            ErrorCode::ImmutableEventField,
            "EventKit does not allow event status to be written; delete the event to cancel it",
            serde_json::json!({ "field": "status" }),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        api::dto::{
            calendar::EventStatusInputDto,
            reminder::{CreateReminderRequest, UpdateReminderRequest},
        },
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
    fn create_rejects_event_status_as_immutable() -> Result<(), Box<dyn std::error::Error>> {
        let request = CreateEventRequest {
            summary: "Test".into(),
            description: None,
            start: crate::apple_types::UnixTimestamp::from_seconds(1_700_000_000),
            end: crate::apple_types::UnixTimestamp::from_seconds(1_700_003_600),
            all_day: false,
            url: None,
            status: Some(EventStatusInputDto::Cancelled),
            location: None,
            alarms: Vec::new(),
            recurrence: None,
        };
        let error = create_event_input(request)
            .err()
            .ok_or("expected event status to be rejected")?;
        assert_eq!(error.status(), axum::http::StatusCode::UNPROCESSABLE_ENTITY);
        let body = serde_json::to_value(error.body())?;
        assert_eq!(body["code"], "immutable_event_field");
        assert_eq!(body["details"]["field"], "status");
        Ok(())
    }

    #[test]
    fn update_rejects_event_status_as_immutable() -> Result<(), Box<dyn std::error::Error>> {
        let request = UpdateEventRequest {
            status: Some(EventStatusInputDto::Confirmed),
            ..UpdateEventRequest::default()
        };
        let error = update_event_input(request, None, EventSpanDto::This)
            .err()
            .ok_or("expected event status to be rejected")?;
        assert_eq!(error.status(), axum::http::StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(
            serde_json::to_value(error.body())?["code"],
            "immutable_event_field"
        );
        Ok(())
    }

    #[test]
    fn an_event_without_status_still_converts() -> Result<(), Box<dyn std::error::Error>> {
        let input = update_event_input(UpdateEventRequest::default(), None, EventSpanDto::This)?;
        assert_eq!(input.span, EventSpan::This);
        Ok(())
    }

    fn reminder_location(latitude: Option<f64>, longitude: Option<f64>) -> LocationInputDto {
        LocationInputDto {
            title: Some("Office".into()),
            latitude,
            longitude,
        }
    }

    fn create_reminder_with_location(location: LocationInputDto) -> CreateReminderRequest {
        CreateReminderRequest {
            title: "Test".into(),
            notes: None,
            due: None,
            completed: None,
            priority: None,
            url: None,
            location: Some(location),
            alarms: Vec::new(),
            recurrence: None,
            section_id: None,
            parent_id: None,
            tags: Vec::new(),
            attachments: Vec::new(),
            flagged: None,
        }
    }

    #[test]
    fn a_text_only_reminder_location_is_accepted() -> Result<(), Box<dyn std::error::Error>> {
        let request = create_reminder_with_location(reminder_location(None, None));
        validate_create_reminder(&request)?;
        let input = create_reminder_input(request)?;
        assert_eq!(
            input.location.and_then(|location| location.title),
            Some("Office".into())
        );
        Ok(())
    }

    /// Latitude and longitude are rejected one at a time as well as together, so a half-specified
    /// coordinate cannot slip through.
    #[test]
    fn reminder_location_coordinates_are_rejected() -> Result<(), Box<dyn std::error::Error>> {
        for location in [
            reminder_location(Some(51.5), None),
            reminder_location(None, Some(-0.12)),
            reminder_location(Some(51.5), Some(-0.12)),
        ] {
            let request = create_reminder_with_location(location);
            let error = validate_create_reminder(&request)
                .err()
                .ok_or("expected reminder coordinates to be rejected")?;
            assert_eq!(error.status(), axum::http::StatusCode::UNPROCESSABLE_ENTITY);
            let body = serde_json::to_value(error.body())?;
            assert_eq!(body["code"], "unsupported_reminder_field");
            assert!(
                body["details"]["field"]
                    .as_str()
                    .is_some_and(|field| field.starts_with("location.")),
                "the rejected field should be named"
            );
            assert!(create_reminder_input(request).is_err());
        }
        Ok(())
    }

    #[test]
    fn update_reminder_rejects_coordinates() -> Result<(), Box<dyn std::error::Error>> {
        let request = UpdateReminderRequest {
            location: Some(Some(reminder_location(Some(51.5), Some(-0.12)))),
            ..UpdateReminderRequest::default()
        };
        assert!(validate_update_reminder(&request).is_err());
        assert!(update_reminder_input(request, None).is_err());
        Ok(())
    }

    /// Events keep their coordinates: `EKEvent` has `structuredLocation`, reminders do not.
    #[test]
    fn event_locations_keep_their_coordinates() -> Result<(), Box<dyn std::error::Error>> {
        let request = UpdateEventRequest {
            location: Some(Some(reminder_location(Some(51.5), Some(-0.12)))),
            ..UpdateEventRequest::default()
        };
        let input = update_event_input(request, None, EventSpanDto::This)?;
        let location = input
            .location
            .flatten()
            .ok_or("expected the event location to survive conversion")?;
        assert_eq!(location.latitude, Some(51.5));
        assert_eq!(location.longitude, Some(-0.12));
        Ok(())
    }

    #[test]
    fn map_eventkit_read_only_to_forbidden() {
        let error = map_eventkit_error(EventKitError::ReadOnlyCalendar);
        assert_eq!(error.status(), axum::http::StatusCode::FORBIDDEN);
    }

    /// A partial update that inverts the range has to answer with the same code a create does,
    /// so a client sees one error for one mistake.
    #[test]
    fn map_eventkit_end_before_start_matches_the_create_path() {
        let error = map_eventkit_error(EventKitError::EndBeforeStart);
        assert_eq!(error.status(), axum::http::StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(error.body().code, ErrorCode::EventEndBeforeStart);
    }

    #[test]
    fn map_eventkit_not_found() {
        let error = map_eventkit_error(EventKitError::NotFound);
        assert_eq!(error.status(), axum::http::StatusCode::NOT_FOUND);
    }

    #[test]
    fn alarm_input_rejects_location_and_unknown_kinds() -> Result<(), Box<dyn std::error::Error>> {
        for kind in [AlarmKindDto::Location, AlarmKindDto::Unknown] {
            let error = alarm_input(AlarmInputDto {
                kind,
                at: None,
                offset_seconds: Some(-600),
            })
            .err()
            .ok_or("expected unsupported alarm kind")?;
            assert_eq!(error.status(), axum::http::StatusCode::UNPROCESSABLE_ENTITY);
            let body = serde_json::to_value(error.body())?;
            assert_eq!(body["code"], "unsupported_alarm_kind");
            assert_eq!(body["details"]["field"], "alarms.kind");
        }
        Ok(())
    }

    #[test]
    fn alarm_input_accepts_relative_and_absolute() -> Result<(), Box<dyn std::error::Error>> {
        let relative = alarm_input(AlarmInputDto {
            kind: AlarmKindDto::Relative,
            at: None,
            offset_seconds: Some(-600),
        })?;
        assert_eq!(relative.kind, AlarmKind::Relative);
        assert_eq!(relative.offset_seconds, Some(-600));

        let absolute = alarm_input(AlarmInputDto {
            kind: AlarmKindDto::Absolute,
            at: Some(crate::apple_types::UnixTimestamp::from_seconds(
                1_700_000_000,
            )),
            offset_seconds: None,
        })?;
        assert_eq!(absolute.kind, AlarmKind::Absolute);
        assert_eq!(absolute.at, Some(1_700_000_000));
        Ok(())
    }
}
