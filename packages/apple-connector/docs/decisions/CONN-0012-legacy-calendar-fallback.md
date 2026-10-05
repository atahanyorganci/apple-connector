---
id: CONN-0012
status: superseded
issues:
  - https://github.com/atahanyorganci/apple-connector/issues/39
  - https://github.com/atahanyorganci/apple-connector/issues/40
  - https://github.com/atahanyorganci/apple-connector/issues/46
provenance:
  - https://github.com/atahanyorganci/apple-connector/pull/48
supersedes: []
superseded-by: [CONN-0020]
---

# Calendar discovery falls back to legacy `~/Library/Calendars` databases.

## Question

Older macOS releases kept Calendar data under `~/Library/Calendars` with a Core Data schema
(`ZCALENDARITEM`); current releases use `Calendar.sqlitedb` with `CalendarItem`.

## Options

- **Support both schemas.**
- **Support only the modern schema.**

## Decision

Discover legacy paths as a fallback and probe the schema variant (#40), with legacy reads planned
"if legacy fixture available" (#46).

## Consequences

The legacy read path was never written. The server opened legacy databases and then queried
modern table names, so inventory reported false zeroes and queries failed at request time (#99).

Superseded by [CONN-0020](CONN-0020-schema-fail-fast.md) (#99, PR #139), which removed the
fallback and the `ZCALENDARITEM` variant.

## Evidence

- #99: "This misleading half-support is worse than an explicit failure."
