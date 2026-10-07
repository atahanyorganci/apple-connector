---
id: CONN-L-0004
status: graduated
observed-on: "macOS 27; EKEventStore removeEvent on a server-backed (CalDAV/Exchange) calendar"
graduated-to: "packages/apple-connector/tests/eventkit_integration.rs"
provenance:
  - https://github.com/atahanyorganci/apple-connector/commit/6b316bf
  - https://github.com/atahanyorganci/apple-connector/pull/154
---

# An event deleted through EventKit can reappear in `Calendar.sqlitedb` when the next server sync runs.

## What happened

The live HTTP tests deleted the events they created, and afterwards marked events were sometimes
still in the calendar.

## Why

The delete can race the server sync of the same event: the row disappears, then the next sync
brings it back from the server before the deletion has propagated.

## Rule

Cleanup of live calendar data keeps deleting until the rows have stayed gone for several
consecutive checks, not until the first check finds none.

## Graduated

`packages/apple-connector/tests/eventkit_integration.rs`, `LiveCalendar::cleanup`: deletes every
event whose summary carries the test's marker and returns only after the marked rows have been
absent for six consecutive passes 500 ms apart (up to 60 passes).
