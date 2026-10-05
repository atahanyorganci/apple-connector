# HTTP conventions

Cross-domain behaviour of the `apple-connector` HTTP API. Route-level detail — every parameter,
response, header, and error code per operation — is in [`docs/openapi.json`](../../../../docs/openapi.json)
and [`docs/errors.md`](../../../../docs/errors.md); this file does not repeat it.

## Surface

- 55 paths, 70 operations, all under `/v1/` except `/healthz`, `/openapi.json`, and the Scalar UI at
  `/docs`.
- Read routes are `GET` (and `HEAD` for attachment content). Write routes exist only for Reminders,
  Calendar events, Contacts, and Groups ([REC-0005](../../../../docs/decisions/REC-0005-hybrid-read-write.md)).
- JSON everywhere except the format suffix routes (`/iCal`, `/caldav`, `/vcard`, `/carddav`),
  attachment content, contact photos, and `GET /v1/notes/{id}/contents` (`text/markdown`)
  ([CONN-0015](../decisions/CONN-0015-format-routes.md)).

## Values

- Timestamps are integer UTC Unix seconds in both directions
  ([REC-0010](../../../../docs/decisions/REC-0010-unix-seconds.md)).
- IDs are strings except `ChatId`, which is the Messages `chat.ROWID`. Path IDs must be non-empty;
  UUID IDs are matched case-insensitively.
- Fields Apple leaves empty are `null`, never a placeholder
  ([CONN-0019](../decisions/CONN-0019-no-synthetic-data.md)).

## Pagination and search

- Keyset only: `limit` 1–200, default 50; `page.has_more`, `page.next_cursor`
  ([CONN-0005](../decisions/CONN-0005-keyset-pagination.md)).
- Cursors are `v1.<base64url JSON>`, opaque, and bound to the filters that produced them (except
  Contacts: known bug).
- `q` is at most 256 characters. Messages, Reminders, and Notes scan at most 500 candidates per
  request and may return a short or empty page with `has_more: true`; keep following the cursor
  ([CONN-0006](../decisions/CONN-0006-bounded-search.md)).
- `/v1/containers` and `/v1/calendar-accounts` are not paginated.

## Writes

- Fields the backing framework cannot store are refused with a typed code and `details.field`
  ([REC-0012](../../../../docs/decisions/REC-0012-reject-not-coerce.md)).
- Request bodies are JSON. Malformed bodies and query strings on write routes are typed errors.
- Responses are `SyncPending*DetailDto`: 201 (create) or 200 (update) with `detail` when SQLite has
  the result; 202 with `sync_pending: true` otherwise. Deletes are 204
  ([CONN-0022](../decisions/CONN-0022-async-mutations.md)).
- `503` with a domain code when the store or framework is unavailable; `403
  eventkit_access_denied` / `contacts_access_denied` when the permission is denied.

## Errors

`{ "error": { "code", "message", "details" } }`, `code` from `ErrorCode`
([CONN-0021](../decisions/CONN-0021-error-codes.md)). Read routes still answer a malformed query
string with axum's plain-text 400 (known bug).

## Attachments and media

Served by one path for Messages, Reminders, Notes, and Calendar: GET and HEAD, byte ranges (206),
`If-None-Match` (304), unsatisfiable ranges (416), `ETag` and `Last-Modified`, a sanitized
`Content-Disposition` filename ([CONN-0008](../decisions/CONN-0008-media-serving.md)).

## Limits and timeouts

| Bound | Value | Error |
| --- | --- | --- |
| Query | 15 s | `504 query_timeout` |
| Pool acquire | 5 s | `504 query_timeout` |
| Request (JSON) | 30 s | `504 request_timeout` |
| Request (path ends in `/content`) | 300 s | `504 request_timeout` |

([CONN-0009](../decisions/CONN-0009-timeouts.md))

## Headers

Every response: `X-Content-Type-Options: nosniff`, `X-Frame-Options: DENY`,
`Referrer-Policy: no-referrer`, `Cache-Control: no-store`. No CORS headers
([CONN-0002](../decisions/CONN-0002-network-exposure.md)).
