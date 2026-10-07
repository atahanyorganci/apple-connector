---
id: REC-0002
status: proposed
issues:
  - https://github.com/atahanyorganci/apple-connector/issues/14
  - https://github.com/atahanyorganci/apple-connector/issues/72
  - https://github.com/atahanyorganci/apple-connector/issues/121
  - https://github.com/atahanyorganci/apple-connector/issues/143
  - https://github.com/atahanyorganci/apple-connector/issues/175
provenance:
  - https://github.com/atahanyorganci/apple-connector/pull/15
  - https://github.com/atahanyorganci/apple-connector/pull/125
  - https://github.com/atahanyorganci/apple-connector/pull/126
  - https://github.com/atahanyorganci/apple-connector/pull/149
supersedes: []
superseded-by: []
---

# Artifacts derived from code are generated, committed, and checked for staleness.

## Question

Three artifacts describe code that lives elsewhere: the OpenAPI document (from utoipa
annotations), the SQLx offline query cache (from five Apple schemas), and the Raycast API client
(from the OpenAPI document). Each can drift from its source while every local test still passes.

## Options

- **Generate at build time only**: nothing committed; reviewers never see contract changes, and
  CI on Linux cannot reach the Apple schemas.
- **Hand-maintain**: drifts by construction (#121 found separate route registries drifting).
- **Generate, commit, and compare**: the diff shows contract changes in review; a check fails when
  the committed copy is stale.

## Decision

Generate, commit, and compare:

| Artifact | Generator | Staleness check |
| --- | --- | --- |
| `docs/openapi.json` | `cargo run -p apple-connector --bin export-openapi docs/openapi.json` | `exported_openapi_matches_committed_contract` in `api/doc.rs` (part of `cargo test`) |
| `packages/apple-connector/sqlx/` | `scripts/sqlx-prepare-all.sh` | `SQLX_OFFLINE=true` builds fail on a missing query entry; the script rebuilds the cache from scratch, so a run also removes stale entries |
| `packages/raycast-extension/src/lib/api.gen.ts` | `pnpm generate` | `pnpm generate:check` |

The router's route inventory is derived from the same OpenAPI spec (#121), so a route cannot
exist in one and not the other.

## Consequences

- A PR that changes handlers, DTOs, or SQL regenerates and commits the artifact in the same PR,
  per schema-changing commit when the history should stay bisectable (PR #141).
- The SQLx check in CI catches *missing* entries only. Stale entries are removed whenever the
  prepare script runs, because it rebuilds the cache from scratch (#175, which deleted 64), but
  nothing fails on them in between.
- `pnpm generate:check` is not part of `nix flake check`, which excludes the Raycast package, so
  Raycast client staleness is only caught locally.

## Evidence

- `.cargo/config.toml` sets `SQLX_OFFLINE=true` and `SQLX_OFFLINE_DIR=packages/apple-connector/sqlx`.
- `nix/checks.nix`: `workspace-test` runs `cargo test --workspace --all-targets`; no check runs
  `pnpm`.
- PR #149: the generator emits 178 schemas and 70 operations and is byte-identical across runs.
- `scripts/sqlx-prepare-all.sh` verifies with an offline build against the rebuilt cache. Its
  earlier `cargo sqlx prepare --check` step could never pass without a live database for every
  store (PR #154 note; fixed in #175).
