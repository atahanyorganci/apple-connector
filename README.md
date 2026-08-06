# Apple Connector

Monorepo for reading and writing Apple platform data on macOS. The
[`packages/apple-connector`](packages/apple-connector/) crate exposes a hybrid
HTTP API over Messages, Reminders, Notes, Calendar, and Contacts. Reads use live
SQLite databases; Reminders, Calendar, and Contacts writes go through EventKit
and the Contacts framework.

See [`AGENTS.md`](AGENTS.md) for crate layout and contributor conventions.

## Requirements

- Apple Silicon Mac (`aarch64-darwin`)
- Nix with flakes enabled
- **Full Disk Access** for the terminal (Messages, Reminders, Notes, Calendar Group Containers, Contacts AddressBook)
- **Reminders**, **Calendars**, and **Contacts** permission for write routes (EventKit / Contacts framework)

Grant access in **System Settings → Privacy & Security → Full Disk Access**, then
restart the terminal. If macOS still denies access when running the compiled
binary directly, add `target/debug/apple-connector` there as well.

## Quick start

```bash
nix develop
cargo run -p apple-connector
```

By default the server binds to `127.0.0.1:3000`, opens
`~/Library/Messages/chat.db`, auto-discovers the Reminders store, opens the Notes
Group Container `NoteStore.sqlite` read-only, and serves the API documented at
`/openapi.json`. Browse and try endpoints interactively at `/docs`.

```bash
curl -s http://127.0.0.1:3000/healthz
curl -s 'http://127.0.0.1:3000/v1/messages?limit=5'
curl -s http://127.0.0.1:3000/openapi.json | jq .info.title
```

## CLI options

| Option                               | Default                                  | Description                                                                          |
| ------------------------------------ | ---------------------------------------- | ------------------------------------------------------------------------------------ |
| `--address <IP>`                     | `127.0.0.1`                              | Bind address. Loopback (`127.0.0.1`, `127.0.0.2`, `::1`) or explicit `0.0.0.0` only. |
| `--port <PORT>`                      | `3000`                                   | TCP port (`1`–`65535`; `0` is rejected).                                             |
| `--messages-database <PATH>`         | `~/Library/Messages/chat.db`             | Read-only path to Messages `chat.db`. The server never creates or mutates this file. |
| `--reminders-database <PATH>`        | auto-discover                            | Read-only path to the Reminders SQLite store.                                        |
| `--reminders-stores-dir <PATH>`      | Group Containers `Stores/`               | Directory scanned when auto-discovering the Reminders store.                         |
| `--attachment-root <PATH>`           | `<messages-database-dir>/Attachments`    | Messages attachment directory (canonicalized and confined at runtime).               |
| `--reminders-attachment-root <PATH>` | `.Data-{UUID}_SUPPORT` next to store     | Reminders attachment support directory.                                              |
| `--notes-database <PATH>`            | Notes Group Container `NoteStore.sqlite` | Read-only path to the Notes SQLite store.                                            |
| `--notes-attachment-root <PATH>`     | `Accounts/` under Notes Group Container  | Notes attachment directory (canonicalized and confined at runtime).                  |
| `--help`                             | —                                        | Print usage and exit.                                                                |
| `--version`                          | —                                        | Print crate version and exit.                                                        |

Examples:

```bash
cargo run -p apple-connector -- --port 8080
cargo run -p apple-connector -- --messages-database /path/to/chat.db
cargo run -p apple-connector -- --reminders-database /path/to/Data-*.sqlite
cargo run -p apple-connector -- --address 0.0.0.0 --port 3000
```

## Deployment and network exposure

The API is **unauthenticated** and does **not** terminate TLS. The default
`127.0.0.1` binding is intended for local development and trusted clients on
the same machine.

`--address 0.0.0.0` binds all interfaces. Only use this when the host is
protected by a **reverse proxy or firewall** that provides:

- TLS termination
- Authentication and authorization
- Network access controls (allowlists, VPN, private subnets)

The server logs a warning when `0.0.0.0` is selected. There is **no permissive
CORS**; browser clients must be same-origin or fronted by your proxy.

Press **Ctrl-C** to shut down. In-flight requests are drained before exit.

## Permissions

- **Full Disk Access** opens `chat.db`, Reminders/Notes/Calendar Group Container
  stores, Contacts AddressBook sources, and attachment directories.
- **Reminders**, **Calendars**, and **Contacts** TCC grants are required for
  write routes (create/update/delete reminders, events, contacts, and groups).
- If a read database cannot be opened, `/healthz` returns `503` with per-service
  status (`messages`, `reminders`, `notes`, `calendar`, `contacts`) and write
  availability (`eventkit_reminders`, `eventkit_events`, `contacts_auth`).
  Data endpoints return structured `service_unavailable` errors. Startup logs do
  **not** include filesystem paths.

## HTTP endpoints

Interactive reference: [`/docs`](http://127.0.0.1:3000/docs) (Scalar UI) and
[`/openapi.json`](http://127.0.0.1:3000/openapi.json). Committed contract:
[`docs/openapi.json`](docs/openapi.json).

| Domain | Read (SQLite) | Write (framework) |
| --- | --- | --- |
| Messages | chats, messages, attachments | — |
| Reminders | lists, reminders, attachments | create/update/delete reminders |
| Notes | folders, notes, attachments | — |
| Calendar | accounts, calendars, events, iCal/CalDAV export | create/update/delete events |
| Contacts | containers, groups, contacts, vCard/CardDAV | create/update/delete contacts and groups |

Common routes: `GET /healthz`, `GET /openapi.json`, `GET /docs`. List endpoints
use keyset pagination (`limit`, `cursor`). Unknown routes return JSON `404`;
unsupported methods return JSON `405`.
responses include `X-Content-Type-Options: nosniff`, `X-Frame-Options: DENY`,
`Referrer-Policy: no-referrer`, and `Cache-Control: no-store`.

## Response data types

Identifiers and timestamps use centralized value types (the `apple_types`
module) shared across the OpenAPI contract:

- **Timestamps** (`sent_at`, `read_at`, `due.at`, `last_modified_at`, …) are
  `UnixTimestamp`: whole **seconds since the Unix epoch (UTC)**, serialized as
  JSON integers.
- **Identifiers** (`MessageId`, `AttachmentId`, `ReminderId`, `ReminderListId`,
  `SectionId`, `ReminderAttachmentId`, `NoteId`, `NoteFolderId`,
  `NoteAttachmentId`, and the integer-backed `ChatId`) are typed wrappers that
  serialize transparently as their underlying string (or integer).

> **Breaking change:** response timestamps were previously emitted as RFC 3339
> strings (for example `"2024-01-15T12:00:00Z"`). They are now Unix-seconds
> integers (for example `1705320000`). Clients that parsed the string form must
> be updated. Query-parameter bounds (`before`, `after`, `due_before`,
> `due_after`) still accept ISO-8601 / RFC 3339 input.

## Pagination and cursors

List endpoints use **keyset pagination only** (no offsets):

- Default `limit`: **50**
- Maximum `limit`: **200**
- Order: **newest first**
- Continuation: pass `cursor` from `page.next_cursor` when `page.has_more` is
  `true`
- Cursor format: `v1.` prefix followed by a URL-safe base64 JSON payload

Filtered `/v1/messages` requests bind cursors to the active filter set. Reusing
a cursor with different filters returns `400 validation_error`.

## Search and filters

`GET /v1/messages` supports:

| Parameter         | Description                                                                                      |
| ----------------- | ------------------------------------------------------------------------------------------------ |
| `q`               | Case-insensitive text search (max 256 chars). Matches plain `text` and decoded `attributedBody`. |
| `chat_id`         | Restrict to one chat row id.                                                                     |
| `sender`          | Handle identifier (phone/email).                                                                 |
| `before`, `after` | ISO-8601 UTC bounds on message time.                                                             |
| `direction`       | `sent` or `received`.                                                                            |
| `transport`       | `imessage`, `sms`, `rcs`, or `unknown`.                                                          |
| `content_type`    | Classified content kind (`text`, `attachment`, `reaction`, …).                                   |
| `has_attachments` | `true` or `false`.                                                                               |

Text search uses a resumable search cursor when filters are active. New rows
written to `chat.db` by Messages.app are visible on the next request without
restarting the server.

## Reminders search and filters

`GET /v1/reminders` and `GET /v1/reminder-lists/{list_id}/reminders` support:

| Parameter                                 | Description                                                      |
| ----------------------------------------- | ---------------------------------------------------------------- |
| `q`                                       | Case-insensitive text search (max 256 chars) on title and notes. |
| `completed`, `flagged`                    | Boolean completion/flag filters.                                 |
| `has_due_date`, `due_before`, `due_after` | Due-date presence and bounds.                                    |
| `priority_min`                            | Minimum Reminders priority value.                                |
| `has_notes`, `top_level_only`             | Notes presence and parent-only listing.                          |
| `section_id`                              | Restrict to one section UUID.                                    |
| `include_subtasks`, `include_tags`        | Expand nested subtasks and hashtag tags.                         |

Filtered requests bind cursors to the active filter set. Reusing a cursor with
different filters returns `400 validation_error`.

## Notes search and filters

`GET /v1/notes` and `GET /v1/note-folders/{folder_id}/notes` support:

| Parameter                                                    | Description                                                                       |
| ------------------------------------------------------------ | --------------------------------------------------------------------------------- |
| `q`                                                          | Case-insensitive text search (max 256 chars) on title, snippet, and decoded body. |
| `folder_id`                                                  | Restrict to one folder (row id or UUID).                                          |
| `is_pinned`, `is_locked`, `has_checklist`, `has_attachments` | Boolean metadata filters.                                                         |
| `include_deleted`                                            | Include Recently Deleted notes.                                                   |
| `modified_before`, `modified_after`                          | ISO-8601 UTC bounds on modification time.                                         |

Filtered requests bind cursors to the active filter set. Reusing a cursor with
different filters returns `400 validation_error`.

## Privacy and logging

- API DTOs omit local filesystem paths, raw balloon payloads, and opaque binary
  fields.
- **Locked notes** (`is_locked: true`) never return decoded body text or
  ciphertext; the snippet may still be present. The server does not attempt
  password cracking.
- Structured request logs include **route template**, **HTTP status**, and
  **latency** only. They never include query strings, handles, message bodies,
  attachment payloads, or request paths with identifiers.
- JSON handlers time out after 30 seconds; attachment streaming allows up to 5
  minutes. SQLite pool acquire and busy timeouts are 5 seconds; query work is
  bounded at 15 seconds.

## Development

### Fixtures

An empty Messages schema lives in
[`packages/apple-connector/fixtures/messages/`](packages/apple-connector/fixtures/messages/).
Use it for local development without reading your real `chat.db`.

A matching Reminders schema and seeded fixture live in
[`packages/apple-connector/fixtures/reminders/`](packages/apple-connector/fixtures/reminders/).

A matching Notes schema and seeded fixture live in
[`packages/apple-connector/fixtures/notes/`](packages/apple-connector/fixtures/notes/).

```bash
./packages/apple-connector/fixtures/messages/create-empty-db.sh
./packages/apple-connector/fixtures/reminders/create-empty-db.sh
./packages/apple-connector/fixtures/notes/create-empty-db.sh
./packages/apple-connector/fixtures/calendar/create-empty-db.sh
./packages/apple-connector/fixtures/contacts/create-empty-db.sh
bash scripts/sqlx-prepare-all.sh
```

The prepare script runs once per domain fixture (Messages, Reminders, Notes,
Calendar, Contacts) and merges query metadata into `packages/apple-connector/sqlx/`.
Apple's five SQLite schemas cannot share one database, so offline verification
uses sequential passes with a growing cache (`SQLX_OFFLINE=true`).

Commit the updated `packages/apple-connector/sqlx/` directory. Nix builds use
`SQLX_OFFLINE=true` with `SQLX_OFFLINE_DIR=packages/apple-connector/sqlx` and do
not need a database connection.

### SQL queries

Queries in `packages/apple-connector/src/` are verified at compile time with SQLx
`query!`, `query_as!`, and `query_scalar!`. After changing SQL, run
`bash scripts/sqlx-prepare-all.sh` and commit `packages/apple-connector/sqlx/`.
Schema loading in fixtures still uses `sqlx::raw_sql(include_str!(...))` (DDL).
Handler integration seeds that bulk-load static SQL may keep runtime `sqlx::query`
in test-only code.

### OpenAPI export

Regenerate the committed contract after handler or schema changes:

```bash
cargo run -p apple-connector --bin export-openapi docs/openapi.json
```

CI byte-compares `docs/openapi.json` against the generated spec and enforces
one OpenAPI operation per production route/method.

### Checks

```bash
nix fmt
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
nix flake check --no-write-lock-file
```

`nix flake check` runs workspace audit, deny, clippy, test, and treefmt on
`aarch64-darwin`.

### Real database smoke test

Optional ignored integration test (requires a **read-only copy** of your
database):

```bash
export APPLE_CONNECTOR_DATABASE=/path/to/chat.db
cargo test -p apple-connector --test integration smoke_real_database_and_attachment_range -- --ignored

export APPLE_CONNECTOR_NOTES_DATABASE=/path/to/NoteStore.sqlite
cargo test -p apple-connector --test integration smoke_notes_real_database -- --ignored
```

## License

MIT
