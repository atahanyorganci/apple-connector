---
id: CONN-0016
status: superseded
issues:
  - https://github.com/atahanyorganci/apple-connector/issues/49
  - https://github.com/atahanyorganci/apple-connector/issues/54
  - https://github.com/atahanyorganci/apple-connector/issues/56
  - https://github.com/atahanyorganci/apple-connector/issues/63
provenance:
  - https://github.com/atahanyorganci/apple-connector/pull/55
  - https://github.com/atahanyorganci/apple-connector/pull/65
supersedes: []
superseded-by: [CONN-0022]
---

# After a write, the handler polls SQLite up to five times, 100 ms apart, before answering.

## Question

A framework write lands in SQLite after an unspecified delay. The response should carry the full
detail DTO, which comes from SQLite ([REC-0005](../../../../docs/decisions/REC-0005-hybrid-read-write.md)).

## Options

- **Poll inside the request**, then fall back to a `sync_pending` envelope.
- **Answer immediately** with whatever one read finds.

## Decision

Poll: up to 5 × 100 ms, then `sync_pending: true` (#49 "Post-save hydrate: poll SQLite up to
5×100ms"; #63 "`sync_pending: true` when SQLite hasn't caught up within 500ms").

## Consequences

Every write held its request for up to half a second of fixed sleeps, and responses still used
200/201 when the envelope said the data was pending.

Superseded by [CONN-0022](CONN-0022-async-mutations.md) (#105, PR #139).

## Evidence

- #105: "Remove in-request hydration sleeps and return **202 Accepted** immediately."
