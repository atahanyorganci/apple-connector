# apple-connector

Rust monorepo exposing a **hybrid HTTP API** over Apple Messages, Reminders, Notes, Calendar, and Contacts on macOS. SQLite reads use live databases; Reminders, Calendar, and Contacts **writes** go through EventKit and the Contacts framework.

## Crates

| Crate                  | Path                             | Role                                                               |
| ---------------------- | -------------------------------- | ------------------------------------------------------------------ |
| `apple-connector`      | `packages/apple-connector/`      | Axum HTTP server, SQLx queries, OpenAPI (`/docs`, `/openapi.json`) |
| `apple-eventkit`       | `packages/apple-eventkit/`       | EventKit wrapper for Reminders/Calendar writes (macOS-only)        |
| `apple-contacts`       | `packages/apple-contacts/`       | Contacts framework wrapper for contact/group writes (macOS-only)   |
| `apple-notes-protobuf` | `packages/apple-notes-protobuf/` | Gzip + protobuf decoder for Apple Notes body blobs                 |
| `apple-typedstream`    | `packages/apple-typedstream/`    | Parser for Apple typedstream / attributed message bodies           |
| `serde-vcard`          | `packages/serde-vcard/`          | RFC 6350 vCard serializer/deserializer                             |
| `serde-carddav`        | `packages/serde-carddav/`        | RFC 6352 CardDAV XML serializer/deserializer                       |
| `serde-caldav`         | `packages/serde-caldav/`         | CalDAV XML serializer/deserializer                                 |
| `serde-icalendar`      | `packages/serde-icalendar/`      | iCalendar serializer/deserializer                                  |
| `raycast-extension`    | `packages/raycast-extension/`    | Raycast extension (TypeScript, pnpm) with a generated API client   |

Domain code lives under `packages/apple-connector/src/{messages,reminders,notes,calendar,contacts,api}/`. Offline SQLx metadata: `packages/apple-connector/sqlx/`. Fixtures: `packages/apple-connector/fixtures/`.

## Run & test

```bash
nix develop
cargo run -p apple-connector                    # http://127.0.0.1:3000
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
nix fmt                                         # fix formatting (also checked in CI)
nix flake check --no-write-lock-file            # deny, clippy, test, fuzz smoke, scripts, treefmt
cargo deny check                                # includes advisories (needs network)
```

Fuzz targets for every public parser live in `fuzz/` (outside the workspace, so
`cargo test --workspace` ignores them). `nix flake check` runs a smoke pass:

```bash
bash scripts/fuzz-smoke.sh
```

Ignored macOS integration tests (permissions + live stores):

```bash
cargo test -p apple-connector --test eventkit_integration -- --ignored
cargo test -p apple-connector --test contacts_integration -- --ignored
cargo test -p apple-connector --test integration -- --ignored
```

Requires **Apple Silicon macOS**, **Full Disk Access** (SQLite reads), **Reminders**, **Calendars**, and **Contacts** TCC grants (writes), and Nix with flakes. The API is unauthenticated and defaults to loopback only.

## Conventions

- SQL uses compile-time `query!` / `query_as!` / `query_scalar!`; refresh the offline cache with `bash scripts/sqlx-prepare-all.sh` inside `nix develop` (it rebuilds the cache from scratch and fails rather than writing an incomplete one) and commit `packages/apple-connector/sqlx/`.
- Date values are UTC Unix seconds (integers), not RFC 3339 strings in JSON responses.
- After handler or schema changes: `cargo run -p apple-connector --bin export-openapi docs/openapi.json`.
- Do not use `.unwrap()` or `.expect()` anywhere (production or tests). Propagate errors with `?` and map into domain errors (`thiserror`); in tests return `Result<(), Box<dyn std::error::Error>>` and use `?`.
- API errors use typed `ErrorCode` values (`ApiError::new` / `with_message` / `with_details`). Never return `error.to_string()` from sqlx/EventKit/Contacts to clients; map with `ApiError::from_sqlx` or domain mappers. See `docs/errors.md`.
- `apple-eventkit` and `apple-contacts` own their Objective-C stores on a dedicated worker thread. Framework work is submitted as a closure and results come back over a channel; the `EKEventStore` / `CNContactStore` never leaves that thread. Do not reach for `spawn_blocking` or share a `Retained<_>` across threads — add a job instead.
- Both framework crates are macOS-only by construction (`compile_error!` on other targets), not by convention.
- Do not accept a request field the framework cannot store. Reject it with a typed `ErrorCode` (see `immutable_event_field`, `unsupported_reminder_field`) rather than dropping it silently.
- Use conventional commits; link issues in PRs; never merge to `main` without maintainer approval.

## Planning

Decision records, lessons, and specs are the authoritative record of what the code does and why:

- Decisions: `docs/decisions/` and `packages/<crate>/docs/decisions/`
- Lessons: `docs/lessons/` and `packages/<crate>/docs/lessons/`
- Specs: `packages/<crate>/docs/SPEC.md` and `packages/<crate>/docs/spec/`

When an issue, PR, or comment disagrees with them, the records win. A PR that changes behavior updates the affected records in the same PR.

GitHub issues and PRs track progress only: what is planned, in flight, and left to do. For multiphase work, create one issue per phase; the parent issue lists child issues as subtasks. Anything an issue or PR settles (a decision, a lesson, a contract change) lands in a record before merge, and the issue links to it. Always add `Closes #ISSUE` to the PR description so that the issue is automatically closed when the PR is merged.
