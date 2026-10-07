---
id: CN-0002
status: proposed
issues:
  - https://github.com/atahanyorganci/apple-connector/issues/81
provenance:
  - https://github.com/atahanyorganci/apple-connector/pull/141
  - https://github.com/atahanyorganci/apple-connector/commit/2d0cf9b
supersedes: [CN-0001]
superseded-by: []
---

# The Contacts framework alone decides whether a container accepts writes; the API exposes no writability flag.

## Question

[CN-0001](CN-0001-sqlite-writability-hint.md)'s SQLite flag was always `false`. #81 asked to
compute it from the container type instead.

## Options

- **Compute `read_only` from `ZTYPE`**: no documented mapping exists; any value is a guess.
- **Remove the flag**: the framework answers at save time with
  `CNErrorCodeRecordNotWritable`, `ParentContainerNotWritable`, or
  `NoAccessableWritableContainers`.

## Decision

Remove it, from the resolve hint and from `ContainerSummaryDto`. Those three framework codes map
to `ContactsError::ReadOnlyContainer` and `403 read_only_container`. This was a breaking response
change beyond what #81 asked for (pre-1.0, [REC-0011](../../../../docs/decisions/REC-0011-pre-1-0-breaking.md));
PR #141 flagged it for a second opinion.

## Consequences

- Clients learn a container is read-only only by trying to write to it.
- Calendars have the same shape: no writability flag on reads, `calendar_read_only` at write time.

## Evidence

- `packages/apple-contacts/src/container.rs` `ContainerResolveHint` doc comment.
- `packages/apple-contacts/src/error.rs` `map_cn_error`; test
  `not_writable_codes_map_to_read_only_container`.
- `docs/openapi.json`: `ContainerSummaryDto` properties are `container_type`, `id`, `name`,
  `source_id`.
