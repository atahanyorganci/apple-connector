---
id: EK-0002
status: superseded
issues:
  - https://github.com/atahanyorganci/apple-connector/issues/52
provenance:
  - https://github.com/atahanyorganci/apple-connector/pull/55
supersedes: []
superseded-by: [EK-0004]
---

# Recurring-event writes take a span of `this`, `future`, or `all`.

## Question

Editing or deleting one occurrence of a recurring event needs a scope.

## Options

- **`this` / `future` / `all`**, the three scopes calendar UIs usually offer.
- **Only what `EKSpan` implements.**

## Decision

Three scopes (#52: `all` as "series master edit").

## Consequences

`EKSpan` has exactly `ThisEvent` and `FutureEvents`. `all` mapped to `FutureEvents`, so earlier
occurrences were silently left untouched (#80).

Superseded by [EK-0004](EK-0004-event-span.md) (#80, PR #141).

## Evidence

- #80: "`EventSpan::All` currently maps identically to `Future`."
