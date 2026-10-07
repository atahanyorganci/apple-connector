---
id: REC-0008
status: proposed
issues:
  - https://github.com/atahanyorganci/apple-connector/issues/127
provenance:
  - https://github.com/atahanyorganci/apple-connector/pull/126
supersedes: []
superseded-by: []
---

# `unwrap` and `expect` are denied everywhere, tests included.

## Question

The server reads hostile-shaped data (other apps' databases, archived blobs) and holds framework
locks; a panic takes down a request or poisons a lock. About 35 production and 500 test call sites
used `unwrap`/`expect` (#127).

## Options

- **Ban in production only**: tests stay terse, but the habit and the patterns leak back.
- **Ban everywhere**: tests return `Result<(), Box<dyn std::error::Error>>` and use `?`.

## Decision

Ban everywhere: `clippy::unwrap_used` and `clippy::expect_used` are `deny` in the workspace
`[lints]`, with no `allow` escapes. Errors propagate with `?` into `thiserror` domain errors.

## Consequences

- Tests return `Result` and use `?`, `ok_or(...)?`, or explicit `assert!`.
- Mutex poison is returned as an error rather than panicking (PR #126).
- `panic!` itself is not covered by these lints; a few fixture readers in tests still use
  `unwrap_or_else(|e| panic!(..))`.

## Evidence

- Root `Cargo.toml` `[workspace.lints.clippy]`.
- `cargo clippy --workspace --all-targets -- -D warnings` is `workspace-clippy` in
  `nix/checks.nix`.
- AGENTS.md conventions.
