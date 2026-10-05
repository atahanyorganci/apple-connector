---
id: CONN-0010
status: proposed
issues:
  - https://github.com/atahanyorganci/apple-connector/issues/12
  - https://github.com/atahanyorganci/apple-connector/issues/17
  - https://github.com/atahanyorganci/apple-connector/issues/30
  - https://github.com/atahanyorganci/apple-connector/issues/40
  - https://github.com/atahanyorganci/apple-connector/issues/50
  - https://github.com/atahanyorganci/apple-connector/issues/57
provenance:
  - https://github.com/atahanyorganci/apple-connector/pull/15
  - https://github.com/atahanyorganci/apple-connector/pull/29
  - https://github.com/atahanyorganci/apple-connector/pull/38
  - https://github.com/atahanyorganci/apple-connector/pull/48
  - https://github.com/atahanyorganci/apple-connector/pull/65
supersedes: []
superseded-by: []
---

# Stores are found automatically, Messages is required, every other store degrades to "unavailable", and `/healthz` reports each one.

## Question

Five stores live in five places, some with several candidate files (Reminders keeps one SQLite
file per account; Contacts one per AddressBook source). Users run the server without all of them,
or without Full Disk Access to some.

## Options

- **Require every store**: one missing app stops the whole server.
- **Require explicit paths**: hostile to the common case.
- **Discover, and degrade per store.**

## Decision

Discover, and degrade per store. Each store has a CLI flag and, for most, an environment variable
that overrides discovery:

| Store | Default | Candidate choice |
| --- | --- | --- |
| Messages | `~/Library/Messages/chat.db` | — |
| Reminders | `~/Library/Group Containers/group.com.apple.reminders/Container_v1/Stores/Data-*.sqlite` | most non-deleted reminders, then newest mtime, then highest `Z_MAX` |
| Notes | `~/Library/Group Containers/group.com.apple.notes/NoteStore.sqlite` | — |
| Calendar | `~/Library/Group Containers/group.com.apple.calendar/Calendar.sqlitedb` | — |
| Contacts | `~/Library/Application Support/AddressBook/Sources/*/` | every source; newest `AddressBook-v*.abcddb` in each |

Messages is required: a missing `chat.db` aborts startup. Any other store that is missing or
cannot be opened is logged, its pool is absent, and its routes answer
`503 <domain>_database_unavailable`. `/healthz` returns 200 only when all five pools answer
`SELECT 1`, otherwise 503; the body always reports each store, plus EventKit and Contacts
authorization, which never affect the status code.

## Consequences

- A Reminders account with more reminders than the one the user means wins discovery; pass
  `--reminders-database` to choose.
- `/healthz` is 503 on any machine without all five stores, so it is a readiness report, not a
  liveness probe.

## Evidence

- `packages/apple-connector/src/lib.rs` `run`; `reminders/discovery.rs`
  `discover_reminders_database`; `contacts/discovery.rs`.
- `packages/apple-connector/src/api/handlers/health.rs` `healthz`.
- `packages/apple-connector/tests/spec.rs`: `a_missing_messages_database_aborts_startup`.
