---
id: REC-L-0001
status: active
observed-on: "macOS 27; Calendar.sqlitedb OccurrenceCache, AddressBook ZABCDRECORD"
graduated-to: ""
provenance:
  - https://github.com/atahanyorganci/apple-connector/issues/152
  - https://github.com/atahanyorganci/apple-connector/issues/153
  - https://github.com/atahanyorganci/apple-connector/pull/154
  - https://github.com/atahanyorganci/apple-connector/commit/201a0cb
---

# A fixture that fills in a column macOS leaves NULL hides every bug in the code that reads it.

## What happened

Two read paths returned almost nothing on real data while every test passed:

- `/v1/events?start=&end=` returned 1 of 5 events in a four-day window, because it filtered on
  `OccurrenceCache.occurrence_start_date` (#152).
- `/v1/contacts` returned an empty page on an AddressBook with 1005 contacts, because it dropped
  rows without `ZCONTAINER` (#153).

The calendar fixture set `occurrence_start_date = occurrence_date` on its only occurrence row; the
contacts fixture set `ZCONTAINER = 1` on its contact. macOS leaves both NULL.

## Why

The seed rows were written by hand to look complete, not dumped from or modelled on what macOS
writes. A query that depends on a column being set passes against a fixture that always sets it.

## Rule

Seed rows mirror what macOS writes, including the columns it leaves NULL; a fixture row whose
shape has not been checked against a real store says so in a comment.

## Why it is not a test yet

There is no automated way to compare a fixture's NULL pattern with a live store without reading
personal data. The two known cases are fixed in `fixtures/calendar/seed.sql` and
`fixtures/contacts/seed.sql` (PR #154, reproduced first in `201a0cb`). The rule belongs in each
`packages/apple-connector/fixtures/*/README.md`, which do not state it yet.
