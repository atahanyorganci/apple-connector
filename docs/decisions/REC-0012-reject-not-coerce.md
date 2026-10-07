---
id: REC-0012
status: proposed
issues:
  - https://github.com/atahanyorganci/apple-connector/issues/107
  - https://github.com/atahanyorganci/apple-connector/issues/51
  - https://github.com/atahanyorganci/apple-connector/issues/76
  - https://github.com/atahanyorganci/apple-connector/issues/77
  - https://github.com/atahanyorganci/apple-connector/issues/80
  - https://github.com/atahanyorganci/apple-connector/issues/98
  - https://github.com/atahanyorganci/apple-connector/issues/106
provenance:
  - https://github.com/atahanyorganci/apple-connector/pull/55
  - https://github.com/atahanyorganci/apple-connector/pull/139
  - https://github.com/atahanyorganci/apple-connector/pull/141
supersedes: [CONN-0011]
superseded-by: []
---

# An input the system cannot honour is rejected with a typed error code, never dropped or coerced.

## Question

The API accepted several inputs it could not act on: an event `status` EventKit cannot set
(#76), reminder coordinates EventKit cannot store (#77), `span: "all"` that silently behaved like
`future` (#80), a `section_id` filter that was parsed and never applied (#98), and location or
unknown alarm kinds coerced into relative alarms (#106). Each one let a client believe something
happened that did not.

## Options

- **Best effort**: accept, apply what can be applied, ignore the rest.
- **Reject**: refuse the request with a code naming the field.

## Decision

Reject. A request field the backing store cannot store is refused before any write, with a
specific `ErrorCode` and `details.field`:

| Input | Code |
| --- | --- |
| reminder `section_id`, `parent_id`, `tags`, `attachments`, `flagged` | `unsupported_reminder_field` |
| reminder location `latitude`/`longitude` | `unsupported_reminder_field` |
| event `status` | `immutable_event_field` |
| alarm kind `location` or unknown | `unsupported_alarm_kind` |
| `span: "all"` | rejected at deserialization, inside the error envelope |

A query parameter that cannot be applied is removed rather than ignored (#98 removed
`section_id` from reminder list and search).

## Consequences

- The same rule applies to the framework crates' input types: a field EventKit cannot store is
  absent from the input struct, so dropping it is unrepresentable (`CreateEventInput` has no
  `status`; `ReminderLocationInput` has no coordinates).
- Clients must not send fields "just in case": `flagged: false` is rejected like `flagged: true`.
  The Raycast "Create Reminder" form does exactly this (see the raycast-extension spec).
- Supersedes [CONN-0011](../../packages/apple-connector/docs/decisions/CONN-0011-section-id-filter.md).

## Evidence

- `packages/apple-connector/src/api/eventkit_convert.rs`: `validate_create_reminder`,
  `reject_reminder_coordinates`, `reject_event_status`, alarm kind mapping.
- `packages/apple-connector/tests/mutations_integration.rs`:
  `event_status_is_rejected_as_immutable`, `event_span_all_is_rejected_with_a_typed_error`.
- `packages/apple-connector/tests/spec.rs`: `flagged_false_is_still_an_unsupported_reminder_field`.
