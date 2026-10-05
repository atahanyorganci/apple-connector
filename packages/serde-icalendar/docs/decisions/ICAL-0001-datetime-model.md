---
id: ICAL-0001
status: proposed
issues:
  - https://github.com/atahanyorganci/apple-connector/issues/89
  - https://github.com/atahanyorganci/apple-connector/issues/90
provenance:
  - https://github.com/atahanyorganci/apple-connector/pull/140
supersedes: []
superseded-by: []
---

# Event times are one of four RFC 5545 forms — DATE, UTC, zoned, or floating — and none is collapsed into another.

## Question

The model stored every time as a UTC instant. A wall-clock time with `TZID=Europe/Istanbul` at
noon became 12:00Z instead of 09:00Z; a floating time had no representation; TZID could not be
written back (#89).

## Options

- **A UTC instant plus flags.**
- **An enum of the four forms RFC 5545 defines.**

## Decision

`EventDateTime` is an enum:

| Variant | Written as | Instant |
| --- | --- | --- |
| `Date { date }` | `VALUE=DATE` | none; `timestamp()` is midnight UTC as a best effort |
| `Utc { timestamp }` | trailing `Z` | exact |
| `Zoned { timestamp, tzid }` | wall-clock time with `TZID=` | resolved through `chrono-tz` |
| `Floating { local }` | wall-clock time, no zone | none; `timestamp()` reads it as UTC as a best effort |

A DATE-valued `DTEND` is exclusive and written exactly as the model holds it. Ambiguous local
times (DST fall-back) resolve to the earlier offset; non-existent ones (spring-forward gap) and
unknown zone names are parse errors. EXDATE uses the same enum, honours `TZID` across a
comma-separated list, and reports malformed values as errors.

## Consequences

- Callers that need an instant from `Date` or `Floating` must supply a zone themselves.
- Zoned times are written without a `VTIMEZONE` component (known bug: RFC 5545 §3.2.19 requires
  one per `TZID`).

## Evidence

- `packages/serde-icalendar/src/model.rs` `EventDateTime`; `de.rs` `resolve_zoned`, `parse_exdate`.
- `packages/serde-icalendar/tests/datetime.rs`, `tests/exdate.rs`.
- `packages/serde-icalendar/tests/spec.rs`: `zoned_times_are_written_with_their_vtimezone`
  (ignored, fails today).
