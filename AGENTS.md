# apple-connector

Rust monorepo exposing a **hybrid HTTP API** over Apple Messages, Reminders, Notes, Calendar, and Contacts on macOS. SQLite reads use live databases; Reminders, Calendar, and Contacts **writes** go through EventKit and the Contacts framework.

## Crates

| Crate | Path | Role |
| --- | --- | --- |
| `apple-connector` | `packages/apple-connector/` | Axum HTTP server, SQLx queries, OpenAPI (`/docs`, `/openapi.json`) |
| `apple-eventkit` | `packages/apple-eventkit/` | EventKit wrapper for Reminders/Calendar writes (macOS-only) |
| `apple-contacts` | `packages/apple-contacts/` | Contacts framework wrapper for contact/group writes (macOS-only) |
| `apple-notes-protobuf` | `packages/apple-notes-protobuf/` | Gzip + protobuf decoder for Apple Notes body blobs |
| `apple-typedstream` | `packages/apple-typedstream/` | Parser for Apple typedstream / attributed message bodies |
| `serde-vcard` | `packages/serde-vcard/` | RFC 6350 vCard serializer/deserializer |
| `serde-carddav` | `packages/serde-carddav/` | RFC 6352 CardDAV XML serializer/deserializer |
| `serde-caldav` | `packages/serde-caldav/` | CalDAV XML serializer/deserializer |
| `serde-icalendar` | `packages/serde-icalendar/` | iCalendar serializer/deserializer |

Domain code lives under `packages/apple-connector/src/{messages,reminders,notes,calendar,contacts,api}/`. Offline SQLx metadata: `packages/apple-connector/sqlx/`. Fixtures: `packages/apple-connector/fixtures/`.

## Run & test

```bash
nix develop
cargo run -p apple-connector                    # http://127.0.0.1:3000
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
nix fmt                                         # fix formatting (also checked in CI)
nix flake check --no-write-lock-file            # audit, deny, clippy, test, treefmt
```

Ignored macOS integration tests (permissions + live stores):

```bash
cargo test -p apple-connector --test eventkit_integration -- --ignored
cargo test -p apple-connector --test contacts_integration -- --ignored
cargo test -p apple-connector --test integration -- --ignored
```

Requires **Apple Silicon macOS**, **Full Disk Access** (SQLite reads), **Reminders**, **Calendars**, and **Contacts** TCC grants (writes), and Nix with flakes. The API is unauthenticated and defaults to loopback only.

## Conventions

- SQL uses compile-time `query!` / `query_as!` / `query_scalar!`; refresh offline cache with `bash scripts/sqlx-prepare-all.sh` and commit `packages/apple-connector/sqlx/`.
- Date values are UTC Unix seconds (integers), not RFC 3339 strings in JSON responses.
- After handler or schema changes: `cargo run -p apple-connector --bin export-openapi docs/openapi.json`.
- Do not use `.unwrap()` or `.expect()` anywhere (production or tests). Propagate errors with `?` and map into domain errors (`thiserror`); in tests return `Result<(), Box<dyn std::error::Error>>` and use `?`.
- API errors use typed `ErrorCode` values (`ApiError::new` / `with_message` / `with_details`). Never return `error.to_string()` from sqlx/EventKit/Contacts to clients; map with `ApiError::from_sqlx` or domain mappers. See `docs/errors.md`.
- Use conventional commits; link issues in PRs; never merge to `main` without maintainer approval.

## Planning

Primary source of truth is issues. For multiphase work, create one issue per phase; the parent issue lists child issues as subtasks.
