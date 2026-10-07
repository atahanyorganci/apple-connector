---
id: CONN-L-0003
status: graduated
observed-on: "macOS 27 (not checked on 26); Calendar.sqlitedb CalendarItem/Calendar vs EKEventStore lookups; Reminders Data-*.sqlite"
graduated-to: "packages/apple-eventkit/tests/identifier_probe.rs"
provenance:
  - https://github.com/atahanyorganci/apple-connector/issues/150
  - https://github.com/atahanyorganci/apple-connector/issues/151
  - https://github.com/atahanyorganci/apple-connector/pull/154
  - https://github.com/atahanyorganci/apple-connector/commit/555d42c
  - https://github.com/atahanyorganci/apple-connector/issues/156
---

# Calendar data has three identifier columns, and EventKit's lookups for events and calendars are case-sensitive while its reminder lookups are not.

## What happened

- `POST /v1/calendars/{id}/events` always answered 202 with an id no read endpoint resolved (#150).
- `PATCH`/`DELETE /v1/events/{id}` answered 404 for every event, with the id the API itself
  returned; the same id uppercased worked (#151).

## Why

| `CalendarItem` column | Is | EventKit |
| --- | --- | --- |
| `UUID` (uppercase) | the local item id | `calendarItemIdentifier` |
| `unique_identifier` | the iCalendar UID | `calendarItemExternalIdentifier` |
| `external_id` | the server's resource id (an Exchange ItemId, for example) | not exposed |

The read API exposes `lower(UUID)`. `calendarItemWithIdentifier:` and `calendarWithIdentifier:`
compare case-sensitively, so a lowercased UUID never resolves. `Calendar.external_id` is likewise
the server's id, not `calendarIdentifier`. Reminders behave differently: a reminder or reminder
list resolves by its identifier in either case, and a reminder's
`calendarItemExternalIdentifier` equals its UUID.

## Rule

Translate an API id to the identifier EventKit expects, in its stored case, before calling
EventKit; never pass `external_id` to EventKit, and do not assume one entity type's lookup
semantics hold for another.

## Graduated

- `packages/apple-eventkit/tests/identifier_probe.rs` (ignored, live, read-only) asserts every
  statement above against the user's stores and prints only counts. On macOS 27:
  event lookups 99/100 by `UUID`, 0/100 by `lower(UUID)`, 99/100 by `unique_identifier`;
  calendar lookups 9/18 by `UUID`, 0/18 by `lower(UUID)`, 0/14 by `external_id`; reminders 59/59
  in both cases.
- `packages/apple-connector/tests/eventkit_integration.rs` (ignored, live):
  `http_created_event_id_resolves_through_get`, `http_listed_event_id_works_for_patch_and_delete`.

Both halves are applied: events since #154 and calendars since #156, which also added the
probe's check that every calendar EventKit offers for events is addressable by its stored UUID,
and `resolve_metadata_carries_the_stored_identifier` in
`packages/apple-connector/src/calendar/repository.rs`.
