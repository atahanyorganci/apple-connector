use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};

use super::health::require_reminders_db;
use crate::{
    api::{
        dto::reminder::{CreateReminderRequest, UpdateReminderRequest},
        error::{ApiError, ErrorCode, ErrorResponse},
        eventkit::require_eventkit_reminders,
        eventkit_convert::{
            create_reminder_input, map_eventkit_error, reminder_list_hint, update_reminder_input,
            validate_create_reminder, validate_update_reminder,
        },
        hydrate::{SyncPendingReminderDetailDto, mutation_status},
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
        (status = 201, description = "Reminder created and hydrated from SQLite", body = SyncPendingReminderDetailDto),
        (status = 202, description = "Reminder created; SQLite read path still syncing", body = SyncPendingReminderDetailDto),
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

    let reminder_entity_ids = state
        .cached_reminders_entity_ids()
        .await
        .map_err(ApiError::from_sqlx)?;
    let eventkit = require_eventkit_reminders(&state).await?;
    path.validated_key()?;
    let list_id = path.list_id.as_str();

    let metadata = run_timed_query(|| async {
        ReminderRepository::with_entity_ids(pool, Arc::clone(&reminder_entity_ids))
            .get_list_resolve_metadata(list_id)
            .await
    })
    .await
    .map_err(ApiError::from_sqlx)?
    .ok_or_else(|| ApiError::new(ErrorCode::ReminderListNotFound))?;

    if metadata.is_smart_list {
        return Err(ApiError::new(ErrorCode::SmartListReadOnly));
    }

    let saved = eventkit
        .create_reminder(
            reminder_list_hint(metadata),
            create_reminder_input(request)?,
        )
        .await
        .map_err(map_eventkit_error)?;

    let response = {
        let entity_ids = state
            .cached_reminders_entity_ids()
            .await
            .map_err(ApiError::from_sqlx)?;
        crate::api::hydrate::hydrate_reminder(pool, entity_ids, &saved.external_id).await?
    };
    Ok((mutation_status(response.sync_pending, true), Json(response)))
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
        (status = 200, description = "Reminder updated and hydrated from SQLite", body = SyncPendingReminderDetailDto),
        (status = 202, description = "Reminder updated; SQLite read path still syncing", body = SyncPendingReminderDetailDto),
        (status = 404, description = "Reminder not found", body = ErrorResponse),
        (status = 422, description = "Unsupported field", body = ErrorResponse),
        (status = 503, description = "Reminders or EventKit unavailable", body = ErrorResponse),
    )
)]
pub async fn update_reminder(
    State(state): State<AppState>,
    Path(path): Path<ReminderIdPath>,
    Json(request): Json<UpdateReminderRequest>,
) -> Result<(StatusCode, Json<SyncPendingReminderDetailDto>), ApiError> {
    validate_update_reminder(&request)?;
    let pool = require_reminders_db(&state.reminders_db)?;

    let reminder_entity_ids = state
        .cached_reminders_entity_ids()
        .await
        .map_err(ApiError::from_sqlx)?;
    let eventkit = require_eventkit_reminders(&state).await?;
    let reminder_id = path.validated()?;

    let list_hint = if let Some(list_id) = request.list_id.as_ref() {
        let metadata = run_timed_query(|| async {
            ReminderRepository::with_entity_ids(pool, Arc::clone(&reminder_entity_ids))
                .get_list_resolve_metadata(list_id.as_str())
                .await
        })
        .await
        .map_err(ApiError::from_sqlx)?
        .ok_or_else(|| ApiError::new(ErrorCode::ReminderListNotFound))?;
        if metadata.is_smart_list {
            return Err(ApiError::new(ErrorCode::SmartListReadOnly));
        }
        Some(reminder_list_hint(metadata))
    } else {
        None
    };

    let external_id = run_timed_query(|| async {
        ReminderRepository::with_entity_ids(pool, Arc::clone(&reminder_entity_ids))
            .get_reminder_external_id(reminder_id.as_str())
            .await
    })
    .await
    .map_err(ApiError::from_sqlx)?;

    let saved = eventkit
        .update_reminder(
            reminder_id.as_str(),
            external_id.as_deref(),
            update_reminder_input(request, list_hint)?,
        )
        .await
        .map_err(map_eventkit_error)?;

    let response = {
        let entity_ids = state
            .cached_reminders_entity_ids()
            .await
            .map_err(ApiError::from_sqlx)?;
        crate::api::hydrate::hydrate_reminder(pool, entity_ids, &saved.external_id).await?
    };
    Ok((
        mutation_status(response.sync_pending, false),
        Json(response),
    ))
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

    let reminder_entity_ids = state
        .cached_reminders_entity_ids()
        .await
        .map_err(ApiError::from_sqlx)?;
    let eventkit = require_eventkit_reminders(&state).await?;
    let reminder_id = path.validated()?;
    let external_id = run_timed_query(|| async {
        ReminderRepository::with_entity_ids(pool, Arc::clone(&reminder_entity_ids))
            .get_reminder_external_id(reminder_id.as_str())
            .await
    })
    .await
    .map_err(ApiError::from_sqlx)?;

    eventkit
        .delete_reminder(reminder_id.as_str(), external_id.as_deref())
        .await
        .map_err(map_eventkit_error)?;

    Ok(StatusCode::NO_CONTENT)
}
