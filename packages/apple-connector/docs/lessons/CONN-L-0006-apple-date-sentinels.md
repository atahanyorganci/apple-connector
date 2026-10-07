---
id: CONN-L-0006
status: active
observed-on: "macOS 27; Calendar.sqlitedb CalendarItem.start_date/last_modified, AddressBook ZABCDRECORD.ZBIRTHDAY"
graduated-to: ""
provenance:
  - https://github.com/atahanyorganci/apple-connector/issues/157
---

# Apple stores a birthday without a year in 1604, and gives Contacts birthday events a `last_modified` of 1976-04-01.

## What happened

Fixing #157 made Core Data dates before 2001 visible. On a live store, every
pre-2001 date fell into three groups:

| Column | Rows before 2001 | Notable values |
| --- | --- | --- |
| `CalendarItem.start_date` (all birthday events) | 36 | 3 in year 1604, the rest real birth years |
| `CalendarItem.last_modified` | 86 | all exactly −781 142 400 (1976-04-01) |
| `ZABCDRECORD.ZBIRTHDAY` | 36 of 41 | 3 in year 1604 |

## Why

A birthday entered without a year is stored in year 1604, the same convention
Apple's vCard export uses (`X-APPLE-OMIT-YEAR=1604`). Birthday events that
Calendar generates from Contacts all share a fixed `last_modified` of
1976-04-01 — the date Apple was founded — rather than a real modification
time.

## Rule

Return Apple's dates as stored. Never treat a date before 2001 as unset. A
client that needs to know "no year" checks for year 1604.

## Why it is not a test yet

These values are written by macOS, not by this project. Fixtures do not
reproduce them yet: a birthday event with the 1976-04-01 `last_modified` and a
year-less birthday belong in `fixtures/calendar/seed.sql` and
`fixtures/contacts/seed.sql` ([REC-L-0001](../../../../docs/lessons/REC-L-0001-idealized-fixtures.md)).
