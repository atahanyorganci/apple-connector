---
id: REC-0005
status: proposed
issues:
  - https://github.com/atahanyorganci/apple-connector/issues/39
  - https://github.com/atahanyorganci/apple-connector/issues/49
  - https://github.com/atahanyorganci/apple-connector/issues/56
provenance:
  - https://github.com/atahanyorganci/apple-connector/pull/55
  - https://github.com/atahanyorganci/apple-connector/pull/65
supersedes: []
superseded-by: []
---

# Reads come from the live SQLite stores; writes go only through EventKit and the Contacts framework.

## Question

The read API was built on Apple's SQLite stores. Writes for Reminders, Calendar, and Contacts
had to go somewhere: the same SQLite files, or Apple's frameworks.

## Options

- **Write SQLite directly**: one code path for reads and writes, but bypasses Apple's sync
  engine, CloudKit/CalDAV/Exchange propagation, and every app's in-memory cache. Rejected for
  Calendar in #39 and, by the same reasoning, for Reminders and Contacts.
- **Frameworks for everything**: EventKit and Contacts expose no sections, subtasks, tags,
  smart-list filters, or attachment files, and Messages and Notes have no public framework.
- **Hybrid**: SQLite for reads, framework for writes, then read the result back from SQLite.

## Decision

Hybrid. Every SQLite pool is opened read-only (`read_only(true)`, `create_if_missing(false)` in
`db.rs`). Reminders and Calendar writes go through `apple-eventkit`; Contacts writes through
`apple-contacts`. Messages and Notes are read-only.

## Consequences

- Writes and reads use different identifier spaces; mapping them is its own problem
  ([CONN-0023](../../packages/apple-connector/docs/decisions/CONN-0023-event-identifiers.md)).
- The SQLite read trails the framework write by an unspecified delay
  ([CONN-0022](../../packages/apple-connector/docs/decisions/CONN-0022-async-mutations.md)).
- A field the framework cannot store is rejected, even when SQLite can read it
  ([REC-0012](REC-0012-reject-not-coerce.md)).
- Two permission models: Full Disk Access for reads, Reminders/Calendars/Contacts TCC for writes.

## Evidence

- #49 capability matrix: sections, subtasks, tags, attachments, and flags are not writable
  through EventKit.
- `packages/apple-connector/src/db.rs`, `connect_pool`.
- `docs/openapi.json`: no write operation under `/v1/messages`, `/v1/chats`, or `/v1/notes`.
