---
id: CONN-L-0001
status: graduated
observed-on: "macOS 27 (not checked on 26); Calendar.sqlitedb OccurrenceCache.occurrence_start_date"
graduated-to: "packages/apple-connector/tests/calendar_integration.rs"
provenance:
  - https://github.com/atahanyorganci/apple-connector/issues/152
  - https://github.com/atahanyorganci/apple-connector/pull/154
  - https://github.com/atahanyorganci/apple-connector/commit/201a0cb
  - https://github.com/atahanyorganci/apple-connector/commit/d72e018
---

# `OccurrenceCache.occurrence_start_date` is NULL on the first row of every occurrence; the start is in `occurrence_date`.

## What happened

A date-range event listing over a four-day window returned 1 event of the 5 inside it. The query
filtered and ordered on `occurrence_start_date`, and `NULL <= ?` is never true.

## Why

macOS writes one `OccurrenceCache` row per day an occurrence touches. The first row keeps the
occurrence start in `occurrence_date` and leaves `occurrence_start_date` NULL. Continuation rows,
for occurrences that cross midnight, set `occurrence_date` to that day and keep the original start
in `occurrence_start_date`. `occurrence_end_date` is set on every row.

| Rows | `occurrence_start_date IS NULL` | `occurrence_end_date IS NULL` |
| --- | --- | --- |
| 2298 | 2283 (99%) | 0 |

The one event that did appear was an overnight flight, found through its day-two continuation row.

## Rule

An occurrence's start is `COALESCE(occurrence_start_date, occurrence_date)`, and its rows are
grouped by `(event_id, start)` so an occurrence crossing midnight is one result.

## Graduated

`packages/apple-connector/tests/calendar_integration.rs`:

- `integration_calendar_range_includes_single_day_events`: a single-day event, whose only row has
  `occurrence_start_date` NULL, is listed by a range query.
- `integration_calendar_range_lists_overnight_occurrence_once`: the overnight event
  (`SEED_OVERNIGHT_EVENT_ID`) is listed once with its real start.

`packages/apple-connector/fixtures/calendar/seed.sql` documents the row shape.
