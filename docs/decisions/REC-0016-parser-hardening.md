---
id: REC-0016
status: proposed
issues:
  - https://github.com/atahanyorganci/apple-connector/issues/84
  - https://github.com/atahanyorganci/apple-connector/issues/85
  - https://github.com/atahanyorganci/apple-connector/issues/86
provenance:
  - https://github.com/atahanyorganci/apple-connector/pull/140
supersedes: []
superseded-by: []
---

# Every public parser has documented input budgets, a bounded reader, and a fuzz target in CI.

## Question

Note bodies decode on every `/v1/notes` read; typedstream blobs come straight out of `chat.db`;
the interchange crates accept request bodies. None had budgets: gzip expanded without a cap, a
14-byte typedstream (`[4294967295@]`) asked for four billion values and aborted the process, and
every `from_reader` did an uncapped `read_to_end` (PR #140).

## Options

- **Trust the input**: it comes from Apple's own stores.
- **Budgets and fuzzing**: the stores are written by other processes and synced from other
  devices, and request bodies are attacker-shaped.

## Decision

Budgets and fuzzing:

- Each crate documents its limits and returns a structured `LimitExceeded { limit, actual, max }`
  error rather than allocating past them.
- Each reader API has a `*_with_limit` variant; the default is the crate's `MAX_INPUT_BYTES`.
- Each public parser has a libFuzzer target in `fuzz/`, with committed seeds, run briefly by
  `scripts/fuzz-smoke.sh` as the `workspace-fuzz-smoke` flake check.

## Consequences

- `fuzz/` is its own workspace with its own lockfile, so `cargo test --workspace` ignores it.
- A new public parser needs a fuzz target and seeds in the same PR.
- The smoke run proves the targets compile and the seeds do not crash; real campaigns run out of
  band with a longer `FUZZ_MAX_TOTAL_TIME`.

## Evidence

- Fuzz targets: `typedstream_parse`, `notes_body_decode`, `notes_table_decode` (added with the
  table decoder in #168), `vcard_parse`, `icalendar_parse`, `caldav_multistatus`,
  `carddav_multistatus`.
- PR #140: about 17.6 million executions across the six targets were clean after the fixes;
  fuzzing found the `iso8601` duration panic
  ([ICAL-L-0001](../../packages/serde-icalendar/docs/lessons/ICAL-L-0001-iso8601-duration-panic.md)).
- Limit tests: `packages/*/tests/limits.rs`, `packages/apple-typedstream/tests/malformed.rs`.
