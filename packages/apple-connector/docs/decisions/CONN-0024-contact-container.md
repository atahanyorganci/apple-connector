---
id: CONN-0024
status: proposed
issues:
  - https://github.com/atahanyorganci/apple-connector/issues/153
provenance:
  - https://github.com/atahanyorganci/apple-connector/pull/154
  - https://github.com/atahanyorganci/apple-connector/commit/1d06251
supersedes: []
superseded-by: []
---

# A contact with no stored container belongs to its source's only container, or to none if the source has several.

## Question

macOS leaves `ZCONTAINER` NULL on contacts in a source with a single container
([CONN-L-0002](../lessons/CONN-L-0002-implied-container.md)). The list paths dropped those rows,
so `/v1/contacts` was empty on a real AddressBook while detail lookups worked.

## Options

- **Drop rows without a container**: what happened; violates
  [CONN-0019](CONN-0019-no-synthetic-data.md).
- **Always `null`**: keeps the row but loses an attribution macOS implies.
- **Infer when unambiguous**: the source's single `CNCDContainer`; `null` when there are several.

## Decision

Infer when unambiguous. Contact queries join on
`COALESCE(r.ZCONTAINER, <the source's only CNCDContainer row>)`. When a source has several
containers and the row stores none, the contact is listed with `container_id: null` and is not
attributed to any container, including by the `container_id` filter.

## Consequences

- The `container_id` filter and group members see uncontained contacts the same way lists do.
- A source that gains a second container stops attributing its uncontained contacts.

## Evidence

- Observed on macOS 27: one iCloud source, 1005 contacts, all with `ZCONTAINER` NULL, one
  `CNCDContainer` row (#153).
- `packages/apple-connector/src/contacts/queries.rs` module comment and joins.
- `packages/apple-connector/tests/contacts_integration.rs`:
  `integration_contacts_with_null_container_are_listed`,
  `integration_contacts_with_ambiguous_container_are_listed_without_one`.
