---
id: CONN-0023
status: proposed
issues:
  - https://github.com/atahanyorganci/apple-connector/issues/150
  - https://github.com/atahanyorganci/apple-connector/issues/151
provenance:
  - https://github.com/atahanyorganci/apple-connector/pull/154
  - https://github.com/atahanyorganci/apple-connector/commit/555d42c
supersedes: [CONN-0017]
superseded-by: []
---

# The event id is `lower(CalendarItem.UUID)`, and EventKit is addressed with the stored identifiers in their own spaces.

## Question

[CONN-0017](CONN-0017-external-id-lookup.md) mixed three identifier spaces, so created ids never
resolved and every PATCH and DELETE was 404.

## Options

- **Make `GET` also accept EventKit identifiers**: widens the read API to two id spaces.
- **Translate at the write boundary**: the API has one id; the write path looks up the stored
  identifiers in SQLite and passes each to the EventKit call that expects it.

## Decision

Translate at the write boundary:

| `CalendarItem` column | EventKit | API |
| --- | --- | --- |
| `UUID` (uppercase) | `calendarItemIdentifier` | `EventId` = `lower(UUID)` |
| `unique_identifier` | `calendarItemExternalIdentifier` (iCalendar UID) | — |
| `external_id` | not exposed (the server's resource id, e.g. an Exchange ItemId) | — |

Create hydrates and answers with `lower(calendarItemIdentifier)`. Update and delete resolve
`UUID` and `unique_identifier` from SQLite (`WHERE lower(UUID) = lower(?)`) and pass the stored
`UUID` to `calendarItemWithIdentifier:` and `unique_identifier` to
`calendarItemsWithExternalIdentifier:`. Update answers with the path id.

## Consequences

- **Known bug, same class**: calendars are not translated. `resolve_event_calendar` tries
  `calendarWithIdentifier:` with `Calendar.external_id` and then the lowercased API
  `CalendarId`; the probe shows neither ever resolves on macOS 27, so every create or move falls
  back to matching the calendar's title, and two calendars with the same title answer
  `409 ambiguous_event_kit_match`.
- Reminders do not need translation: reminder and reminder-list lookups are case-insensitive, and
  a reminder's external identifier is its API id (probe, macOS 27).

## Evidence

- Observed on macOS 27 ([CONN-L-0003](../lessons/CONN-L-0003-calendar-identifier-spaces.md)).
- `packages/apple-eventkit/tests/identifier_probe.rs` (ignored, live, read-only), macOS 27:

  | Lookup | Resolved |
  | --- | --- |
  | `calendarItemWithIdentifier:` with `CalendarItem.UUID` | 99/100 |
  | … with `lower(UUID)` | 0/100 |
  | `calendarItemsWithExternalIdentifier:` with `unique_identifier` | 99/100 |
  | `calendarWithIdentifier:` with `Calendar.UUID` | 9/18 |
  | … with `lower(UUID)` | 0/18 |
  | … with `Calendar.external_id` | 0/14 |
  | `calendarItemWithIdentifier:` with a reminder's `lower(ZIDENTIFIER)` | 59/59 |

- `packages/apple-connector/tests/eventkit_integration.rs` (ignored, live):
  `http_created_event_id_resolves_through_get`, `http_listed_event_id_works_for_patch_and_delete`.
- `packages/apple-eventkit/src/calendar_resolve.rs` `resolve_event_calendar`, `lookup_calendar`;
  `packages/apple-connector/src/calendar/queries.rs` (`lower(c.UUID) AS api_id`, `c.external_id`).
