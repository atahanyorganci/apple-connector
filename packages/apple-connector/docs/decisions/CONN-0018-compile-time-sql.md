---
id: CONN-0018
status: proposed
issues:
  - https://github.com/atahanyorganci/apple-connector/issues/66
  - https://github.com/atahanyorganci/apple-connector/issues/70
  - https://github.com/atahanyorganci/apple-connector/issues/71
  - https://github.com/atahanyorganci/apple-connector/issues/72
  - https://github.com/atahanyorganci/apple-connector/issues/73
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
lists are bound as JSON and expanded with `json_each(?)`. `scripts/sqlx-prepare-all.sh` runs
`cargo sqlx prepare` against each fixture in turn and merges the results into
`packages/apple-connector/sqlx/`; builds use `SQLX_OFFLINE=true`.

## Consequences

- A query change regenerates and commits `packages/apple-connector/sqlx/`
  ([REC-0002](../../../../docs/decisions/REC-0002-generated-artifacts.md)).
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
- PR #125: 171 query files at the time; 211 entries in `packages/apple-connector/sqlx/` now.
