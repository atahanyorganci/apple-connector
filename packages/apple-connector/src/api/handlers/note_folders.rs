use std::sync::Arc;

use axum::{
    Json,
    extract::{Query, State},
};

use super::health::{require_notes_db, validate_page};
use crate::{
    api::{
        cursor::{FolderListCursor, FolderNoteCursor, decode, decode_note_search_cursor},
        dto::{
            NoteFolderDetailDto, NoteFolderPageDto, NotePageDto,
            note_convert::{note_folder_detail_to_dto, note_folder_page_to_dto, note_page_to_dto},
        },
        error::{ApiError, ErrorResponse},
        params::{NoteFolderIdPath, NoteFolderKey, NoteListParams, PageParams},
        router::AppState,
    },
    db::run_timed_query,
    notes::{FolderLookupError, NoteRepository},
};

/// List note folders
#[utoipa::path(
    get,
    path = "/v1/note-folders",
    operation_id = "listNoteFolders",
    tag = "note-folders",
    params(PageParams),
    responses(
        (status = 200, description = "Paginated note folder summaries", body = NoteFolderPageDto),
        (status = 400, description = "Invalid pagination parameters", body = ErrorResponse),
        (status = 503, description = "Notes database is unavailable", body = ErrorResponse),
    )
)]
pub async fn list_note_folders(
    State(state): State<AppState>,
    Query(page): Query<PageParams>,
) -> Result<Json<NoteFolderPageDto>, ApiError> {
    let pool = require_notes_db(&state.notes_db)?;

    let note_entity_ids = state
        .cached_notes_entity_ids()
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;
    let limit = validate_page(&page)?;
    let cursor = page
        .validated_cursor()?
        .map(decode::<FolderListCursor>)
        .transpose()?;

    let page = run_timed_query(|| async {
        NoteRepository::with_entity_ids(pool, Arc::clone(&note_entity_ids))
            .list_folders(limit, cursor, false)
            .await
    })
    .await
    .map_err(|error| ApiError::internal(error.to_string()))?;

    Ok(Json(note_folder_page_to_dto(
        page.items,
        page.has_more,
        page.next_cursor,
        limit,
    )))
}

/// Get a note folder
#[utoipa::path(
    get,
    path = "/v1/note-folders/{folder_id}",
    operation_id = "getNoteFolder",
    tag = "note-folders",
    params(NoteFolderIdPath),
    responses(
        (status = 200, description = "Note folder detail", body = NoteFolderDetailDto),
        (status = 404, description = "Folder not found", body = ErrorResponse),
        (status = 503, description = "Notes database is unavailable", body = ErrorResponse),
    )
)]
pub async fn get_note_folder(
    State(state): State<AppState>,
    axum::extract::Path(path): axum::extract::Path<NoteFolderIdPath>,
) -> Result<Json<NoteFolderDetailDto>, ApiError> {
    let pool = require_notes_db(&state.notes_db)?;

    let note_entity_ids = state
        .cached_notes_entity_ids()
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;
    let key = path.validated_key()?;
    let folder_id = path.folder_id;
    let folder = match key {
        NoteFolderKey::Row(row_id) => run_timed_query(|| async {
            NoteRepository::with_entity_ids(pool, Arc::clone(&note_entity_ids))
                .get_folder(row_id)
                .await
        })
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?,
        NoteFolderKey::Id(id) => run_timed_query(|| async {
            NoteRepository::with_entity_ids(pool, Arc::clone(&note_entity_ids))
                .get_folder_by_id(&id)
                .await
        })
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?,
    }
    .ok_or_else(|| ApiError::not_found(format!("note folder {folder_id} not found")))?;

    Ok(Json(note_folder_detail_to_dto(&folder)))
}

/// List notes in a note folder
#[utoipa::path(
    get,
    path = "/v1/note-folders/{folder_id}/notes",
    operation_id = "listFolderNotes",
    tag = "note-folders",
    params(NoteFolderIdPath, NoteListParams),
    responses(
        (status = 200, description = "Paginated notes in folder", body = NotePageDto),
        (status = 404, description = "Folder not found", body = ErrorResponse),
        (status = 503, description = "Notes database is unavailable", body = ErrorResponse),
    )
)]
pub async fn list_folder_notes(
    State(state): State<AppState>,
    axum::extract::Path(path): axum::extract::Path<NoteFolderIdPath>,
    Query(params): Query<NoteListParams>,
) -> Result<Json<NotePageDto>, ApiError> {
    let pool = require_notes_db(&state.notes_db)?;

    let note_entity_ids = state
        .cached_notes_entity_ids()
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;
    let key = path.validated_key()?;
    let folder_id = path.folder_id;
    let folder_row_id = match &key {
        NoteFolderKey::Row(row_id) => *row_id,
        NoteFolderKey::Id(id) => {
            let folder = NoteRepository::with_entity_ids(pool, Arc::clone(&note_entity_ids))
                .get_folder_by_id(id)
                .await
                .map_err(|error| ApiError::internal(error.to_string()))?
                .ok_or_else(|| ApiError::not_found(format!("note folder {folder_id} not found")))?;
            folder.row_id.get()
        }
    };

    let limit = params.validated_limit()?;
    params.validated_cursor()?;
    let mut filters = params.validated_filters()?;
    filters.folder_id = Some(crate::notes::FolderIdFilter::RowId(folder_row_id));
    let filter_snapshot = filters.snapshot();
    let cursor = match params.cursor.as_deref() {
        None => None,
        Some(value) if filters.is_active() => Some(
            decode_note_search_cursor(value, &filter_snapshot).map(|cursor| FolderNoteCursor {
                modified_at: cursor.modified_at,
                row_id: cursor.row_id,
            })?,
        ),
        Some(value) => Some(decode::<FolderNoteCursor>(value)?),
    };

    let page = if filters.requires_text_scan() {
        let search_cursor = params
            .cursor
            .as_deref()
            .map(|value| decode_note_search_cursor(value, &filter_snapshot))
            .transpose()?;
        Ok(run_timed_query(|| async {
            NoteRepository::with_entity_ids(pool, Arc::clone(&note_entity_ids))
                .search_notes(&filters, limit, search_cursor)
                .await
        })
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?)
    } else {
        run_timed_query(|| async {
            NoteRepository::with_entity_ids(pool, Arc::clone(&note_entity_ids))
                .list_notes_in_folder(folder_row_id, &filters, limit, cursor)
                .await
        })
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?
    };

    match page {
        Ok(page) => Ok(Json(note_page_to_dto(
            page.items,
            page.has_more,
            page.next_cursor,
            limit,
        ))),
        Err(FolderLookupError::NotFound) => Err(ApiError::not_found(format!(
            "note folder {folder_id} not found"
        ))),
    }
}
