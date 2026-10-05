---
id: CONN-0017
status: superseded
issues:
  - https://github.com/atahanyorganci/apple-connector/issues/49
provenance:
  - https://github.com/atahanyorganci/apple-connector/pull/55
supersedes: []
superseded-by: [CONN-0023]
---

# Calendar writes find EventKit items by external identifier and hydrate by the identifier EventKit returns.

## Question

The read API names events by `lower(CalendarItem.UUID)`. EventKit has `calendarItemIdentifier`
(local) and `calendarItemExternalIdentifier` (shared with the server). Which does the write path
use?

## Options

- **External identifier everywhere**: look items up with `calendarItemsWithExternalIdentifier:`,
  and hydrate by the identifier EventKit returns after save.
- **Map from SQLite first**: resolve the API id to the stored identifiers, then call EventKit with
  each in its own space.

## Decision

External identifier everywhere (#49, "ID resolution": "hydrate from SQLite by external
identifier"; "Updates/deletes: `calendarItems(withExternalIdentifier:)`").

## Consequences

The identifier spaces were mixed. Create hydrated the iCalendar UID against the `UUID` column, so
it always answered 202 with an id no read endpoint resolves (#150). Update and delete passed the
lowercased API id to the case-sensitive `calendarItemWithIdentifier:` and the server's own
`external_id` where EventKit expects the iCalendar UID, so every PATCH and DELETE was 404 (#151).

Superseded by [CONN-0023](CONN-0023-event-identifiers.md) (PR #154).

## Evidence

- #150 and #151 root-cause sections; observed on macOS 27.
