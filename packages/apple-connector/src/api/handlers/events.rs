use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use sqlx::SqlitePool;

use super::{
    calendars::{event_page_caldav, event_page_ics, event_page_json},
    health::require_calendar_db,
};
use crate::{
    api::{
        dto::{EventDetailDto, EventPageDto, calendar_convert::event_detail_to_dto},
        error::{ApiError, ErrorResponse},
        params::{EventIdPath, EventListParams},
        router::AppState,
    },
    apple_types::EventId,
    calendar::{CalendarRepository, Event, EventDetail, EventSummary, Page},
    db::run_timed_query,
};

const ICS_CONTENT_TYPE: &str = "text/calendar; charset=utf-8";
const CALDAV_CONTENT_TYPE: &str = "application/caldav+xml; charset=utf-8";

/// List events globally as JSON
#[utoipa::path(
    get,
    path = "/v1/events",
    operation_id = "listEvents",
    tag = "events",
    params(EventListParams),
    responses(
        (status = 200, description = "Paginated events", body = EventPageDto,
            content_type = "application/json"),
        (status = 400, description = "Invalid query parameters", body = ErrorResponse),
        (status = 503, description = "Calendar database is unavailable", body = ErrorResponse),
    )
)]
pub async fn list_events(
    State(state): State<AppState>,
    Query(params): Query<EventListParams>,
) -> Result<Json<EventPageDto>, ApiError> {
    let pool = require_calendar_db(&state.calendar_db)?;
    let page = fetch_event_page(pool, &params).await?;
    Ok(event_page_json(
        page.items,
        page.has_more,
        page.next_cursor,
        params.validated_limit()?,
    ))
}

/// List events globally as iCalendar
#[utoipa::path(
    get,
    path = "/v1/events/iCal",
    operation_id = "listEventsIcal",
    tag = "events",
    params(EventListParams),
    responses(
        (status = 200, description = "iCalendar feed", content_type = "text/calendar"),
        (status = 400, description = "Invalid query parameters", body = ErrorResponse),
        (status = 503, description = "Calendar database is unavailable", body = ErrorResponse),
    )
)]
pub async fn list_events_ical(
    State(state): State<AppState>,
    Query(params): Query<EventListParams>,
) -> Result<Response, ApiError> {
    let pool = require_calendar_db(&state.calendar_db)?;
    let page = fetch_event_page(pool, &params).await?;
    event_page_ics(page.items)
}

/// List events globally as CalDAV XML
#[utoipa::path(
    get,
    path = "/v1/events/caldav",
    operation_id = "listEventsCaldav",
    tag = "events",
    params(EventListParams),
    responses(
        (status = 200, description = "CalDAV multistatus", content_type = "application/caldav+xml"),
        (status = 400, description = "Invalid query parameters", body = ErrorResponse),
        (status = 503, description = "Calendar database is unavailable", body = ErrorResponse),
    )
)]
pub async fn list_events_caldav(
    State(state): State<AppState>,
    Query(params): Query<EventListParams>,
) -> Result<Response, ApiError> {
    let pool = require_calendar_db(&state.calendar_db)?;
    let page = fetch_event_page(pool, &params).await?;
    event_page_caldav(page.items)
}

/// Get an event as JSON
#[utoipa::path(
    get,
    path = "/v1/events/{event_id}",
    operation_id = "getEvent",
    tag = "events",
    params(EventIdPath),
    responses(
        (status = 200, description = "Event detail", body = EventDetailDto,
            content_type = "application/json"),
        (status = 404, description = "Event not found", body = ErrorResponse),
        (status = 503, description = "Calendar database is unavailable", body = ErrorResponse),
    )
)]
pub async fn get_event(
    State(state): State<AppState>,
    axum::extract::Path(path): axum::extract::Path<EventIdPath>,
) -> Result<Json<EventDetailDto>, ApiError> {
    let pool = require_calendar_db(&state.calendar_db)?;
    let event_id = path.validated()?;
    let event = fetch_event_detail(pool, &event_id).await?;
    Ok(event_detail_json(&event))
}

/// Get an event as iCalendar
#[utoipa::path(
    get,
    path = "/v1/events/{event_id}/iCal",
    operation_id = "getEventIcal",
    tag = "events",
    params(EventIdPath),
    responses(
        (status = 200, description = "iCalendar document", content_type = "text/calendar"),
        (status = 404, description = "Event not found", body = ErrorResponse),
        (status = 503, description = "Calendar database is unavailable", body = ErrorResponse),
    )
)]
pub async fn get_event_ical(
    State(state): State<AppState>,
    axum::extract::Path(path): axum::extract::Path<EventIdPath>,
) -> Result<Response, ApiError> {
    let pool = require_calendar_db(&state.calendar_db)?;
    let event_id = path.validated()?;
    let event = fetch_event_detail(pool, &event_id).await?;
    event_detail_ics(&event)
}

/// Get an event as CalDAV XML
#[utoipa::path(
    get,
    path = "/v1/events/{event_id}/caldav",
    operation_id = "getEventCaldav",
    tag = "events",
    params(EventIdPath),
    responses(
        (status = 200, description = "CalDAV resource", content_type = "application/caldav+xml"),
        (status = 404, description = "Event not found", body = ErrorResponse),
        (status = 503, description = "Calendar database is unavailable", body = ErrorResponse),
    )
)]
pub async fn get_event_caldav(
    State(state): State<AppState>,
    axum::extract::Path(path): axum::extract::Path<EventIdPath>,
) -> Result<Response, ApiError> {
    let pool = require_calendar_db(&state.calendar_db)?;
    let event_id = path.validated()?;
    let event = fetch_event_detail(pool, &event_id).await?;
    event_detail_caldav(&event)
}

async fn fetch_event_page(
    pool: &SqlitePool,
    params: &EventListParams,
) -> Result<Page<EventSummary>, ApiError> {
    let limit = params.validated_limit()?;
    params.validated_cursor()?;
    let filters = params.validated_filters()?;
    let filter_snapshot = filters.snapshot();

    if filters.q.is_some() {
        let cursor = params
            .cursor
            .as_deref()
            .map(|value| crate::api::cursor::decode_event_search_cursor(value, &filter_snapshot))
            .transpose()?;
        run_timed_query(|| async {
            CalendarRepository::new(pool)
                .search_events(&filters, limit, cursor)
                .await
        })
        .await
        .map_err(ApiError::from_sqlx)
    } else {
        let cursor = match params.cursor.as_deref() {
            None => None,
            Some(value) if filters.is_active() => Some(
                crate::api::cursor::decode_event_search_cursor(value, &filter_snapshot).map(
                    |cursor| crate::api::cursor::GlobalEventCursor {
                        modified_at: cursor.start_at,
                        row_id: cursor.row_id,
                    },
                )?,
            ),
            Some(value) => Some(crate::api::cursor::decode(value)?),
        };
        run_timed_query(|| async {
            CalendarRepository::new(pool)
                .list_events(&filters, limit, cursor)
                .await
        })
        .await
        .map_err(ApiError::from_sqlx)
    }
}

async fn fetch_event_detail(
    pool: &SqlitePool,
    event_id: &EventId,
) -> Result<EventDetail, ApiError> {
    run_timed_query(|| async {
        CalendarRepository::new(pool)
            .get_event(event_id.as_str())
            .await
    })
    .await
    .map_err(ApiError::from_sqlx)?
    .ok_or_else(|| ApiError::not_found("Event not found"))
}

fn event_detail_json(event: &EventDetail) -> Json<EventDetailDto> {
    Json(event_detail_to_dto(event))
}

fn event_detail_ics(event: &EventDetail) -> Result<Response, ApiError> {
    let interchange: Event = event.into();
    let body = serde_icalendar::to_string(&interchange.to_ics_event())
        .map_err(|_| ApiError::internal("serialization failed"))?;
    Ok((
        StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, ICS_CONTENT_TYPE)],
        body,
    )
        .into_response())
}

fn event_detail_caldav(event: &EventDetail) -> Result<Response, ApiError> {
    let interchange: Event = event.into();
    let ics_event = interchange.to_ics_event();
    let object = serde_caldav::CalDavCalendarObject {
        href: Some(format!("/v1/events/{}/caldav", event.summary.id)),
        etag: None,
        content_type: Some("text/calendar; charset=utf-8".to_owned()),
        event: ics_event,
    };
    let body =
        serde_caldav::to_string(&object).map_err(|_| ApiError::internal("serialization failed"))?;
    Ok((
        StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, CALDAV_CONTENT_TYPE)],
        body,
    )
        .into_response())
}
