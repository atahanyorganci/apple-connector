use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};

use super::health::require_reminders_db;
use crate::{
    api::{
        dto::reminder::{CreateReminderRequest, UpdateReminderRequest},
        error::{ApiError, ErrorResponse},
        eventkit::require_eventkit_reminders,
        eventkit_convert::{
            create_reminder_input, map_eventkit_error, reminder_list_hint, update_reminder_input,
            validate_create_reminder, validate_update_reminder,
        },
        hydrate::SyncPendingReminderDetailDto,
        params::{ReminderIdPath, ReminderListIdPath},
        router::AppState,
    },
    db::run_timed_query,
    reminders::ReminderRepository,
};

/// Create a reminder in a list
#[utoipa::path(
    post,
    path = "/v1/reminder-lists/{list_id}/reminders",
    operation_id = "createReminder",
    tag = "reminders",
    params(ReminderListIdPath),
    request_body = CreateReminderRequest,
    responses(
        (status = 201, description = "Reminder created", body = SyncPendingReminderDetailDto),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 403, description = "Smart list or read-only target", body = ErrorResponse),
        (status = 422, description = "Unsupported field", body = ErrorResponse),
        (status = 503, description = "Reminders or EventKit unavailable", body = ErrorResponse),
    )
)]
pub async fn create_reminder(
    State(state): State<AppState>,
    Path(path): Path<ReminderListIdPath>,
    Json(request): Json<CreateReminderRequest>,
) -> Result<(StatusCode, Json<SyncPendingReminderDetailDto>), ApiError> {
    validate_create_reminder(&request)?;
    let pool = require_reminders_db(&state.reminders_db)?;
    let eventkit = require_eventkit_reminders(&state).await?;
    let list_id = path.list_id.as_str();

    let metadata = run_timed_query(|| async {
        ReminderRepository::new(pool)
            .get_list_resolve_metadata(list_id)
            .await
    })
    .await
    .map_err(|error| ApiError::internal(error.to_string()))?
    .ok_or_else(|| ApiError::not_found("reminder list not found"))?;

    if metadata.is_smart_list {
        return Err(ApiError::forbidden("cannot write to smart reminder lists"));
    }

    let saved = eventkit
        .create_reminder(reminder_list_hint(metadata), create_reminder_input(request))
        .await
        .map_err(map_eventkit_error)?;

    let response = crate::api::hydrate::hydrate_reminder(pool, &saved.external_id).await?;
    Ok((StatusCode::CREATED, Json(response)))
}

/// Update a reminder
#[utoipa::path(
    patch,
    path = "/v1/reminders/{reminder_id}",
    operation_id = "updateReminder",
    tag = "reminders",
    params(ReminderIdPath),
    request_body = UpdateReminderRequest,
    responses(
        (status = 200, description = "Reminder updated", body = SyncPendingReminderDetailDto),
        (status = 404, description = "Reminder not found", body = ErrorResponse),
        (status = 422, description = "Unsupported field", body = ErrorResponse),
        (status = 503, description = "Reminders or EventKit unavailable", body = ErrorResponse),
    )
)]
pub async fn update_reminder(
    State(state): State<AppState>,
    Path(path): Path<ReminderIdPath>,
    Json(request): Json<UpdateReminderRequest>,
) -> Result<Json<SyncPendingReminderDetailDto>, ApiError> {
    validate_update_reminder(&request)?;
    let pool = require_reminders_db(&state.reminders_db)?;
    let eventkit = require_eventkit_reminders(&state).await?;
    let reminder_id = path.reminder_id.as_str();

    let list_hint = if let Some(list_id) = request.list_id.as_ref() {
        let metadata = run_timed_query(|| async {
            ReminderRepository::new(pool)
                .get_list_resolve_metadata(list_id.as_str())
                .await
        })
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?
        .ok_or_else(|| ApiError::not_found("reminder list not found"))?;
        if metadata.is_smart_list {
            return Err(ApiError::forbidden("cannot write to smart reminder lists"));
        }
        Some(reminder_list_hint(metadata))
    } else {
        None
    };

    let external_id = run_timed_query(|| async {
        ReminderRepository::new(pool)
            .get_reminder_external_id(reminder_id)
            .await
    })
    .await
    .map_err(|error| ApiError::internal(error.to_string()))?;

    let saved = eventkit
        .update_reminder(
            reminder_id,
            external_id.as_deref(),
            update_reminder_input(request, list_hint),
        )
        .await
        .map_err(map_eventkit_error)?;

    let response = crate::api::hydrate::hydrate_reminder(pool, &saved.external_id).await?;
    Ok(Json(response))
}

/// Delete a reminder
#[utoipa::path(
    delete,
    path = "/v1/reminders/{reminder_id}",
    operation_id = "deleteReminder",
    tag = "reminders",
    params(ReminderIdPath),
    responses(
        (status = 204, description = "Reminder deleted"),
        (status = 404, description = "Reminder not found", body = ErrorResponse),
        (status = 503, description = "Reminders or EventKit unavailable", body = ErrorResponse),
    )
)]
pub async fn delete_reminder(
    State(state): State<AppState>,
    Path(path): Path<ReminderIdPath>,
) -> Result<StatusCode, ApiError> {
    let pool = require_reminders_db(&state.reminders_db)?;
    let eventkit = require_eventkit_reminders(&state).await?;
    let reminder_id = path.reminder_id.as_str();
    let external_id = run_timed_query(|| async {
        ReminderRepository::new(pool)
            .get_reminder_external_id(reminder_id)
            .await
    })
    .await
    .map_err(|error| ApiError::internal(error.to_string()))?;

    eventkit
        .delete_reminder(reminder_id, external_id.as_deref())
        .await
        .map_err(map_eventkit_error)?;

    Ok(StatusCode::NO_CONTENT)
}
