---
id: REC-0010
status: proposed
issues:
  - https://github.com/atahanyorganci/apple-connector/issues/104
provenance:
  - https://github.com/atahanyorganci/apple-connector/pull/139
supersedes: [REC-0003]
superseded-by: []
---

# Every timestamp on the wire, in responses and in query parameters, is an integer of UTC Unix seconds.

## Question

After [REC-0003](REC-0003-unix-response-timestamps.md), responses were Unix seconds but query
bounds accepted RFC 3339 or integers, with each domain parsing differently.

## Options

- **Keep both query forms.**
- **Unix seconds only, no deprecation period** (pre-1.0, [REC-0011](REC-0011-pre-1-0-breaking.md)).

## Decision

Unix seconds only. `before`/`after`, `due_before`/`due_after`, `modified_before`/`modified_after`,
`start`/`end`, and `occurrence_start` are `i64`. An inverted range (`before <= after`) is
`400 invalid_timestamp`.

## Consequences

- Clients convert once, at their boundary (the Raycast client does this in `lib/time.ts`).
- An RFC 3339 value is not an integer, so it fails query deserialization. Read endpoints still use
  axum's `Query` extractor, which answers that with a plain-text 400 outside the error envelope
  rather than `invalid_timestamp` (known bug in the apple-connector spec;
  [CONN-L-0005](../../packages/apple-connector/docs/lessons/CONN-L-0005-extractor-rejections.md)).
- Apple's epochs (2001-01-01, nanoseconds for Messages, seconds for Core Data) never appear on the
  wire. Only `NULL` and an exact `0` mean unset; instants before 2001 are negative in Apple's
  epoch, and instants before 1970 are negative on the wire. Reading anything `<= 0` as unset
  dropped every birthday before 2001 (#157).

## Evidence

- `docs/openapi.json`: `UnixTimestamp` is `integer`/`int64`; no schema in the document uses
  `format: date-time`; every timestamp query parameter is `integer`.
- `packages/apple-connector/src/apple_types/timestamp.rs`.
- `packages/apple-connector/tests/spec.rs`: `an_rfc3339_query_bound_is_a_typed_error` (ignored,
  fails today).
