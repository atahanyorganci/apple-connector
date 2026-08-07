use std::sync::Arc;

use axum::{
    Json,
    extract::{Query, State},
    http::{HeaderValue, header},
    response::Response,
};

use super::health::require_notes_db;
use crate::{
    api::{
        cursor::{GlobalNoteCursor, decode, decode_note_search_cursor},
        dto::{
            NoteDetailDto, NotePageDto,
            note_convert::{note_detail_to_dto, note_page_to_dto},
        },
        error::{ApiError, ErrorCode, ErrorResponse},
        params::{NoteIdPath, NoteListParams},
        router::AppState,
    },
    db::run_timed_query,
    notes::{NoteRepository, preamble_from_note, render_document},
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

    let note_entity_ids = state
        .cached_notes_entity_ids()
        .await
        .map_err(ApiError::from_sqlx)?;
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
            NoteRepository::with_entity_ids(pool, Arc::clone(&note_entity_ids))
                .search_notes(&filters, limit, cursor)
                .await
        })
        .await
    } else {
        let cursor = match params.cursor.as_deref() {
            None => None,
            Some(value) if filters.is_active() => Some(
                decode_note_search_cursor(value, &filter_snapshot).map(|cursor| {
                    GlobalNoteCursor {
                        modified_at: cursor.modified_at,
                        row_id: cursor.row_id,
                    }
                })?,
            ),
            Some(value) => Some(decode::<GlobalNoteCursor>(value)?),
        };
        run_timed_query(|| async {
            NoteRepository::with_entity_ids(pool, Arc::clone(&note_entity_ids))
                .list_notes(&filters, limit, cursor)
                .await
        })
        .await
    }
    .map_err(ApiError::from_sqlx)?;

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
    axum::extract::Path(path): axum::extract::Path<NoteIdPath>,
) -> Result<Json<NoteDetailDto>, ApiError> {
    let note_id = path.validated()?;
    let pool = require_notes_db(&state.notes_db)?;

    let note_entity_ids = state
        .cached_notes_entity_ids()
        .await
        .map_err(ApiError::from_sqlx)?;
    let note = run_timed_query(|| async {
        NoteRepository::with_entity_ids(pool, Arc::clone(&note_entity_ids))
            .get_note(note_id.as_str())
            .await
    })
    .await
    .map_err(ApiError::from_sqlx)?
    .ok_or_else(|| {
        ApiError::with_details(
            ErrorCode::NoteNotFound,
            format!("note {note_id} not found"),
            serde_json::json!({ "note_id": note_id.as_str() }),
        )
    })?;

    let attachments = run_timed_query(|| async {
        NoteRepository::with_entity_ids(pool, Arc::clone(&note_entity_ids))
            .list_attachments_for_note(note.summary.row_id.get())
            .await
    })
    .await
    .map_err(ApiError::from_sqlx)?;

    Ok(Json(note_detail_to_dto(&note, &attachments)))
}

/// Get note contents as Markdown
///
/// Returns a `text/markdown` document with YAML front matter (`NoteContentsPreambleDto`)
/// followed by the note body rendered as Markdown. Hashtags appear under `tags` in the
/// preamble (leading `#` stripped). Locked notes and decode failures yield an empty body
/// while still returning preamble metadata.
#[utoipa::path(
    get,
    path = "/v1/notes/{note_id}/contents",
    operation_id = "getNoteContents",
    tag = "notes",
    params(NoteIdPath),
    responses(
        (
            status = 200,
            description = "Markdown document with YAML front matter",
            content_type = "text/markdown",
            body = String,
        ),
        (status = 404, description = "Note not found", body = ErrorResponse),
        (status = 503, description = "Notes database is unavailable", body = ErrorResponse),
    )
)]
pub async fn get_note_contents(
    State(state): State<AppState>,
    axum::extract::Path(path): axum::extract::Path<NoteIdPath>,
) -> Result<Response, ApiError> {
    let note_id = path.validated()?;
    let pool = require_notes_db(&state.notes_db)?;

    let note_entity_ids = state
        .cached_notes_entity_ids()
        .await
        .map_err(ApiError::from_sqlx)?;
    let note = run_timed_query(|| async {
        NoteRepository::with_entity_ids(pool, Arc::clone(&note_entity_ids))
            .get_note(note_id.as_str())
            .await
    })
    .await
    .map_err(ApiError::from_sqlx)?
    .ok_or_else(|| {
        ApiError::with_details(
            ErrorCode::NoteNotFound,
            format!("note {note_id} not found"),
            serde_json::json!({ "note_id": note_id.as_str() }),
        )
    })?;

    let tags = run_timed_query(|| async {
        NoteRepository::with_entity_ids(pool, Arc::clone(&note_entity_ids))
            .fetch_tags_for_note(note.summary.row_id.get())
            .await
    })
    .await
    .map_err(ApiError::from_sqlx)?;

    let preamble = preamble_from_note(&note.summary, tags);
    let document = render_document(&preamble, &note.body);

    Response::builder()
        .status(200)
        .header(
            header::CONTENT_TYPE,
            HeaderValue::from_static("text/markdown; charset=utf-8"),
        )
        .body(axum::body::Body::from(document))
        .map_err(|_| ApiError::internal("failed to build markdown response"))
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    use crate::{
        api::router::{AppState, router},
        connect_pool,
        fixtures::{NotesFixtureDb, SEED_CHECKLIST_NOTE_ID, SEED_LOCKED_NOTE_ID},
    };

    #[tokio::test]
    async fn get_note_contents_returns_markdown_with_tags() -> Result<(), Box<dyn std::error::Error>>
    {
        let fixture = NotesFixtureDb::seeded().await?;
        let pool = connect_pool(fixture.path()).await?;
        let app = router(AppState::new(None, None, Some(pool), None));

        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/v1/notes/{SEED_CHECKLIST_NOTE_ID}/contents"))
                    .body(Body::empty())?,
            )
            .await?;

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response
                .headers()
                .get("content-type")
                .and_then(|value| value.to_str().ok()),
            Some("text/markdown; charset=utf-8")
        );

        let body = response.into_body().collect().await?.to_bytes();
        let document = String::from_utf8(body.to_vec())?;

        assert!(document.starts_with("---\n"));
        assert!(document.contains("schema_version: 1"));
        assert!(document.contains("tags:"));
        assert!(document.contains("reading"));
        assert!(
            document.contains("- [ ]") || document.contains("- [x]"),
            "document: {document}"
        );
        Ok(())
    }

    #[tokio::test]
    async fn get_note_contents_locked_note_has_empty_body() -> Result<(), Box<dyn std::error::Error>>
    {
        let fixture = NotesFixtureDb::seeded().await?;
        let pool = connect_pool(fixture.path()).await?;
        let app = router(AppState::new(None, None, Some(pool), None));

        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/v1/notes/{SEED_LOCKED_NOTE_ID}/contents"))
                    .body(Body::empty())?,
            )
            .await?;

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await?.to_bytes();
        let document = String::from_utf8(body.to_vec())?;

        assert!(document.contains("is_locked: true"));
        assert!(document.trim_end().ends_with("---"));
        Ok(())
    }
}
