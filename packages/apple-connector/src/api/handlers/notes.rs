use axum::{
    Json,
    extract::{Query, State},
};

use super::health::require_notes_db;
use crate::{
    api::{
        cursor::{GlobalNoteCursor, decode, decode_note_search_cursor},
        dto::{
            NoteDetailDto, NotePageDto,
            note_convert::{note_detail_to_dto, note_page_to_dto},
        },
        error::{ApiError, ErrorResponse},
        params::{NoteIdPath, NoteListParams},
        router::AppState,
    },
    db::run_timed_query,
    notes::NoteRepository,
};

/// List notes globally
#[utoipa::path(
    get,
    path = "/v1/notes",
    operation_id = "listNotes",
    tag = "notes",
    params(NoteListParams),
    responses(
        (status = 200, description = "Paginated notes", body = NotePageDto),
        (status = 400, description = "Invalid query parameters", body = ErrorResponse),
        (status = 503, description = "Notes database is unavailable", body = ErrorResponse),
    )
)]
pub async fn list_notes(
    State(state): State<AppState>,
    Query(params): Query<NoteListParams>,
) -> Result<Json<NotePageDto>, ApiError> {
    let pool = require_notes_db(&state.notes_db)?;
    let limit = params.validated_limit()?;
    params.validated_cursor()?;
    let filters = params.validated_filters()?;
    let filter_snapshot = filters.snapshot();

    let page = if filters.requires_text_scan() {
        let cursor = params
            .cursor
            .as_deref()
            .map(|value| decode_note_search_cursor(value, &filter_snapshot))
            .transpose()?;
        run_timed_query(|| async {
            NoteRepository::new(pool)
                .search_notes(&filters, limit, cursor)
                .await
        })
        .await
    } else {
        let cursor = match params.cursor.as_deref() {
            None => None,
            Some(value) if filters.is_active() => Some(
                decode_note_search_cursor(value, &filter_snapshot).map(|cursor| GlobalNoteCursor {
                    modified_at: cursor.modified_at,
                    row_id: cursor.row_id,
                })?,
            ),
            Some(value) => Some(decode::<GlobalNoteCursor>(value)?),
        };
        run_timed_query(|| async {
            NoteRepository::new(pool)
                .list_notes(&filters, limit, cursor)
                .await
        })
        .await
    }
    .map_err(|error| ApiError::internal(error.to_string()))?;

    Ok(Json(note_page_to_dto(
        page.items,
        page.has_more,
        page.next_cursor,
        limit,
    )))
}

/// Get a note
#[utoipa::path(
    get,
    path = "/v1/notes/{note_id}",
    operation_id = "getNote",
    tag = "notes",
    params(NoteIdPath),
    responses(
        (status = 200, description = "Note detail", body = NoteDetailDto),
        (status = 404, description = "Note not found", body = ErrorResponse),
        (status = 503, description = "Notes database is unavailable", body = ErrorResponse),
    )
)]
pub async fn get_note(
    State(state): State<AppState>,
    axum::extract::Path(NoteIdPath { note_id }): axum::extract::Path<NoteIdPath>,
) -> Result<Json<NoteDetailDto>, ApiError> {
    let pool = require_notes_db(&state.notes_db)?;
    let note = run_timed_query(|| async { NoteRepository::new(pool).get_note(&note_id).await })
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?
        .ok_or_else(|| ApiError::not_found(format!("note {note_id} not found")))?;

    let attachments = run_timed_query(|| async {
        NoteRepository::new(pool)
            .list_attachments_for_note(note.summary.row_id)
            .await
    })
    .await
    .map_err(|error| ApiError::internal(error.to_string()))?;

    Ok(Json(note_detail_to_dto(&note, &attachments)))
}
