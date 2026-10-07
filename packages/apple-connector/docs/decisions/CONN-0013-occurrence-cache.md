---
id: CONN-0013
status: proposed
issues:
  - https://github.com/atahanyorganci/apple-connector/issues/39
  - https://github.com/atahanyorganci/apple-connector/issues/42
  - https://github.com/atahanyorganci/apple-connector/issues/152
provenance:
  - https://github.com/atahanyorganci/apple-connector/pull/48
  - https://github.com/atahanyorganci/apple-connector/pull/154
supersedes: []
superseded-by: []
---

# Date-range event listings read Apple's `OccurrenceCache`, one row per occurrence, instead of expanding recurrence rules.

## Question

A date-range query must return each occurrence of a recurring event inside the range. Recurrence
lives in `Recurrence` rows plus exception dates and detached instances.

## Options

- **Expand RRULEs ourselves**: re-implements Calendar's recurrence engine, time zones, and
  exceptions, and will disagree with Calendar.app at the edges.
- **Read `OccurrenceCache`**: the table Calendar itself maintains, one row per day an occurrence
  touches.

## Decision

Read `OccurrenceCache` whenever `start` or `end` is set; without a range, list `CalendarItem` rows
directly. The occurrence start is `COALESCE(occurrence_start_date, occurrence_date)` and the
occurrence end is the maximum `occurrence_end_date`; a CTE groups by `(event_id, start)` so an
occurrence that crosses midnight is one row. Range filtering, ordering, and the cursor all use
that resolved start.

## Consequences

- Occurrences exist only as far ahead as Calendar has cached them.
- The first version filtered on `occurrence_start_date` alone and dropped about 99% of events,
  because macOS leaves it NULL on each occurrence's first row
  ([CONN-L-0001](../lessons/CONN-L-0001-occurrence-start-null.md)).

## Evidence

- `packages/apple-connector/src/calendar/queries.rs` `fetch_occurrence_events_page`;
  `calendar/repository.rs` (`use_occurrence` when a bound is set).
- `packages/apple-connector/tests/calendar_integration.rs`: range listing includes single-day
  events, lists an overnight occurrence once with its real start, and pages through every event
  exactly once.
