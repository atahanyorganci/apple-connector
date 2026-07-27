use axum::{
    Json,
    body::Bytes,
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::Response,
};

use super::{calendars::respond_with_events, health::require_calendar_db};
use crate::{
    api::{
        dto::{EventDetailDto, calendar_convert::event_detail_to_dto},
        error::{ApiError, ErrorResponse},
        format::{ResponseFormat, parse_request_format, resolve_format},
        params::{EventIdPath, EventListParams},
        router::AppState,
    },
    calendar::{CalendarRepository, Event, EventDetail},
    db::run_timed_query,
};

/// List events globally
#[utoipa::path(
    get,
    path = "/v1/events",
    operation_id = "listEvents",
    tag = "events",
    params(EventListParams),
    responses(
        (status = 200, description = "Paginated events", body = crate::api::dto::calendar::EventPageDto,
            content_type = "application/json"),
        (status = 200, description = "iCalendar feed", content_type = "text/calendar"),
        (status = 200, description = "CalDAV multistatus", content_type = "application/caldav+xml"),
        (status = 400, description = "Invalid query parameters", body = ErrorResponse),
        (status = 503, description = "Calendar database is unavailable", body = ErrorResponse),
    )
)]
pub async fn list_events(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<EventListParams>,
) -> Result<Response, ApiError> {
    let pool = require_calendar_db(&state.calendar_db)?;
    let limit = params.validated_limit()?;
    params.validated_cursor()?;
    let filters = params.validated_filters()?;
    let filter_snapshot = filters.snapshot();
    let format = resolve_format(&headers, params.format.as_deref())?;

    let page = if filters.q.is_some() {
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
    }
    .map_err(|error| ApiError::internal(error.to_string()))?;

    respond_with_events(page.items, page.has_more, page.next_cursor, limit, format).await
}

/// Get an event
#[utoipa::path(
    get,
    path = "/v1/events/{event_id}",
    operation_id = "getEvent",
    tag = "events",
    params(EventIdPath, EventListParams),
    responses(
        (status = 200, description = "Event detail", body = EventDetailDto,
            content_type = "application/json"),
        (status = 200, description = "iCalendar document", content_type = "text/calendar"),
        (status = 200, description = "CalDAV resource", content_type = "application/caldav+xml"),
        (status = 404, description = "Event not found", body = ErrorResponse),
        (status = 503, description = "Calendar database is unavailable", body = ErrorResponse),
    )
)]
pub async fn get_event(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Path(path): axum::extract::Path<EventIdPath>,
    Query(params): Query<EventListParams>,
) -> Result<Response, ApiError> {
    let pool = require_calendar_db(&state.calendar_db)?;
    let format = resolve_format(&headers, params.format.as_deref())?;
    let event = run_timed_query(|| async {
        CalendarRepository::new(pool)
            .get_event(&path.event_id)
            .await
    })
    .await
    .map_err(|error| ApiError::internal(error.to_string()))?
    .ok_or_else(|| ApiError::not_found("Event not found"))?;
    respond_with_event(&event, format).await
}

/// Parse an ICS or CalDAV payload into a JSON event DTO (no database writes).
#[utoipa::path(
    post,
    path = "/v1/events/parse",
    operation_id = "parseEvent",
    tag = "events",
    request_body(content = String, content_type = "text/calendar"),
    responses(
        (status = 200, description = "Parsed event", body = EventDetailDto),
        (status = 400, description = "Invalid payload", body = ErrorResponse),
    )
)]
pub async fn parse_event(
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<EventDetailDto>, ApiError> {
    let format = parse_request_format(&headers)?;
    let interchange: Event = match format {
        ResponseFormat::Ics => serde_icalendar::from_slice(&body)
            .map_err(|error| ApiError::validation(error.to_string()))?,
        ResponseFormat::CalDav => {
            let object: serde_caldav::CalDavCalendarObject = serde_caldav::from_slice(&body)
                .map_err(|error| ApiError::validation(error.to_string()))?;
            serde_json::from_value(
                serde_json::to_value(object.event)
                    .map_err(|error| ApiError::internal(error.to_string()))?,
            )
            .map_err(|error| ApiError::validation(error.to_string()))?
        }
        ResponseFormat::Json => {
            return Err(ApiError::validation_with_details(
                "parse endpoint expects text/calendar or application/caldav+xml",
                serde_json::json!({ "field": "content-type" }),
            ));
        }
    };
    Ok(Json(interchange_to_detail_dto(&interchange)))
}

async fn respond_with_event(
    event: &EventDetail,
    format: ResponseFormat,
) -> Result<Response, ApiError> {
    match format {
        ResponseFormat::Json => Ok(Json(event_detail_to_dto(event)).into_response()),
        ResponseFormat::Ics => {
            let interchange: Event = event.into();
            let body = serde_icalendar::to_string(&interchange.to_ics_event())
                .map_err(|error| ApiError::internal(error.to_string()))?;
            Ok((
                StatusCode::OK,
                [(axum::http::header::CONTENT_TYPE, format.content_type())],
                body,
            )
                .into_response())
        }
        ResponseFormat::CalDav => {
            let interchange: Event = event.into();
            let ics_event = interchange.to_ics_event();
            let object = serde_caldav::CalDavCalendarObject {
                href: Some(format!("/v1/events/{}", event.summary.id)),
                etag: None,
                content_type: Some("text/calendar; charset=utf-8".to_owned()),
                event: ics_event,
            };
            let body = serde_caldav::to_string(&object)
                .map_err(|error| ApiError::internal(error.to_string()))?;
            Ok((
                StatusCode::OK,
                [(axum::http::header::CONTENT_TYPE, format.content_type())],
                body,
            )
                .into_response())
        }
    }
}

fn interchange_to_detail_dto(event: &Event) -> EventDetailDto {
    use chrono::TimeZone;

    use crate::calendar::{
        EventDetail, EventLocation, EventParticipant, EventSummary, InterchangeStatus,
        RecurrenceRule,
        enums::{Availability, EventClass, EventStatus, InvitationStatus, PrivacyLevel},
    };

    let summary = EventSummary {
        row_id: 0,
        id: event.uid.clone(),
        calendar_row_id: 0,
        calendar_id: String::new(),
        summary: event.summary.clone(),
        start: event
            .start
            .and_then(|ts| chrono::Utc.timestamp_opt(ts, 0).single()),
        end: event
            .end
            .and_then(|ts| chrono::Utc.timestamp_opt(ts, 0).single()),
        all_day: event.all_day,
        status: event
            .status
            .map(|s| match s {
                InterchangeStatus::Confirmed => EventStatus::Confirmed,
                InterchangeStatus::Tentative => EventStatus::Tentative,
                InterchangeStatus::Cancelled => EventStatus::Cancelled,
            })
            .unwrap_or_default(),
        hidden: false,
        is_recurring: event.recurrence_rule.is_some(),
        occurrence_start: None,
        occurrence_end: None,
        event_class: EventClass::Standard,
    };
    let detail = EventDetail {
        summary,
        description: event.description.clone(),
        url: event.url.clone(),
        location: event.location.as_ref().map(|l| EventLocation {
            title: Some(l.clone()),
            address: None,
            latitude: None,
            longitude: None,
        }),
        organizer: event
            .organizer_email
            .as_ref()
            .map(|email| EventParticipant {
                id: String::new(),
                email: Some(email.clone()),
                phone_number: None,
                name: None,
                is_self: false,
                status: InvitationStatus::Unknown,
                role: None,
                comment: None,
            }),
        attendees: event
            .attendees
            .iter()
            .map(|a| EventParticipant {
                id: String::new(),
                email: Some(a.email.clone()),
                phone_number: None,
                name: a.name.clone(),
                is_self: false,
                status: InvitationStatus::Unknown,
                role: None,
                comment: None,
            })
            .collect(),
        recurrence: event.recurrence_rule.as_ref().map(|spec| RecurrenceRule {
            frequency: 0,
            interval: 1,
            count: None,
            end_date: None,
            specifier: Some(spec.clone()),
            raw_specifier: spec.clone(),
        }),
        exception_dates: event
            .exception_dates
            .iter()
            .filter_map(|ts| chrono::Utc.timestamp_opt(*ts, 0).single())
            .collect(),
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
    };
    event_detail_to_dto(&detail)
}

use axum::response::IntoResponse;
