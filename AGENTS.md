# apple-connector

Rust monorepo that exposes a **read-only HTTP API** over Apple Messages and Reminders data on macOS. It reads live SQLite databases (`~/Library/Messages/chat.db` and the Reminders Group Containers store) and serves JSON plus attachment bytes.

## Crates

| Crate               | Path                          | Role                                                               |
| ------------------- | ----------------------------- | ------------------------------------------------------------------ |
| `apple-connector`   | `packages/apple-connector/`   | Axum HTTP server, SQLx queries, OpenAPI (`/docs`, `/openapi.json`) |
| `apple-typedstream` | `packages/apple-typedstream/` | Parser for Apple typedstream / attributed message bodies           |

## Layout

- `packages/apple-connector/src/api/` — routes, DTOs, handlers
- `packages/apple-connector/src/messages/` — Messages DB access, classification, attachments
- `packages/apple-connector/src/reminders/` — Reminders DB access and assembly
- `packages/apple-connector/src/apple_types/` — shared ID and timestamp types
- `packages/apple-connector/fixtures/` — empty Messages/Reminders schemas for offline SQLx
- `docs/openapi.json` — committed OpenAPI contract (regenerate after API changes)

## Run & test

```bash
nix develop
cargo run -p apple-connector          # http://127.0.0.1:3000
cargo test -p apple-connector
cargo fmt --all && cargo clippy -p apple-connector --all-targets -- -D warnings
nix flake check
```

Requires **Apple Silicon macOS**, **Full Disk Access**, and Nix with flakes. The API is unauthenticated and defaults to loopback only.

## Planning

Primary sources of truth is issues. While creating a multiphase plan that spans multiple issues, create a new issue for each phase. The parent issue should only have a description with child issues as subtasks.

## Conventions

- SQL uses compile-time checked `query_as!`; refresh offline cache after query changes (`cargo sqlx prepare`, commit `packages/apple-connector/sqlx/`).
- Date values are always in UTC and represented as Unix seconds (integers), not RFC 3339 strings.
- After handler or schema changes, run `cargo run -p apple-connector --bin export-openapi docs/openapi.json`.
- Always use conventional and commits for changes.
- While creating PRs always make sure appropriate issues are linked.
- NEVER merge anything to main without approval from a maintainer.
- While writing commit messages and PR descriptions prefer markdown formatting.
