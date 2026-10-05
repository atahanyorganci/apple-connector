---
id: CN-0001
status: superseded
issues:
  - https://github.com/atahanyorganci/apple-connector/issues/56
  - https://github.com/atahanyorganci/apple-connector/issues/62
  - https://github.com/atahanyorganci/apple-connector/issues/63
provenance:
  - https://github.com/atahanyorganci/apple-connector/pull/65
supersedes: []
superseded-by: [CN-0002]
---

# Container writability comes from the SQLite read path and is checked before the framework write.

## Question

Some containers (read-only CardDAV, directory services) refuse writes. Where does the server learn
that?

## Options

- **From SQLite**: carry a `read_only` flag in the container hint and reject before calling the
  framework (`403`).
- **From the framework**: let `CNContactStore` refuse at save time.

## Decision

From SQLite. #63: "Resolve metadata from SQLite before write (403 for read-only containers, smart
groups)"; `ContainerSummaryDto` exposed `read_only`.

## Consequences

The SQLite value is `ZTYPE` on the AddressBook source record, and there is no documented mapping
from it to writability. `container_from_row` hardcoded `read_only: false`, so the pre-write guards
in `create_contact` and `create_group` never fired (PR #141).

Discussed in PR #141 (reviewer note 2, "deserves a second opinion") and superseded by
[CN-0002](CN-0002-container-writability.md).

## Evidence

- PR #141, "Two defects the issues did not mention" and "Reviewer notes — four deviations from the
  plan", item 2.
