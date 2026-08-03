# apple-connector

Rust monorepo that exposes a **hybrid HTTP API** over Apple Messages, Reminders, Notes, Calendar, and Contacts data on macOS. Reads use live SQLite databases; Reminders, Calendar, and Contacts **writes** go through EventKit / Contacts framework.

| Database | Read path | Write path |
| --- | --- | --- |
| Messages | `~/Library/Messages/chat.db` | — |
| Reminders | Group Containers SQLite store | EventKit (`EKReminder`) |
| Notes | `~/Library/Group Containers/group.com.apple.notes/NoteStore.sqlite` | — |
| Calendar | Calendar Group Containers SQLite store | EventKit (`EKEvent`) |
| Contacts | `~/Library/Application Support/AddressBook/Sources/*/AddressBook-v*.abcddb` | Contacts framework (`CNContactStore`) |

## Crates

| Crate | Path | Role |
| ----------------------- | ------------------------------- | ------------------------------------------------------------------ |
| `apple-connector` | `packages/apple-connector/` | Axum HTTP server, SQLx queries, OpenAPI (`/docs`, `/openapi.json`) |
| `apple-eventkit` | `packages/apple-eventkit/` | EventKit wrapper for Reminders/Calendar writes (macOS-only) |
| `apple-contacts` | `packages/apple-contacts/` | Contacts framework wrapper for contact/group writes (macOS-only) |
| `apple-notes-protobuf` | `packages/apple-notes-protobuf/`| Gzip + protobuf decoder for Apple Notes body blobs |
| `apple-typedstream` | `packages/apple-typedstream/` | Parser for Apple typedstream / attributed message bodies |
| `serde-vcard` | `packages/serde-vcard/` | RFC 6350 vCard Serializer/Deserializer |
| `serde-carddav` | `packages/serde-carddav/` | RFC 6352 CardDAV XML Serializer/Deserializer |

## Layout

- `packages/apple-connector/src/api/` — routes, DTOs, handlers
- `packages/apple-connector/src/messages/` — Messages DB access, classification, attachments
- `packages/apple-connector/src/reminders/` — Reminders DB access and assembly
- `packages/apple-connector/src/notes/` — Notes DB access, body decoding, attachments
- `packages/apple-connector/src/calendar/` — Calendar DB access and assembly
- `packages/apple-connector/src/contacts/` — Contacts DB access and assembly
- `packages/apple-eventkit/src/` — EventKit store, auth, reminder/event mutations
- `packages/apple-contacts/src/` — Contacts store, auth, contact/group mutations
- `packages/apple-connector/src/apple_types/` — shared ID and timestamp types
- `packages/apple-connector/fixtures/` — empty Messages/Reminders/Notes/Calendar/Contacts schemas for offline SQLx
- `docs/openapi.json` — committed OpenAPI contract (regenerate after API changes)

## Run & test

```bash
nix develop
cargo run -p apple-connector # http://127.0.0.1:3000
cargo test -p apple-connector
cargo test -p apple-eventkit
cargo test -p apple-contacts
cargo test -p apple-connector --test contacts_integration -- --ignored  # macOS + permissions
cargo test -p apple-connector --test eventkit_integration -- --ignored  # macOS + permissions
cargo fmt --all && cargo clippy -p apple-connector --all-targets -- -D warnings
nix flake check
```

Requires **Apple Silicon macOS**, **Full Disk Access** (SQLite reads), **Reminders**, **Calendars**, and **Contacts** TCC grants (EventKit/Contacts writes), and Nix with flakes. The API is unauthenticated and defaults to loopback only.

## Planning

Primary sources of truth is issues. While creating a multiphase plan that spans multiple issues, create a new issue for each phase. The parent issue should only have a description with child issues as subtasks.

## Conventions

- SQL uses compile-time checked `query!` / `query_as!` / `query_scalar!`; refresh offline cache with `bash scripts/sqlx-prepare-all.sh` and commit `packages/apple-connector/sqlx/`.
- Date values are always in UTC and represented as Unix seconds (integers), not RFC 3339 strings.
- After handler or schema changes, run `cargo run -p apple-connector --bin export-openapi docs/openapi.json`.
- Always use conventional and commits for changes.
- While creating PRs always make sure appropriate issues are linked.
- NEVER merge anything to main without approval from a maintainer.
- While writing commit messages and PR descriptions prefer markdown formatting.
