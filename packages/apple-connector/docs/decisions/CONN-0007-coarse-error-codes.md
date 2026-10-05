---
id: CONN-0007
status: superseded
issues:
  - https://github.com/atahanyorganci/apple-connector/issues/14
provenance:
  - https://github.com/atahanyorganci/apple-connector/pull/15
supersedes: []
superseded-by: [CONN-0021]
---

# Errors carry a small set of codes aligned with HTTP statuses.

## Question

The first API needed a structured error body for 400/404/500/503 responses.

## Options

- **HTTP-aligned codes**: `validation_error`, `not_found`, `service_unavailable`, … — one per
  status.
- **Granular codes**: one per failure the client can act on.

## Decision

HTTP-aligned codes in an `{ "error": { "code", "message", "details" } }` envelope (#14:
"structured 400/404/503/500 mappings").

## Consequences

Clients could not tell a missing chat from a missing attachment, or a closed Messages database
from denied EventKit access, without parsing `message`. About 126 call sites built errors with
`ApiError::internal(error.to_string())`, sending SQL and framework text to clients (#102).

Superseded by [CONN-0021](CONN-0021-error-codes.md) (#129–#138, PR #139).

## Evidence

- #129: "Replace the current 7 coarse HTTP-aligned error codes (`not_found`, `validation_error`,
  etc.) with a flat registry of ~60–80 granular snake_case codes."
