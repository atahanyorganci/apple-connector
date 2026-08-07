use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use sqlx::SqlitePool;

use super::health::require_calendar_db;
use crate::{
    api::{
        dto::{
            CalendarAccountPageDto, CalendarDetailDto, CalendarPageDto, EventPageDto,
            calendar_convert::{
                calendar_account_page_to_dto, calendar_detail_to_dto, calendar_page_to_dto,
                event_page_to_dto,
            },
        },
        error::{ApiError, ErrorResponse},
        params::{CalendarIdPath, EventListParams, PageParams},
        router::AppState,
    },
    calendar::{
        CalendarRepository, Event, EventDetail, EventSummary, Page,
        enums::{Availability, InvitationStatus, PrivacyLevel},
    },
    db::run_timed_query,
};

const ICS_CONTENT_TYPE: &str = "text/calendar; charset=utf-8";
const CALDAV_CONTENT_TYPE: &str = "application/caldav+xml; charset=utf-8";

/// List calendar accounts
#[utoipa::path(
    get,
    path = "/v1/calendar-accounts",
    operation_id = "listCalendarAccounts",
    tag = "calendar-accounts",
    responses(
        (status = 200, description = "Calendar accounts", body = CalendarAccountPageDto),
        (status = 503, description = "Calendar database is unavailable", body = ErrorResponse),
    )
)]
pub async fn list_calendar_accounts(
    State(state): State<AppState>,
) -> Result<Json<CalendarAccountPageDto>, ApiError> {
    let pool = require_calendar_db(&state.calendar_db)?;
    let accounts =
        run_timed_query(|| async { CalendarRepository::new(pool).list_accounts().await })
            .await
            .map_err(|error| ApiError::internal(error.to_string()))?;
    Ok(Json(calendar_account_page_to_dto(accounts)))
}

/// List calendars
#[utoipa::path(
    get,
    path = "/v1/calendars",
    operation_id = "listCalendars",
    tag = "calendars",
    params(PageParams),
    responses(
        (status = 200, description = "Paginated calendars", body = CalendarPageDto),
        (status = 400, description = "Invalid query parameters", body = ErrorResponse),
        (status = 503, description = "Calendar database is unavailable", body = ErrorResponse),
    )
)]
pub async fn list_calendars(
    State(state): State<AppState>,
    Query(params): Query<PageParams>,
) -> Result<Json<CalendarPageDto>, ApiError> {
    let pool = require_calendar_db(&state.calendar_db)?;
    let limit = params.validated_limit()?;
    params.validated_cursor()?;
    let cursor = params
        .cursor
        .as_deref()
        .map(crate::api::cursor::decode::<crate::api::cursor::CalendarListCursor>)
        .transpose()?;
    let page = run_timed_query(|| async {
        CalendarRepository::new(pool)
            .list_calendars(limit, cursor)
            .await
    })
    .await
    .map_err(|error| ApiError::internal(error.to_string()))?;
    Ok(Json(calendar_page_to_dto(
        page.items,
        page.has_more,
        page.next_cursor,
        limit,
    )))
}

/// Get a calendar
#[utoipa::path(
    get,
    path = "/v1/calendars/{calendar_id}",
    operation_id = "getCalendar",
    tag = "calendars",
    params(CalendarIdPath),
    responses(
        (status = 200, description = "Calendar detail", body = CalendarDetailDto),
        (status = 404, description = "Calendar not found", body = ErrorResponse),
        (status = 503, description = "Calendar database is unavailable", body = ErrorResponse),
    )
)]
pub async fn get_calendar(
    State(state): State<AppState>,
    axum::extract::Path(path): axum::extract::Path<CalendarIdPath>,
) -> Result<Json<CalendarDetailDto>, ApiError> {
    let calendar_id = path.validated()?;
    let pool = require_calendar_db(&state.calendar_db)?;
    let calendar = run_timed_query(|| async {
        CalendarRepository::new(pool)
            .get_calendar(calendar_id.as_str())
            .await
    })
    .await
    .map_err(|error| ApiError::internal(error.to_string()))?
    .ok_or_else(|| ApiError::not_found("Calendar not found"))?;
    Ok(Json(calendar_detail_to_dto(&calendar)))
}

/// List events for a calendar as JSON
#[utoipa::path(
    get,
    path = "/v1/calendars/{calendar_id}/events",
    operation_id = "listCalendarEvents",
    tag = "events",
    params(CalendarIdPath, EventListParams),
    responses(
        (status = 200, description = "Paginated events", body = EventPageDto,
            content_type = "application/json"),
        (status = 400, description = "Invalid query parameters", body = ErrorResponse),
        (status = 503, description = "Calendar database is unavailable", body = ErrorResponse),
    )
)]
pub async fn list_calendar_events(
    State(state): State<AppState>,
    axum::extract::Path(path): axum::extract::Path<CalendarIdPath>,
    Query(params): Query<EventListParams>,
) -> Result<Json<EventPageDto>, ApiError> {
    let pool = require_calendar_db(&state.calendar_db)?;
    let calendar_id = path.validated()?;
    let page = fetch_calendar_event_page(pool, calendar_id.as_str(), &params).await?;
    Ok(event_page_json(
        page.items,
        page.has_more,
        page.next_cursor,
        params.validated_limit()?,
    ))
}

/// List events for a calendar as iCalendar
#[utoipa::path(
    get,
    path = "/v1/calendars/{calendar_id}/events/iCal",
    operation_id = "listCalendarEventsIcal",
    tag = "events",
    params(CalendarIdPath, EventListParams),
    responses(
        (status = 200, description = "iCalendar feed", content_type = "text/calendar"),
        (status = 400, description = "Invalid query parameters", body = ErrorResponse),
        (status = 503, description = "Calendar database is unavailable", body = ErrorResponse),
    )
)]
pub async fn list_calendar_events_ical(
    State(state): State<AppState>,
    axum::extract::Path(path): axum::extract::Path<CalendarIdPath>,
    Query(params): Query<EventListParams>,
) -> Result<Response, ApiError> {
    let pool = require_calendar_db(&state.calendar_db)?;
    let calendar_id = path.validated()?;
    let page = fetch_calendar_event_page(pool, calendar_id.as_str(), &params).await?;
    event_page_ics(page.items)
}

/// List events for a calendar as CalDAV XML
#[utoipa::path(
    get,
    path = "/v1/calendars/{calendar_id}/events/caldav",
    operation_id = "listCalendarEventsCaldav",
    tag = "events",
    params(CalendarIdPath, EventListParams),
    responses(
        (status = 200, description = "CalDAV multistatus", content_type = "application/caldav+xml"),
        (status = 400, description = "Invalid query parameters", body = ErrorResponse),
        (status = 503, description = "Calendar database is unavailable", body = ErrorResponse),
    )
)]
pub async fn list_calendar_events_caldav(
    State(state): State<AppState>,
    axum::extract::Path(path): axum::extract::Path<CalendarIdPath>,
    Query(params): Query<EventListParams>,
) -> Result<Response, ApiError> {
    let pool = require_calendar_db(&state.calendar_db)?;
    let calendar_id = path.validated()?;
    let page = fetch_calendar_event_page(pool, calendar_id.as_str(), &params).await?;
    event_page_caldav(page.items)
}

pub(crate) async fn fetch_calendar_event_page(
    pool: &SqlitePool,
    calendar_id: &str,
    params: &EventListParams,
) -> Result<Page<EventSummary>, ApiError> {
    let limit = params.validated_limit()?;
    params.validated_cursor()?;
    let filters = params.validated_filters()?;
    let cursor = params
        .cursor
        .as_deref()
        .map(crate::api::cursor::decode::<crate::api::cursor::CalendarEventCursor>)
        .transpose()?;
    run_timed_query(|| async {
        CalendarRepository::new(pool)
            .list_calendar_events(calendar_id, &filters, limit, cursor)
            .await
    })
    .await
    .map_err(|error| ApiError::internal(error.to_string()))
}

pub(crate) fn event_page_json(
    items: Vec<EventSummary>,
    has_more: bool,
    next_cursor: Option<String>,
    limit: u32,
) -> Json<EventPageDto> {
    Json(event_page_to_dto(items, has_more, next_cursor, limit))
}

pub(crate) fn event_page_ics(items: Vec<EventSummary>) -> Result<Response, ApiError> {
    let mut body = String::new();
    for summary in &items {
        let detail = empty_event_detail(summary.clone());
        let event: Event = (&detail).into();
        body.push_str(
            &serde_icalendar::to_string(&event.to_ics_event())
                .map_err(|e| ApiError::internal(e.to_string()))?,
        );
        body.push('\n');
    }
    Ok((
        StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, ICS_CONTENT_TYPE)],
        body,
    )
        .into_response())
}

pub(crate) fn event_page_caldav(items: Vec<EventSummary>) -> Result<Response, ApiError> {
    let mut xml_parts = Vec::new();
    for summary in &items {
        let detail = empty_event_detail(summary.clone());
        let event: Event = (&detail).into();
        let object = serde_caldav::CalDavCalendarObject {
            href: Some(format!("/v1/events/{}/caldav", summary.id)),
            etag: None,
            content_type: Some("text/calendar; charset=utf-8".to_owned()),
            event: event.to_ics_event(),
        };
        xml_parts
            .push(serde_caldav::to_string(&object).map_err(|e| ApiError::internal(e.to_string()))?);
    }
    Ok((
        StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, CALDAV_CONTENT_TYPE)],
        xml_parts.join("\n"),
    )
        .into_response())
}

fn empty_event_detail(summary: EventSummary) -> EventDetail {
    EventDetail {
        summary,
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
        invitation_status: InvitationStatus::Unknown,
        availability: Availability::Busy,
        privacy_level: PrivacyLevel::Default,
        series_id: None,
        series_row_id: None,
        original_start: None,
        last_modified: None,
        creation_date: None,
        structured_data: None,
        app_link: None,
    }
}
