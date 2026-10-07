---
id: EK-0004
status: proposed
issues:
  - https://github.com/atahanyorganci/apple-connector/issues/80
provenance:
  - https://github.com/atahanyorganci/apple-connector/pull/141
supersedes: [EK-0002]
superseded-by: []
---

# Recurring-event writes take `this` or `future`, the two scopes `EKSpan` implements.

## Question

[EK-0002](EK-0002-event-span-all.md)'s `all` silently behaved like `future`.

## Options

- **Implement `all`** by finding and editing the series master and every detached occurrence.
- **Remove `all`.**

## Decision

Remove it. `EventSpan` has `This` and `Future`, mapped to `EKSpanThisEvent` and
`EKSpanFutureEvents`. The API defaults to `this` when `span` is omitted; `span: "all"` fails to
deserialize and is answered inside the error envelope.

## Consequences

Editing a whole series means editing its first occurrence with `future`.

## Evidence

- `packages/apple-eventkit/src/event.rs` `EventSpan`.
- `packages/apple-connector/src/api/handlers/event_mutations.rs`:
  `span.unwrap_or(EventSpanDto::This)`.
- `packages/apple-connector/tests/mutations_integration.rs`:
  `event_span_all_is_rejected_with_a_typed_error`.
- Live, ignored: `recurring_event_edit_with_span_this`, `recurring_event_edit_with_span_future` in
  `packages/apple-connector/tests/eventkit_integration.rs`.
