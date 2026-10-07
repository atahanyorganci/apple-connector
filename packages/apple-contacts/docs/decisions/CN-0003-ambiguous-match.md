---
id: CN-0003
status: proposed
issues:
  - https://github.com/atahanyorganci/apple-connector/issues/81
provenance:
  - https://github.com/atahanyorganci/apple-connector/pull/141
supersedes: []
superseded-by: []
---

# Container resolution only falls back to the name on a genuine miss, and ambiguity is a typed 409.

## Question

`containersMatchingPredicate` errors were swallowed by `.ok()?`, so a framework failure looked like
"not found" and fell through to matching by name, which can pick a different account's container.
Ambiguity answered 422 here and 409 in EventKit.

## Options

- **Keep the name fallback for every failure.**
- **Fall back only when the framework says the identifier does not exist.**

## Decision

Resolution tries the stored external identifier, then the API id, by
`predicateForContainersWithIdentifiers:`. Only `CNErrorCodeRecordDoesNotExist` (mapped to
`NotFound`) falls through to the next candidate and finally to a case-insensitive name match; every
other framework error is returned. A container returned under a different identifier than the one
asked for is `NotFound`. Two matches are `ContactsError::AmbiguousMatch` →
`409 ambiguous_contacts_match`, the same status as `ambiguous_event_kit_match`.

## Consequences

The name fallback still exists; two containers with the same name answer 409.

## Evidence

- `packages/apple-contacts/src/container.rs` `resolve_container`, `container_with_identifier`,
  `container_with_name`.
- `packages/apple-connector/src/api/contacts_convert.rs` test
  `ambiguity_answers_the_same_way_as_eventkit`.
