use axum::{Json, extract::{Query, State}};

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
            ReminderRepository::new(pool)
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
            ReminderRepository::new(pool)
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
    .map_err(|error| ApiError::internal(error.to_string()))?;

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
    axum::extract::Path(ReminderIdPath { reminder_id }): axum::extract::Path<ReminderIdPath>,
) -> Result<Json<ReminderDetailDto>, ApiError> {
    let pool = require_reminders_db(&state.reminders_db)?;
    let reminder = run_timed_query(|| async {
        ReminderRepository::new(pool).get_reminder(&reminder_id).await
    })
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?
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
    async fn list_and_get_reminders_from_fixture() {
        let fixture = RemindersFixtureDb::seeded().await.expect("fixture");
        let pool = connect_pool(fixture.path()).await.expect("pool");
        let app = router(AppState::new(None, Some(pool)));

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/v1/reminders?limit=10")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::OK);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/v1/reminders/bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::OK);
    }
}
