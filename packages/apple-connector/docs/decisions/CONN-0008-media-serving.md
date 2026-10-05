---
id: CONN-0008
status: proposed
issues:
  - https://github.com/atahanyorganci/apple-connector/issues/13
  - https://github.com/atahanyorganci/apple-connector/issues/25
  - https://github.com/atahanyorganci/apple-connector/issues/34
  - https://github.com/atahanyorganci/apple-connector/issues/46
  - https://github.com/atahanyorganci/apple-connector/issues/103
  - https://github.com/atahanyorganci/apple-connector/issues/114
provenance:
  - https://github.com/atahanyorganci/apple-connector/pull/15
  - https://github.com/atahanyorganci/apple-connector/pull/128
  - https://github.com/atahanyorganci/apple-connector/pull/139
supersedes: []
superseded-by: []
---

# Attachment bytes are served by one shared path, from database-resolved files confined to their store's attachment root.

## Question

Messages, Reminders, Notes, and Calendar all have attachment files on disk. Each domain first got
its own handler with its own Range and conditional-request behaviour (#103), and each path came
from a database column that another process writes.

## Options

- **Per-domain handlers**: four implementations of Range, ETag, and HEAD.
- **One serving path**, with per-domain path resolution in front of it.

## Decision

One serving path (`api/media.rs`, `serve_media_bytes`) for all four domains:

- The file path comes only from the database row for the requested id, never from the request.
- It is canonicalized and must stay inside the canonicalized attachment root for that store;
  symlinks that leave the root and non-files are refused.
- Bytes are served by `tower_http::services::ServeFile`: GET, HEAD, byte ranges (206), and
  `416 byte_range_not_satisfiable`. ETag and Last-Modified validators are set; `If-None-Match`
  answers 304.
- Content type, a sanitized download filename, and `nosniff` are set on every response.
- Filesystem checks run on Tokio's blocking pool (`BlockingIoPool`), not on async workers.

## Consequences

- A missing or incomplete file is the domain's `*_attachment_unavailable` code, not a 500.
- Contact photos come from a database blob, not a file, and are served by their own handler.
- Request timeouts end when the response head is returned, so no download is cut off by them
  ([CONN-0009](CONN-0009-timeouts.md)).

## Evidence

- `packages/apple-connector/src/api/media.rs`; callers in `api/handlers/attachments.rs`,
  `reminder_attachments.rs`, `note_attachments.rs`, `event_attachments.rs`.
- `packages/apple-connector/src/messages/attachment_path.rs` `validate_attachment_path`;
  `notes/attachment_path.rs` additionally confines to `Accounts/<account>/`.
- `packages/apple-connector/src/api/blocking_io.rs`: test
  `unrelated_async_work_is_not_stalled_by_blocking_io`.
