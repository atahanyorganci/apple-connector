# `apple-connector`: HTTP API over Apple Messages, Reminders, Notes, Calendar, and Contacts

A local HTTP server that reads the live SQLite stores of five Apple apps and writes Reminders,
Calendar, and Contacts through Apple's frameworks. Clients are local tools (curl, the Raycast
extension) or anything behind a reverse proxy the operator provides.

## Kind

Application.

## Public API

- **Binaries**
  - `apple-connector`: the server.
  - `export-openapi <output-path>`: writes the OpenAPI document without opening any database.
- **HTTP**: 55 paths, 70 operations. The contract is [`docs/openapi.json`](../../../docs/openapi.json)
  (also served at `/openapi.json`, rendered at `/docs`); error codes are in
  [`docs/errors.md`](../../../docs/errors.md). Cross-domain conventions:
  [`spec/http.md`](spec/http.md).
- **Library** (`apple_connector`): `run(Cli)`, `router(AppState)`, `AppState`,
  `build_openapi_spec()`, `connect_pool`, the `apple_types` value types, test `fixtures`, and the
  Messages loader (`load_all`, `load_chats`, `MessageInventory`).

## Guarantees

- **Read-only stores.** Every SQLite pool is opened `read_only`, `create_if_missing(false)`;
  writes go only through EventKit and Contacts
  ([REC-0005](../../../docs/decisions/REC-0005-hybrid-read-write.md)). Enforced by: `src/db.rs`
  `connect_pool`; no write route exists for Messages or Notes (`docs/openapi.json`).
- **Network.** Binds `127.0.0.1:3000` by default; only loopback addresses and `0.0.0.0` are
  accepted; no auth, TLS, or CORS; fixed security headers
  ([CONN-0002](decisions/CONN-0002-network-exposure.md)). Enforced by: `src/cli.rs` tests,
  `src/api/middleware.rs::security_headers_are_applied`.
- **Privacy.** Request logs carry route template, status, and latency only; no response carries a
  file path, raw blob, or locked-note content ([CONN-0003](decisions/CONN-0003-privacy-boundary.md)).
  Enforced by: `trace_middleware_does_not_log_request_uri`, `locked_notes_never_decode_body`,
  `get_note_contents_locked_note_has_empty_body`.
- **Contract.** Routes are registered through the OpenAPI router, and the committed document
  matches the code ([CONN-0004](decisions/CONN-0004-openapi-contract.md)). Enforced by:
  `src/api/doc.rs::exported_openapi_matches_committed_contract`.
- **Timestamps** are integer Unix seconds everywhere
  ([REC-0010](../../../docs/decisions/REC-0010-unix-seconds.md)). Enforced by: the `UnixTimestamp`
  schema; no `date-time` format in `docs/openapi.json`.
- **Pagination.** Keyset only; `limit` 1–200, default 50; cursors versioned and filter-bound
  ([CONN-0005](decisions/CONN-0005-keyset-pagination.md)). Enforced by: `src/api/params.rs`
  `default_limit_is_50_and_max_is_200`; `tests/calendar_integration.rs`
  `integration_calendar_filtered_listings_page_each_event_once`.
- **Search** over decoded content is a bounded, resumable scan
  ([CONN-0006](decisions/CONN-0006-bounded-search.md)). Enforced by: the scan-budget constants and
  the search tests in `src/messages`, `src/notes`, `src/reminders`.
- **Partial data.** A blob that fails to decode is marked on its record and never fails the page
  ([CONN-0001](decisions/CONN-0001-best-effort-decoding.md)); missing values are `null` and list
  endpoints never drop rows ([CONN-0019](decisions/CONN-0019-no-synthetic-data.md)). Enforced by:
  `tests/contacts_integration.rs::integration_contacts_with_null_container_are_listed`.
- **Startup.** Messages is required (`tests/spec.rs::a_missing_messages_database_aborts_startup`);
  other stores degrade to `503 <domain>_database_unavailable`; Reminders, Notes, and Contacts
  schema metadata must load or startup stops
  ([CONN-0010](decisions/CONN-0010-store-discovery.md),
  [CONN-0020](decisions/CONN-0020-schema-fail-fast.md)). Enforced by: `src/api/router.rs`
  `warm_entity_id_caches` tests; `src/api/handlers/health.rs` tests.
- **Errors** are `{ "error": { code, message, details } }` with a documented `ErrorCode`; database
  errors never carry driver text ([CONN-0021](decisions/CONN-0021-error-codes.md)). Enforced by:
  `scripts/check-api-error-leakage.sh` (flake check), `src/api/error.rs` tests, the OpenAPI
  contract tests.
- **Writes** reject what the framework cannot store
  ([REC-0012](../../../docs/decisions/REC-0012-reject-not-coerce.md)) and answer 201/200 with the
  detail or 202 `sync_pending` ([CONN-0022](decisions/CONN-0022-async-mutations.md)). Enforced by:
  `tests/mutations_integration.rs`, `tests/spec.rs::flagged_false_is_still_an_unsupported_reminder_field`,
  `src/api/hydrate.rs::mutation_status_returns_accepted_when_sync_pending`.
- **Event identifiers.** `EventId` is `lower(CalendarItem.UUID)`; writes translate it to EventKit's
  identifiers ([CONN-0023](decisions/CONN-0023-event-identifiers.md)). Enforced by (live, ignored):
  `tests/eventkit_integration.rs::http_created_event_id_resolves_through_get`,
  `http_listed_event_id_works_for_patch_and_delete`.
- **Occurrences.** Range listings read `OccurrenceCache`, one row per occurrence
  ([CONN-0013](decisions/CONN-0013-occurrence-cache.md)). Enforced by:
  `tests/calendar_integration.rs` range tests.
- **Attachments** are served from database-resolved paths confined to their store's root, with
  Range, HEAD, and conditional requests ([CONN-0008](decisions/CONN-0008-media-serving.md)).
  Enforced by: attachment handler tests in `src/api/handlers/*attachments.rs`.
- **Bounds.** Query 15 s, JSON request 30 s, media request 300 s, each a 504 with its own code
  ([CONN-0009](decisions/CONN-0009-timeouts.md)). Enforced by: `src/api/error.rs` tests.
- **SQL** is compile-time checked ([CONN-0018](decisions/CONN-0018-compile-time-sql.md)). Enforced
  by: `SQLX_OFFLINE` builds and `scripts/check-runtime-sql.sh`.

### Not yet enforced

- The runtime-SQL guard does not scan `tests/`, so integration tests use runtime SQL (#71).
- Nothing tests the request timeouts themselves; only their error mapping is tested.
- Live behaviour (EventKit and Contacts writes, real stores) is covered only by `#[ignore]` tests
  run by hand: `tests/eventkit_integration.rs`, `tests/contacts_integration.rs`,
  `tests/integration.rs`.

### Known bugs

- **Coarse error codes.** Framework outcomes still answer `resource_not_found`,
  `unprocessable_entity`, and `gateway_timeout`, and body rejections `validation_error`, which
  #130 removed. Proven by: `framework_errors_map_to_granular_codes` in
  `src/api/eventkit_convert.rs` and `src/api/contacts_convert.rs` (ignored, fail today).
- **Framework text in responses.** EventKit and Contacts `ValidationFailed` carry
  `NSError.localizedDescription`, which becomes the 422 `message`. Proven by:
  `framework_validation_text_is_not_returned_to_clients` in both files (ignored, fail today).
- **Read endpoints answer malformed query strings outside the envelope** (axum's plain-text 400)
  ([CONN-L-0005](lessons/CONN-L-0005-extractor-rejections.md)). Proven by:
  `tests/spec.rs::an_rfc3339_query_bound_is_a_typed_error` (ignored, fails today).
- **Contacts cursors are not bound to their filters.** Proven by:
  `tests/spec.rs::contact_cursors_are_bound_to_their_filters` (ignored, fails today).
- **Legacy Calendar schemas pass startup** and `/healthz`, then fail each query; #99 required a
  startup failure. Proven by: `tests/spec.rs::a_legacy_calendar_schema_fails_the_startup_gate`
  (ignored, fails today).
- **Dates before 2001-01-01 are read as `null`** for every Core Data timestamp — contact
  birthdays, calendar events, reminders, notes — because a value `<= 0` is treated as unset.
  Proven by: `tests/spec.rs::core_data_dates_before_2001_are_kept` (ignored, fails today).
- **Calendar writes resolve the calendar by title**, not by the `calendar_id` given, so two
  calendars with the same title answer `409 ambiguous_event_kit_match` (`apple-eventkit` known
  bug). Proven by: `apple-eventkit/tests/identifier_probe.rs` (live).
- **The contact photo route treats `%` and `_` in the id as wildcards**, so
  `GET /v1/contacts/%25/photo` returns some contact's photo. Proven by:
  `tests/spec.rs::a_photo_is_only_served_for_the_exact_contact_id` (ignored, fails today).
- **Event attachments get the JSON timeout.** The media timeout is chosen by a `/content` path
  suffix, which `/v1/events/{id}/attachments/{attachment_id}` does not have. Proven by: code
  (`src/api/middleware.rs` `request_timeout`).
- **`GET /v1/contacts/{id}/photo` documents no response content type** in OpenAPI, so generated
  clients type it as `void`. Proven by: `docs/openapi.json`.

## Limits and non-goals

- Messages and Notes are read-only. There is no sending, and locked notes are never decrypted.
- Write bodies are JSON only; vCard and CardDAV request bodies (#56, #63) were never implemented
  ([CONN-0015](decisions/CONN-0015-format-routes.md)).
- No `Accept` negotiation: formats are separate routes.
- ID validation checks only that an id is non-empty.
- `/healthz` is 503 unless all five stores are open; it is a readiness report.
- Recurring occurrences exist only as far as Calendar has populated `OccurrenceCache`.
- Store mappings and their limits: [`spec/messages.md`](spec/messages.md),
  [`spec/reminders.md`](spec/reminders.md), [`spec/notes.md`](spec/notes.md),
  [`spec/calendar.md`](spec/calendar.md), [`spec/contacts.md`](spec/contacts.md).

## Platform and permissions

macOS only (it depends on `apple-eventkit` and `apple-contacts`). Full Disk Access for the SQLite
reads; Reminders, Calendars, and Contacts TCC grants for writes, requested at startup. Without a
grant, the corresponding writes answer `403 eventkit_access_denied` or `contacts_access_denied`.

## Application

| Flag | Environment | Default |
| --- | --- | --- |
| `--address` | — | `127.0.0.1` |
| `--port` | — | `3000` |
| `--messages-database` | `APPLE_CONNECTOR_MESSAGES_DATABASE` (`APPLE_CONNECTOR_DATABASE`, deprecated) | `~/Library/Messages/chat.db` |
| `--attachment-root` | — | `Attachments` next to `chat.db` |
| `--reminders-database` | `APPLE_CONNECTOR_REMINDERS_DATABASE` | discovered |
| `--reminders-stores-dir` | — | `~/Library/Group Containers/group.com.apple.reminders/Container_v1/Stores` |
| `--reminders-attachment-root` | — | the store's `.Data-<uuid>_SUPPORT` sibling |
| `--notes-database` | `APPLE_CONNECTOR_NOTES_DATABASE` | `~/Library/Group Containers/group.com.apple.notes/NoteStore.sqlite` |
| `--notes-attachment-root` | — | `~/Library/Group Containers/group.com.apple.notes/Accounts`, whatever `--notes-database` is |
| `--calendar-database` | `APPLE_CONNECTOR_CALENDAR_DATABASE` | `~/Library/Group Containers/group.com.apple.calendar/Calendar.sqlitedb` |
| `--calendar-attachment-root` | — | `~/Library/Group Containers/group.com.apple.calendar/Attachments`, whatever `--calendar-database` is |
| `--contacts-sources-dir` | `APPLE_CONNECTOR_CONTACTS_SOURCES_DIR` | `~/Library/Application Support/AddressBook/Sources` |

| Domain | Read source | Write path |
| --- | --- | --- |
| Messages | `chat.db` | none |
| Reminders | Reminders Core Data store | `apple-eventkit` |
| Notes | `NoteStore.sqlite` | none |
| Calendar | `Calendar.sqlitedb` | `apple-eventkit` |
| Contacts | AddressBook source databases | `apple-contacts` |

Depends on `apple-eventkit`, `apple-contacts`, `apple-typedstream`, `apple-notes-protobuf`,
`serde-icalendar`, `serde-caldav`, `serde-vcard`, and `serde-carddav`.
