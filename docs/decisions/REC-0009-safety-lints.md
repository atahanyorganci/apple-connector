---
id: REC-0009
status: proposed
issues:
  - https://github.com/atahanyorganci/apple-connector/issues/118
  - https://github.com/atahanyorganci/apple-connector/issues/122
provenance:
  - https://github.com/atahanyorganci/apple-connector/pull/126
supersedes: []
superseded-by: []
---

# `unsafe` is forbidden workspace-wide except in the two FFI crates, and CI checks the whole workspace.

## Question

Only `apple-eventkit` and `apple-contacts` need `unsafe` for Objective-C calls, but nothing stopped
it elsewhere. CI ran tests and Clippy for `apple-connector` alone, so sibling crates were
unchecked (#118).

## Options

- **Convention**: review catches `unsafe`.
- **Lint**: `unsafe_code = "forbid"` in the workspace, opted out per FFI crate.

## Decision

Lint. The workspace `[lints]` forbids `unsafe_code`; `apple-eventkit` and `apple-contacts` set
`rust.unsafe_code = "allow"` in their own manifests. `nix flake check` runs Clippy and tests with
`--workspace --all-targets`, `cargo deny` for bans, licenses, and sources, a runtime-SQL guard,
an API error-leakage guard, a fuzz smoke run, and treefmt.

## Consequences

- New `unsafe` outside the two FFI crates fails the build, not review.
- `cargo deny` advisories need the network, so they are not part of `nix flake check`; run
  `cargo deny check` locally.

## Evidence

- `nix/checks.nix`: `workspace-clippy`, `workspace-test`, `workspace-deny`
  (`check bans licenses sources`), `workspace-runtime-sql`, `workspace-api-error-leakage`,
  `workspace-fuzz-smoke`.
- `.github/workflows/checks.yml`: `nix flake check --no-write-lock-file` on `macos-latest`.
- `packages/apple-eventkit/Cargo.toml`, `packages/apple-contacts/Cargo.toml` `[lints]`.
