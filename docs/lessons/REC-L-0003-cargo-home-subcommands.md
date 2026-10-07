---
id: REC-L-0003
status: graduated
observed-on: "macOS 27; cargo 1.100.0-nightly in `nix develop`; $CARGO_HOME/bin not on PATH"
graduated-to: "scripts/sqlx-prepare-all.sh"
provenance:
  - https://github.com/atahanyorganci/apple-connector/issues/175
  - https://github.com/rust-lang/cargo/issues/11020
---

# Unless `$CARGO_HOME/bin` is on `PATH`, cargo searches it for subcommands before `PATH`, so it can shadow the dev shell's tools.

## What happened

Inside `nix develop`, which puts `sqlx-cli` 0.9.0 on `PATH`, `cargo sqlx prepare` ran
`~/.cargo/bin/cargo-sqlx` instead: an old install linked against a `libiconv` that the Nix store
had since garbage-collected. Every pass aborted with a `dyld` error, and the prepare script
reported success because it ignored the exit status (#175).

## Why

To find an external subcommand `cargo foo`, cargo searches `$CARGO_HOME/bin` (default
`~/.cargo/bin`) and then `PATH`. Only when `$CARGO_HOME/bin` is itself listed on `PATH` does
cargo respect `PATH` order instead (rust-lang/cargo#11020). A `nix develop` shell does not add
`~/.cargo/bin` to `PATH`, so anything installed there with `cargo install` wins over the shell's
own tools.

## Rule

Scripts call a dev-shell tool's own binary (`sqlx`, with `CARGO` set), never the `cargo <tool>`
subcommand form, and check that the tool runs and has the expected version before using it.

## Graduated

`scripts/sqlx-prepare-all.sh` runs `"$(command -v sqlx)" prepare` with `CARGO` exported, and stops
if `sqlx --version` fails or its minor version differs from the `sqlx` in `Cargo.lock`. Running it
with `~/.cargo/bin` first on `PATH` fails at the version check with the `dyld` error (#175).
