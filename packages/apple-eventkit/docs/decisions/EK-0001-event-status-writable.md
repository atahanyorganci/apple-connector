---
id: EK-0001
status: superseded
issues:
  - https://github.com/atahanyorganci/apple-connector/issues/49
  - https://github.com/atahanyorganci/apple-connector/issues/52
provenance:
  - https://github.com/atahanyorganci/apple-connector/pull/55
supersedes: []
superseded-by: [EK-0003]
---

# Event create and update accept a `status`.

## Question

Calendar events have a status (confirmed, tentative, cancelled). Should writes accept it?

## Options

- **Accept `status`** on create and update.
- **Leave it read-only.**

## Decision

Accept it. #49's capability matrix lists "Completed / status: Yes" for Calendar, and #52's
`CreateEventRequest` includes `status`.

## Consequences

`EKEvent.status` has a getter and no setter, so the value was accepted and silently dropped
(#76).

Superseded by [EK-0003](EK-0003-event-status.md) (#76, PR #141).

## Evidence

- PR #141, "Three issues that turned out smaller than their titles": "`EKEvent.status` has a getter
  and no setter. Apple's own header says you cannot set it and to remove the event to cancel it."
