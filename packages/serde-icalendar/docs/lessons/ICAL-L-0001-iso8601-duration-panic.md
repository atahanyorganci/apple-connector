---
id: ICAL-L-0001
status: graduated
observed-on: "iso8601 crate 0.6.5, iso8601::duration"
graduated-to: "packages/serde-icalendar/tests/participants.rs; fuzz/seeds/icalendar_parse/seed-overflowing-trigger.ics"
provenance:
  - https://github.com/atahanyorganci/apple-connector/issues/86
  - https://github.com/atahanyorganci/apple-connector/pull/140
  - https://github.com/atahanyorganci/apple-connector/commit/e444048
---

# `iso8601::duration` unwraps internally and panics when a digit run overflows.

## What happened

The `icalendar_parse` fuzz target aborted the process serializing an alarm with
`TRIGGER:-P5444444444444444444444444444D`.

## Why

The crate's duration parser converts each digit run with an unchecked unwrap, so an out-of-range
component panics instead of returning an error. Parsing a duration from untrusted input through it
is a crash.

## Rule

Durations from untrusted input are validated with checked arithmetic, and an out-of-range
component is an error.

## Graduated

- `packages/serde-icalendar/src/ser.rs` `validate_duration` is hand-written with checked
  arithmetic, and the `iso8601` dependency was removed in `e444048`.
- `packages/serde-icalendar/tests/participants.rs`:
  `an_absurd_trigger_duration_is_rejected_not_a_panic`,
  `malformed_trigger_durations_are_rejected`.
- `fuzz/seeds/icalendar_parse/seed-overflowing-trigger.ics` keeps the input in the fuzz corpus.
