use axum::{Json, extract::State, http::StatusCode};

use super::health::require_calendar_db;
use crate::{
    api::{
        dto::calendar::{
            CreateEventRequest, DeleteEventParams, UpdateEventParams, UpdateEventRequest,
        },
        error::{ApiError, ErrorCode, ErrorResponse},
        eventkit::require_eventkit_events,
        eventkit_convert::{
            calendar_hint, create_event_input, delete_event_input, map_eventkit_error,
            update_event_input, validate_create_event, validate_update_event,
        },
        extract::{ApiJson, ApiPath, ApiQuery},
        hydrate::{SyncPendingEventDetailDto, mutation_status},
        params::{CalendarIdPath, EventIdPath},
        router::AppState,
    },
    apple_types::EventId,
    calendar::{CalendarRepository, EventKitIdentifiers},
    db::run_timed_query,
};

/// Create a calendar event
#[utoipa::path(
    post,
    path = "/v1/calendars/{calendar_id}/events",
    operation_id = "createEvent",
    tag = "events",
    params(CalendarIdPath),
    request_body = CreateEventRequest,
    responses(
        (status = 201, description = "Event created and hydrated from SQLite", body = SyncPendingEventDetailDto),
        (status = 202, description = "Event created; SQLite read path still syncing", body = SyncPendingEventDetailDto),
        (status = 403, description = "Read-only calendar", body = ErrorResponse),
        (status = 422, description = "Inverted date range, or an immutable field such as status", body = ErrorResponse),
        (status = 503, description = "Calendar or EventKit unavailable", body = ErrorResponse),
    )
)]
pub async fn create_event(
    State(state): State<AppState>,
    ApiPath(path): ApiPath<CalendarIdPath>,
    ApiJson(request): ApiJson<CreateEventRequest>,
) -> Result<(StatusCode, Json<SyncPendingEventDetailDto>), ApiError> {
    validate_create_event(&request)?;

    let pool = require_calendar_db(&state.calendar_db)?;
    let eventkit = require_eventkit_events(&state).await?;
    let calendar_id = path.validated()?;

    let metadata = run_timed_query(|| async {
        CalendarRepository::new(pool)
            .get_calendar_resolve_metadata(calendar_id.as_str())
            .await
    })
    .await
    .map_err(ApiError::from_sqlx)?
    .ok_or_else(|| ApiError::new(ErrorCode::CalendarNotFound))?;

    let saved = eventkit
        .create_event(calendar_hint(metadata), create_event_input(request)?)
        .await
        .map_err(map_eventkit_error)?;

    // The read API's event id is EventKit's `calendarItemIdentifier`, lowercased.
    let event_id = EventId::new(saved.calendar_item_id.to_ascii_lowercase());
    let response = crate::api::hydrate::hydrate_event(pool, event_id).await?;
    Ok((mutation_status(response.sync_pending, true), Json(response)))
}

/// Update a calendar event
#[utoipa::path(
    patch,
    path = "/v1/events/{event_id}",
    operation_id = "updateEvent",
    tag = "events",
    params(EventIdPath, UpdateEventParams),
    request_body = UpdateEventRequest,
    responses(
        (status = 200, description = "Event updated and hydrated from SQLite", body = SyncPendingEventDetailDto),
        (status = 202, description = "Event updated; SQLite read path still syncing", body = SyncPendingEventDetailDto),
        (status = 404, description = "Event not found", body = ErrorResponse),
        (status = 422, description = "Inverted merged date range, or an immutable field such as status", body = ErrorResponse),
        (status = 503, description = "Calendar or EventKit unavailable", body = ErrorResponse),
    )
)]
pub async fn update_event(
    State(state): State<AppState>,
    ApiPath(path): ApiPath<EventIdPath>,
    ApiQuery(params): ApiQuery<UpdateEventParams>,
    ApiJson(request): ApiJson<UpdateEventRequest>,
) -> Result<(StatusCode, Json<SyncPendingEventDetailDto>), ApiError> {
    use crate::api::dto::calendar::EventSpanDto;

    validate_update_event(&request)?;

    let pool = require_calendar_db(&state.calendar_db)?;
    let eventkit = require_eventkit_events(&state).await?;
    let event_id = path.validated()?;

    let calendar_hint = if let Some(calendar_id) = request.calendar_id.as_ref() {
        let metadata = run_timed_query(|| async {
            CalendarRepository::new(pool)
                .get_calendar_resolve_metadata(calendar_id.as_str())
                .await
        })
        .await
        .map_err(ApiError::from_sqlx)?
        .ok_or_else(|| ApiError::new(ErrorCode::CalendarNotFound))?;
        Some(calendar_hint(metadata))
    } else {
        None
    };

    let identifiers = eventkit_identifiers(pool, &event_id).await?;

    let span = request.span.unwrap_or(EventSpanDto::This);
    eventkit
        .update_event(
            &identifiers.calendar_item_id,
            identifiers.external_id.as_deref(),
            params.occurrence_start.map(|value| value.seconds()),
            update_event_input(request, calendar_hint, span)?,
        )
        .await
        .map_err(map_eventkit_error)?;

    let response = crate::api::hydrate::hydrate_event(pool, event_id).await?;
    Ok((
        mutation_status(response.sync_pending, false),
        Json(response),
    ))
}

/// Delete a calendar event
#[utoipa::path(
    delete,
    path = "/v1/events/{event_id}",
    operation_id = "deleteEvent",
    tag = "events",
    params(EventIdPath, DeleteEventParams),
    responses(
        (status = 204, description = "Event deleted"),
        (status = 404, description = "Event not found", body = ErrorResponse),
        (status = 503, description = "Calendar or EventKit unavailable", body = ErrorResponse),
    )
)]
pub async fn delete_event(
    State(state): State<AppState>,
    ApiPath(path): ApiPath<EventIdPath>,
    ApiQuery(params): ApiQuery<DeleteEventParams>,
) -> Result<StatusCode, ApiError> {
    use crate::api::dto::calendar::EventSpanDto;

    let pool = require_calendar_db(&state.calendar_db)?;
    let eventkit = require_eventkit_events(&state).await?;
    let event_id = path.validated()?;
    let identifiers = eventkit_identifiers(pool, &event_id).await?;

    let span = params.span.unwrap_or(EventSpanDto::This);
    eventkit
        .delete_event(
            &identifiers.calendar_item_id,
            identifiers.external_id.as_deref(),
            delete_event_input(span, params.occurrence_start.map(|value| value.seconds())),
        )
        .await
        .map_err(map_eventkit_error)?;

    Ok(StatusCode::NO_CONTENT)
}

/// Resolves the read API's event id to the identifiers EventKit looks the event up by.
///
/// The API id is `lower(CalendarItem.UUID)`, but EventKit compares `calendarItemIdentifier`
/// case-sensitively against the stored `UUID`, and its external identifier is the iCalendar UID in
/// `unique_identifier`. When the row is not in SQLite yet (the write path can run ahead of the
/// read path), fall back to the uppercase form EventKit hands out for its identifiers.
async fn eventkit_identifiers(
    pool: &sqlx::SqlitePool,
    event_id: &EventId,
) -> Result<EventKitIdentifiers, ApiError> {
    let stored = run_timed_query(|| async {
        CalendarRepository::new(pool)
            .get_eventkit_identifiers(event_id.as_str())
            .await
    })
    .await
    .map_err(ApiError::from_sqlx)?;
    Ok(stored.unwrap_or_else(|| EventKitIdentifiers {
        calendar_item_id: event_id.as_str().to_ascii_uppercase(),
        external_id: None,
    }))
}
