---
id: REC-0007
status: superseded
issues:
  - https://github.com/atahanyorganci/apple-connector/issues/50
  - https://github.com/atahanyorganci/apple-connector/issues/62
provenance:
  - https://github.com/atahanyorganci/apple-connector/pull/55
  - https://github.com/atahanyorganci/apple-connector/pull/65
supersedes: []
superseded-by: [REC-0015]
---

# The framework crates compile everywhere, with stubs that return `UnsupportedPlatform` off macOS.

## Question

`apple-eventkit` and `apple-contacts` link macOS frameworks. CI at the time also ran on
`x86_64-linux`.

## Options

- **`cfg`-gated stubs**: the crate builds on every target; calls fail at runtime.
- **Refuse to build off macOS.**

## Decision

Stubs (#50: "`#![cfg(target_os = "macos")]` gate; non-macOS stub"; #62: "`stub.rs`: Non-macOS stub
returning `UnsupportedPlatform`").

## Consequences

The `cfg` attributes were never complete enough to compile off macOS, so the stubs promised a
portability that did not exist, and a non-macOS API could look operational until called (#82).

Superseded by [REC-0015](REC-0015-macos-only.md) (#82, PR #141).

## Evidence

- #82: "Non-macOS APIs cannot appear operational at compile time."
- PR #141, "#82": "Both crates are macOS-only by `compile_error!` instead of the half-finished
  `cfg` attributes that could not compile off macOS."
