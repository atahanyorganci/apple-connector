# Apple Connector

Monorepo for reading Apple platform data on macOS. The
[`packages/apple-connector`](packages/apple-connector/) crate exposes a
read-only HTTP API over Messages.app and Reminders.app data backed by live
SQLite connections to `~/Library/Messages/chat.db` and the Reminders Group
Containers store.

## Requirements

- Apple Silicon Mac (`aarch64-darwin`)
- Messages.app signed in
- Nix with flakes enabled
- Full Disk Access for the terminal that runs the server (Messages **and**
  Reminders Group Containers)

Grant access in **System Settings → Privacy & Security → Full Disk Access**, then
restart the terminal. If macOS still denies access when running the compiled
binary directly, add `target/debug/apple-connector` there as well.

## Quick start

```bash
nix develop
cargo run -p apple-connector
```

By default the server binds to `127.0.0.1:3000`, opens
`$HOME/Library/Messages/chat.db` and auto-discovers the Reminders store
read-only, and serves the API documented at `/openapi.json`. Browse and try
endpoints interactively at `/docs`.

```bash
curl -s http://127.0.0.1:3000/healthz
curl -s 'http://127.0.0.1:3000/v1/messages?limit=5'
curl -s http://127.0.0.1:3000/openapi.json | jq .info.title
```

## CLI options

| Option | Default | Description |
| --- | --- | --- |
| `--address <IP>` | `127.0.0.1` | Bind address. Loopback (`127.0.0.1`, `127.0.0.2`, `::1`) or explicit `0.0.0.0` only. |
| `--port <PORT>` | `3000` | TCP port (`1`–`65535`; `0` is rejected). |
| `--messages-database <PATH>` | `$HOME/Library/Messages/chat.db` | Read-only path to Messages `chat.db`. The server never creates or mutates this file. |
| `--reminders-database <PATH>` | auto-discover | Read-only path to the Reminders SQLite store. |
| `--reminders-stores-dir <PATH>` | Group Containers `Stores/` | Directory scanned when auto-discovering the Reminders store. |
| `--attachment-root <PATH>` | `<messages-database-dir>/Attachments` | Messages attachment directory (canonicalized and confined at runtime). |
| `--reminders-attachment-root <PATH>` | `.Data-{UUID}_SUPPORT` next to store | Reminders attachment support directory. |
| `--help` | — | Print usage and exit. |
| `--version` | — | Print crate version and exit. |

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

- **Full Disk Access** is required to open `chat.db`, Reminders Group Containers
  stores, and attachment files under `~/Library/Messages/Attachments/`.
- If either database cannot be opened, `/healthz` returns `503` with per-service
  status and data endpoints return structured `service_unavailable` errors.
  Startup logs do **not** include filesystem paths.

## HTTP endpoints

| Method | Path | Description |
| --- | --- | --- |
| `GET` | `/healthz` | Composite database pool health (`messages` / `reminders`: `ok` / `unavailable`). Returns `200` only when both are `ok`. |
| `GET` | `/v1/chats` | Paginated chat list (newest activity first). |
| `GET` | `/v1/chats/{chat_id}` | Single chat metadata and participants. |
| `GET` | `/v1/chats/{chat_id}/messages` | Paginated messages for one chat. |
| `GET` | `/v1/messages` | Global message list with optional search/filters. |
| `GET` | `/v1/messages/{guid}` | Single message with classified content. |
| `GET` | `/v1/attachments/{guid}` | Attachment metadata (no local paths). |
| `GET`, `HEAD` | `/v1/attachments/{guid}/content` | Attachment bytes with range/conditional support. |
| `GET` | `/openapi.json` | OpenAPI 3.1 contract (same document as `docs/openapi.json`). |
| `GET` | `/docs` | Scalar API reference (embedded OpenAPI 3.1 contract). |

Unknown routes return JSON `404`; unsupported methods return JSON `405`. All
responses include `X-Content-Type-Options: nosniff`, `X-Frame-Options: DENY`,
`Referrer-Policy: no-referrer`, and `Cache-Control: no-store`.

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

| Parameter | Description |
| --- | --- |
| `q` | Case-insensitive text search (max 256 chars). Matches plain `text` and decoded `attributedBody`. |
| `chat_id` | Restrict to one chat row id. |
| `sender` | Handle identifier (phone/email). |
| `before`, `after` | ISO-8601 UTC bounds on message time. |
| `direction` | `sent` or `received`. |
| `transport` | `imessage`, `sms`, `rcs`, or `unknown`. |
| `content_type` | Classified content kind (`text`, `attachment`, `reaction`, …). |
| `has_attachments` | `true` or `false`. |

Text search uses a resumable search cursor when filters are active. New rows
written to `chat.db` by Messages.app are visible on the next request without
restarting the server.

## Privacy and logging

- API DTOs omit local filesystem paths, raw balloon payloads, and opaque binary
  fields.
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

```bash
./packages/apple-connector/fixtures/messages/create-empty-db.sh
cp packages/apple-connector/.env.example packages/apple-connector/.env
cargo install sqlx-cli --version 0.9.0 --no-default-features --features sqlite
cargo sqlx prepare -p apple-connector
rsync -a --delete .sqlx/ packages/apple-connector/sqlx/
```

Commit the updated `packages/apple-connector/sqlx/` directory. Nix builds use
`SQLX_OFFLINE=true` with `SQLX_OFFLINE_DIR=packages/apple-connector/sqlx` and do
not need a database connection.

### SQL queries

Queries in `packages/apple-connector/src/` are verified at compile time with SQLx
`query_as!`. After changing SQL, run the fixture steps above to refresh the offline
cache.

### OpenAPI export

Regenerate the committed contract after handler or schema changes:

```bash
cargo run -p apple-connector --bin export-openapi docs/openapi.json
```

CI byte-compares `docs/openapi.json` against the generated spec and enforces
one OpenAPI operation per production route/method.

### Checks

```bash
cargo fmt --all
cargo clippy -p apple-connector --all-targets -- -D warnings
cargo test -p apple-connector
nix flake check --no-write-lock-file
```

### Real database smoke test

Optional ignored integration test (requires a **read-only copy** of your
database):

```bash
export APPLE_CONNECTOR_DATABASE=/path/to/chat.db
cargo test -p apple-connector --test integration smoke_real_database_and_attachment_range -- --ignored
```

## License

MIT
