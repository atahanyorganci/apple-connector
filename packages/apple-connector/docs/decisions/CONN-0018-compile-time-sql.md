---
id: CONN-0018
status: proposed
issues:
  - https://github.com/atahanyorganci/apple-connector/issues/66
  - https://github.com/atahanyorganci/apple-connector/issues/70
  - https://github.com/atahanyorganci/apple-connector/issues/71
  - https://github.com/atahanyorganci/apple-connector/issues/72
  - https://github.com/atahanyorganci/apple-connector/issues/73
  - https://github.com/atahanyorganci/apple-connector/issues/175
provenance:
  - https://github.com/atahanyorganci/apple-connector/pull/125
  - https://github.com/atahanyorganci/apple-connector/pull/126
supersedes: []
superseded-by: []
---

# Production SQL is compile-time checked with SQLx macros against per-store fixture schemas.

## Question

Queries ran through `QueryBuilder` and `format!`, so a column Apple renamed failed at request
time. SQLx's compile-time macros need a schema, but the five stores are unrelated Core Data
schemas whose tables collide.

## Options

- **Runtime SQL with tests**: catches only what the tests reach.
- **Macros against one union fixture**: blocked; the Core Data schemas conflict.
- **Macros against one fixture per store**, with the offline metadata merged.

## Decision

One fixture per store. Every production query uses `query!`, `query_as!`, or `query_scalar!`
with a static shape: optional filters bind `NULL` (`?n IS NULL OR …`), and variable-length ID
lists are bound as JSON and expanded with `json_each(?)`. Builds use `SQLX_OFFLINE=true`
against the committed cache in `packages/apple-connector/sqlx/`.

`scripts/sqlx-prepare-all.sh` rebuilds that cache from scratch with one `sqlx prepare` pass per
fixture. Each pass saves the queries its store can describe, and the other stores' queries fail to
compile, which is expected. The script therefore checks pass outcomes, not compiler messages:

- a pass must exit 0, or 1 because `cargo check` failed, and must save at least one query;
- a query saved by several passes must be described identically by each;
- an offline build against the rebuilt cache must succeed.

The offline build is what proves every tolerated failure was a cross-store one: a real Rust
error fails it too, and so does a query its own store cannot describe, because it has no cache
entry. The committed cache is replaced only after that build passes. Classifying compiler
messages instead is fragile, because a failed query macro also causes follow-on errors
(`` `!` is not an iterator ``, mismatched types).

## Consequences

- A query change regenerates and commits `packages/apple-connector/sqlx/`
  ([REC-0002](../../../../docs/decisions/REC-0002-generated-artifacts.md)). A run leaves exactly
  the queries the code uses, so it also deletes stale entries.
- The script calls the dev shell's `sqlx` with `CARGO` set, never `cargo sqlx`
  ([REC-L-0003](../../../../docs/lessons/REC-L-0003-cargo-home-subcommands.md)), and checks that
  its version matches the `sqlx` in `Cargo.lock`. Until #175 a crashed tool was hidden behind
  `|| true` and the script reported success without preparing anything.
- `scripts/check-runtime-sql.sh` rejects non-macro `sqlx::query`/`query_as` under
  `packages/apple-connector/src`, allowing only `fixtures.rs` and the test module of
  `api/handlers/attachments.rs`. It does not scan `packages/apple-connector/tests/`, which still
  use runtime SQL, so #71's "CI rejects new runtime SQL in test code" is not met.
- Fixtures must reproduce what macOS writes, or the checked queries are checked against the wrong
  data ([REC-L-0001](../../../../docs/lessons/REC-L-0001-idealized-fixtures.md)).

## Evidence

- `scripts/check-runtime-sql.sh`; flake check `workspace-runtime-sql`.
- `rg 'sqlx::query(_as|_scalar)?\(' packages/apple-connector/tests` matches
  `integration.rs` and `eventkit_integration.rs`.
- PR #125: 171 query files at the time. The cache had grown to 212 entries by #173; the first
  rebuild from scratch (#175) left 148, deleting 64 stale entries, and the offline build and
  `nix flake check` passed against it.
- #175 runs on macOS 27: the stale `~/.cargo/bin/sqlx` fails the version check with its `dyld`
  error; a tool that aborts during a pass, one that exits 0 without preparing anything, and one
  that reports sqlx-cli 0.8 each stop the script; a query referencing a column its store lacks
  fails the offline build; in every case the committed cache is unchanged. Two clean runs give a
  byte-identical cache, and the 37 queries several passes save (`SELECT 1`, `sqlite_master`
  probes, `Z_PRIMARYKEY` lookups, and Calendar's `Attachment`, which SQLite also resolves in the
  Messages fixture because table names are case-insensitive) are described identically.
