---
id: EK-0003
status: proposed
issues:
  - https://github.com/atahanyorganci/apple-connector/issues/76
provenance:
  - https://github.com/atahanyorganci/apple-connector/pull/141
supersedes: [EK-0001]
superseded-by: []
---

# Event status is not writable: the input types have no `status`, and the API rejects it.

## Question

[EK-0001](EK-0001-event-status-writable.md) accepted a status EventKit cannot set. #76 asked to
apply the supported statuses.

## Options

- **Map status onto other EventKit operations** (cancel = remove): changes what the request means.
- **Remove the field.**

## Decision

Remove it. `CreateEventInput` and `UpdateEventInput` have no status field, so dropping one is
unrepresentable in this crate. The HTTP layer rejects `status` on create and update with
`422 immutable_event_field`, `details.field = "status"`. Read responses still report status from
SQLite.

## Consequences

To cancel an event, delete it.

## Evidence

- `packages/apple-eventkit/src/event.rs`: `CreateEventInput` doc comment.
- `packages/apple-connector/tests/mutations_integration.rs`: `event_status_is_rejected_as_immutable`.
