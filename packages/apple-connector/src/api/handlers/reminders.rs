use std::sync::Arc;

use axum::{
    Json,
    extract::{Query, State},
};

use super::health::require_reminders_db;
use crate::{
    api::{
        cursor::{GlobalReminderCursor, decode, decode_reminder_search_cursor},
        dto::{
            ReminderDetailDto, ReminderPageDto,
            reminder_convert::{reminder_detail_to_dto, reminder_page_to_dto},
        },
        error::{ApiError, ErrorResponse},
        params::{ReminderIdPath, ReminderListParams},
        router::AppState,
    },
    db::run_timed_query,
    reminders::ReminderRepository,
};

/// List reminders globally
#[utoipa::path(
    get,
    path = "/v1/reminders",
    operation_id = "listReminders",
    tag = "reminders",
    params(ReminderListParams),
    responses(
        (status = 200, description = "Paginated reminders", body = ReminderPageDto),
        (status = 400, description = "Invalid query parameters", body = ErrorResponse),
        (status = 503, description = "Reminders database is unavailable", body = ErrorResponse),
    )
)]
pub async fn list_reminders(
    State(state): State<AppState>,
    Query(params): Query<ReminderListParams>,
) -> Result<Json<ReminderPageDto>, ApiError> {
    let pool = require_reminders_db(&state.reminders_db)?;

    let reminder_entity_ids = state
        .cached_reminders_entity_ids()
        .await
        .map_err(ApiError::from_sqlx)?;
    let limit = params.validated_limit()?;
    params.validated_cursor()?;
    let filters = params.validated_filters()?;
    let filter_snapshot = filters.snapshot();
    let include_subtasks = params.include_subtasks.unwrap_or(false);
    let include_tags = params.include_tags.unwrap_or(false);

    let page = if filters.requires_text_scan() {
        let cursor = params
            .cursor
            .as_deref()
            .map(|value| decode_reminder_search_cursor(value, &filter_snapshot))
            .transpose()?;
        run_timed_query(|| async {
            ReminderRepository::with_entity_ids(pool, Arc::clone(&reminder_entity_ids))
                .search_reminders(&filters, limit, cursor, include_subtasks, include_tags)
                .await
        })
        .await
    } else {
        let cursor = match params.cursor.as_deref() {
            None => None,
            Some(value) if filters.is_active() => Some(
                decode_reminder_search_cursor(value, &filter_snapshot).map(|cursor| {
                    GlobalReminderCursor {
                        modified_at: cursor.modified_at,
                        row_id: cursor.row_id,
                    }
                })?,
            ),
            Some(value) => Some(decode::<GlobalReminderCursor>(value)?),
        };
        run_timed_query(|| async {
            ReminderRepository::with_entity_ids(pool, Arc::clone(&reminder_entity_ids))
                .list_reminders(
                    &filters,
                    limit,
                    cursor,
                    include_subtasks,
                    include_tags,
                    None,
                )
                .await
        })
        .await
    }
    .map_err(ApiError::from_sqlx)?;

    Ok(Json(reminder_page_to_dto(
        page.items,
        page.has_more,
        page.next_cursor,
        limit,
    )))
}

/// Get a reminder
#[utoipa::path(
    get,
    path = "/v1/reminders/{reminder_id}",
    operation_id = "getReminder",
    tag = "reminders",
    params(ReminderIdPath),
    responses(
        (status = 200, description = "Reminder detail", body = ReminderDetailDto),
        (status = 404, description = "Reminder not found", body = ErrorResponse),
        (status = 503, description = "Reminders database is unavailable", body = ErrorResponse),
    )
)]
pub async fn get_reminder(
    State(state): State<AppState>,
    axum::extract::Path(path): axum::extract::Path<ReminderIdPath>,
) -> Result<Json<ReminderDetailDto>, ApiError> {
    let reminder_id = path.validated()?;
    let pool = require_reminders_db(&state.reminders_db)?;

    let reminder_entity_ids = state
        .cached_reminders_entity_ids()
        .await
        .map_err(ApiError::from_sqlx)?;
    let reminder = run_timed_query(|| async {
        ReminderRepository::with_entity_ids(pool, Arc::clone(&reminder_entity_ids))
            .get_reminder(reminder_id.as_str())
            .await
    })
    .await
    .map_err(ApiError::from_sqlx)?
    .ok_or_else(|| ApiError::not_found(format!("reminder {reminder_id} not found")))?;

    Ok(Json(reminder_detail_to_dto(&reminder)))
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use http::{Request, StatusCode};
    use tower::ServiceExt;

    use crate::{
        api::router::{AppState, router},
        connect_pool,
        fixtures::RemindersFixtureDb,
    };

    #[tokio::test]
    async fn list_and_get_reminders_from_fixture() -> Result<(), Box<dyn std::error::Error>> {
        let fixture = RemindersFixtureDb::seeded().await?;
        let pool = connect_pool(fixture.path()).await?;
        let app = router(AppState::new(None, Some(pool), None, None));

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/v1/reminders?limit=10")
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(response.status(), StatusCode::OK);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/v1/reminders/bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb")
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(response.status(), StatusCode::OK);
        Ok(())
    }
}
