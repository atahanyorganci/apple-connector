---
id: REC-0015
status: proposed
issues:
  - https://github.com/atahanyorganci/apple-connector/issues/82
provenance:
  - https://github.com/atahanyorganci/apple-connector/pull/141
supersedes: [REC-0007]
superseded-by: []
---

# The framework crates refuse to build off macOS, by `compile_error!`.

## Question

[REC-0007](REC-0007-non-macos-stubs.md)'s stubs never compiled off macOS and could make a
non-macOS build look operational.

## Options

- **Finish the `cfg` stubs.**
- **Gate the `objc2-*` dependencies under `[target.'cfg(target_os = "macos")'.dependencies]`**
  and `compile_error!`.
- **`compile_error!` with the dependencies ungated.**

## Decision

`compile_error!` at the top of `lib.rs`, with the `objc2-*` dependencies left ungated. Gating them
would bury the one clear message under a pile of unresolved-import errors (PR #141, reviewer note
1).

## Consequences

- The workspace builds only on macOS; CI runs on `macos-latest`.
- `apple-connector` depends on both crates unconditionally, so it is macOS-only too.

## Evidence

- `packages/apple-eventkit/src/lib.rs`, `packages/apple-contacts/src/lib.rs`:
  `#[cfg(not(target_os = "macos"))] compile_error!(...)`.
- `.github/workflows/checks.yml`: `runs-on: macos-latest`.
