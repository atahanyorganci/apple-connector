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
        error::{ApiError, ErrorResponse},
        eventkit::require_eventkit_events,
        eventkit_convert::{
            calendar_hint, create_event_input, delete_event_input, map_eventkit_error,
            update_event_input,
        },
        hydrate::SyncPendingEventDetailDto,
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
        (status = 201, description = "Event created", body = SyncPendingEventDetailDto),
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
        return Err(ApiError::unprocessable(
            "end must be greater than or equal to start",
        ));
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
    .map_err(|error| ApiError::internal(error.to_string()))?
    .ok_or_else(|| ApiError::not_found("calendar not found"))?;

    let saved = eventkit
        .create_event(calendar_hint(metadata), create_event_input(request))
        .await
        .map_err(map_eventkit_error)?;

    let response = crate::api::hydrate::hydrate_event(pool, &saved.external_id).await?;
    Ok((StatusCode::CREATED, Json(response)))
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
        (status = 200, description = "Event updated", body = SyncPendingEventDetailDto),
        (status = 404, description = "Event not found", body = ErrorResponse),
        (status = 503, description = "Calendar or EventKit unavailable", body = ErrorResponse),
    )
)]
pub async fn update_event(
    State(state): State<AppState>,
    Path(path): Path<EventIdPath>,
    Query(params): Query<UpdateEventParams>,
    Json(request): Json<UpdateEventRequest>,
) -> Result<Json<SyncPendingEventDetailDto>, ApiError> {
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
        .map_err(|error| ApiError::internal(error.to_string()))?
        .ok_or_else(|| ApiError::not_found("calendar not found"))?;
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
    .map_err(|error| ApiError::internal(error.to_string()))?;

    let span = request.span.unwrap_or(EventSpanDto::This);
    let saved = eventkit
        .update_event(
            event_id.as_str(),
            external_id.as_deref(),
            params.occurrence_start.map(|value| value.seconds()),
            update_event_input(request, calendar_hint, span),
        )
        .await
        .map_err(map_eventkit_error)?;

    let response = crate::api::hydrate::hydrate_event(pool, &saved.external_id).await?;
    Ok(Json(response))
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
    .map_err(|error| ApiError::internal(error.to_string()))?;

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
