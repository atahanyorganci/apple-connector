---
id: CONN-0019
status: proposed
issues:
  - https://github.com/atahanyorganci/apple-connector/issues/28
  - https://github.com/atahanyorganci/apple-connector/issues/108
  - https://github.com/atahanyorganci/apple-connector/issues/109
  - https://github.com/atahanyorganci/apple-connector/issues/110
  - https://github.com/atahanyorganci/apple-connector/issues/153
provenance:
  - https://github.com/atahanyorganci/apple-connector/pull/29
  - https://github.com/atahanyorganci/apple-connector/pull/128
  - https://github.com/atahanyorganci/apple-connector/pull/154
supersedes: []
superseded-by: []
---

# Identifiers and Apple codes are validated types from the row layer inward, and missing data stays missing.

## Question

Raw strings and integers flowed from rows to DTOs. Assembly code filled gaps with plausible
values — `"unknown"`, empty IDs, epoch zero — and list code dropped rows it could not complete.
Each made corrupt or partial data look real, or made real data vanish.

## Options

- **Defaults**: always produce a complete-looking record.
- **Types and absence**: typed IDs and codes, `Option` for anything the store can leave empty,
  and partial records rather than dropped ones.

## Decision

Types and absence:

- Only row structs hold raw ID strings. Domain models and DTOs use distinct newtypes
  (`ChatId`, `ReminderId`, `EventId`, `ContactId`, …), and path parameters are validated into
  them before any query runs; an empty ID is `400 invalid_parameter`.
- Apple numeric codes become validated types with an explicit unknown variant
  (`ReminderPriority` keeps 0–9 without bucketing loss).
- A missing relationship, title, or timestamp is `None`; cursors never default a key.
- List endpoints never drop a row because a field is missing (#153: contacts without a stored
  container vanished from every list).

## Consequences

- Clients see `null` where Apple stores nothing.
- ID validation checks only that the value is non-empty. The per-type shape checks #28 planned
  (36-character lowercase UUIDs, 256-character GUIDs) were not implemented; repository lookups
  lowercase UUID ids instead, so an uppercase id still resolves.
- Every value derived from the store has a type that names its unit and range.

## Evidence

- `packages/apple-connector/src/apple_types/ids.rs` (`string_id!` `try_new`: non-empty only),
  `codes.rs` (`ReminderPriority::try_new` accepts 0–9 and round-trips each value).
- `packages/apple-connector/tests/contacts_integration.rs`:
  `integration_contacts_with_null_container_are_listed`.
