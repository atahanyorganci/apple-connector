use std::sync::Arc;

use axum::{
    Json,
    extract::{Query, State},
};

use super::health::{require_reminders_db, validate_page};
use crate::{
    api::{
        cursor::{ListCursor, ListReminderCursor, decode, decode_reminder_search_cursor},
        dto::{
            ReminderListDetailDto, ReminderListPageDto, ReminderPageDto,
            reminder_convert::{
                reminder_list_detail_to_dto, reminder_list_page_to_dto, reminder_page_to_dto,
            },
        },
        error::{ApiError, ErrorResponse},
        params::{PageParams, ReminderListIdPath, ReminderListKey, ReminderListParams},
        router::AppState,
    },
    db::run_timed_query,
    reminders::{ListLookupError, ReminderRepository},
};

/// List reminder lists
#[utoipa::path(
    get,
    path = "/v1/reminder-lists",
    operation_id = "listReminderLists",
    tag = "reminder-lists",
    params(PageParams),
    responses(
        (status = 200, description = "Paginated reminder list summaries", body = ReminderListPageDto),
        (status = 400, description = "Invalid pagination parameters", body = ErrorResponse),
        (status = 503, description = "Reminders database is unavailable", body = ErrorResponse),
    )
)]
pub async fn list_reminder_lists(
    State(state): State<AppState>,
    Query(page): Query<PageParams>,
) -> Result<Json<ReminderListPageDto>, ApiError> {
    let pool = require_reminders_db(&state.reminders_db)?;
    let reminder_entity_ids = state
        .cached_reminders_entity_ids()
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;
    let limit = validate_page(&page)?;
    let cursor = page
        .validated_cursor()?
        .map(decode::<ListCursor>)
        .transpose()?;

    let repo = ReminderRepository::with_entity_ids(pool, Arc::clone(&reminder_entity_ids));
    let page = run_timed_query(|| async { repo.list_lists(limit, cursor).await })
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;

    Ok(Json(reminder_list_page_to_dto(
        page.items,
        page.has_more,
        page.next_cursor,
        limit,
    )))
}

/// Get a reminder list
#[utoipa::path(
    get,
    path = "/v1/reminder-lists/{list_id}",
    operation_id = "getReminderList",
    tag = "reminder-lists",
    params(ReminderListIdPath),
    responses(
        (status = 200, description = "Reminder list detail", body = ReminderListDetailDto),
        (status = 404, description = "List not found", body = ErrorResponse),
        (status = 503, description = "Reminders database is unavailable", body = ErrorResponse),
    )
)]
pub async fn get_reminder_list(
    State(state): State<AppState>,
    axum::extract::Path(path): axum::extract::Path<ReminderListIdPath>,
) -> Result<Json<ReminderListDetailDto>, ApiError> {
    let pool = require_reminders_db(&state.reminders_db)?;
    let reminder_entity_ids = state
        .cached_reminders_entity_ids()
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;
    let list_id = path.list_id.clone();
    let key = path.validated_key()?;
    let repo = ReminderRepository::with_entity_ids(pool, Arc::clone(&reminder_entity_ids));
    let list = run_timed_query(|| async { repo.get_list_by_key(&key).await })
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?
        .ok_or_else(|| ApiError::not_found(format!("reminder list {list_id} not found")))?;

    Ok(Json(reminder_list_detail_to_dto(&list)))
}

/// List reminders in a reminder list
#[utoipa::path(
    get,
    path = "/v1/reminder-lists/{list_id}/reminders",
    operation_id = "listReminderListReminders",
    tag = "reminder-lists",
    params(ReminderListIdPath, ReminderListParams),
    responses(
        (status = 200, description = "Paginated reminders in list", body = ReminderPageDto),
        (status = 404, description = "List not found", body = ErrorResponse),
        (status = 503, description = "Reminders database is unavailable", body = ErrorResponse),
    )
)]
pub async fn list_reminder_list_reminders(
    State(state): State<AppState>,
    axum::extract::Path(path): axum::extract::Path<ReminderListIdPath>,
    Query(params): Query<ReminderListParams>,
) -> Result<Json<ReminderPageDto>, ApiError> {
    let pool = require_reminders_db(&state.reminders_db)?;
    let reminder_entity_ids = state
        .cached_reminders_entity_ids()
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;
    let list_id = path.list_id.clone();
    let key = path.validated_key()?;
    let list_row_id = match &key {
        ReminderListKey::Row(row_id) => *row_id,
        ReminderListKey::Id(id) => {
            let repo = state
                .reminder_repository(pool)
                .await
                .map_err(|error| ApiError::internal(error.to_string()))?;
            let list = repo
                .get_list_by_uuid(id)
                .await
                .map_err(|error| ApiError::internal(error.to_string()))?
                .ok_or_else(|| ApiError::not_found(format!("reminder list {list_id} not found")))?;
            list.row_id.get()
        }
    };

    let limit = params.validated_limit()?;
    params.validated_cursor()?;
    let mut filters = params.validated_filters()?;
    filters.list_id = Some(crate::reminders::ListIdFilter::RowId(list_row_id));
    let filter_snapshot = filters.snapshot();
    let include_subtasks = params.include_subtasks.unwrap_or(false);
    let include_tags = params.include_tags.unwrap_or(false);
    let cursor = match params.cursor.as_deref() {
        None => None,
        Some(value) if filters.is_active() => Some(
            decode_reminder_search_cursor(value, &filter_snapshot).map(|cursor| {
                ListReminderCursor {
                    modified_at: cursor.modified_at,
                    row_id: cursor.row_id,
                }
            })?,
        ),
        Some(value) => Some(decode::<ListReminderCursor>(value)?),
    };

    let repo = ReminderRepository::with_entity_ids(pool, Arc::clone(&reminder_entity_ids));
    let page = run_timed_query(|| async {
        repo.list_list_reminders(
            list_row_id,
            &filters,
            limit,
            cursor,
            include_subtasks,
            include_tags,
        )
        .await
    })
    .await
    .map_err(|error| ApiError::internal(error.to_string()))?;

    match page {
        Ok(page) => Ok(Json(reminder_page_to_dto(
            page.items,
            page.has_more,
            page.next_cursor,
            limit,
        ))),
        Err(ListLookupError::NotFound) => Err(ApiError::not_found(format!(
            "reminder list {list_id} not found"
        ))),
    }
}
