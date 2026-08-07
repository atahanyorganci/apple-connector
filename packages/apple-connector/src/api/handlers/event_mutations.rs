use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};

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
            update_event_input,
        },
        hydrate::{SyncPendingEventDetailDto, mutation_status},
        params::{CalendarIdPath, EventIdPath},
        router::AppState,
    },
    calendar::CalendarRepository,
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
        (status = 503, description = "Calendar or EventKit unavailable", body = ErrorResponse),
    )
)]
pub async fn create_event(
    State(state): State<AppState>,
    Path(path): Path<CalendarIdPath>,
    Json(request): Json<CreateEventRequest>,
) -> Result<(StatusCode, Json<SyncPendingEventDetailDto>), ApiError> {
    if request.end.seconds() < request.start.seconds() {
        return Err(ApiError::new(ErrorCode::EventEndBeforeStart));
    }

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

    let response = crate::api::hydrate::hydrate_event(pool, &saved.external_id).await?;
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
        (status = 503, description = "Calendar or EventKit unavailable", body = ErrorResponse),
    )
)]
pub async fn update_event(
    State(state): State<AppState>,
    Path(path): Path<EventIdPath>,
    Query(params): Query<UpdateEventParams>,
    Json(request): Json<UpdateEventRequest>,
) -> Result<(StatusCode, Json<SyncPendingEventDetailDto>), ApiError> {
    use crate::api::dto::calendar::EventSpanDto;

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

    let external_id = run_timed_query(|| async {
        CalendarRepository::new(pool)
            .get_event_external_id(event_id.as_str())
            .await
    })
    .await
    .map_err(ApiError::from_sqlx)?;

    let span = request.span.unwrap_or(EventSpanDto::This);
    let saved = eventkit
        .update_event(
            event_id.as_str(),
            external_id.as_deref(),
            params.occurrence_start.map(|value| value.seconds()),
            update_event_input(request, calendar_hint, span)?,
        )
        .await
        .map_err(map_eventkit_error)?;

    let response = crate::api::hydrate::hydrate_event(pool, &saved.external_id).await?;
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
    Path(path): Path<EventIdPath>,
    Query(params): Query<DeleteEventParams>,
) -> Result<StatusCode, ApiError> {
    use crate::api::dto::calendar::EventSpanDto;

    let pool = require_calendar_db(&state.calendar_db)?;
    let eventkit = require_eventkit_events(&state).await?;
    let event_id = path.validated()?;
    let external_id = run_timed_query(|| async {
        CalendarRepository::new(pool)
            .get_event_external_id(event_id.as_str())
            .await
    })
    .await
    .map_err(ApiError::from_sqlx)?;

    let span = params.span.unwrap_or(EventSpanDto::This);
    eventkit
        .delete_event(
            event_id.as_str(),
            external_id.as_deref(),
            delete_event_input(span, params.occurrence_start.map(|value| value.seconds())),
        )
        .await
        .map_err(map_eventkit_error)?;

    Ok(StatusCode::NO_CONTENT)
}
