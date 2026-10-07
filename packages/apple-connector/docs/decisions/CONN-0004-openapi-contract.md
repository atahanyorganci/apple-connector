---
id: CONN-0004
status: proposed
issues:
  - https://github.com/atahanyorganci/apple-connector/issues/14
  - https://github.com/atahanyorganci/apple-connector/issues/121
provenance:
  - https://github.com/atahanyorganci/apple-connector/pull/15
  - https://github.com/atahanyorganci/apple-connector/pull/126
supersedes: []
superseded-by: []
---

# The OpenAPI 3.1 document generated from the handlers is the HTTP contract, and routes are registered once.

## Question

Clients (first curl, later the Raycast extension) need a precise description of every route,
parameter, response, and header. Hand-written docs and a hand-maintained route list each drift.

## Options

- **Hand-written OpenAPI**: drifts from the code.
- **Code-first with utoipa**: annotations next to handlers; the document is exported.

## Decision

Code-first. Every handler carries a `#[utoipa::path]` annotation; routes are registered through
`utoipa_axum`'s `OpenApiRouter` with `routes!`, so registering a route and documenting it are the
same call. The spec is served at `/openapi.json`, rendered at `/docs`, and exported to
`docs/openapi.json` ([REC-0002](../../../../docs/decisions/REC-0002-generated-artifacts.md)).
Contract tests derive the route and operation inventory from the generated spec (#121).

## Consequences

- Adding a route is one registration change; the contract test fails if the export is stale.
- DTOs are API types, separate from domain models
  ([CONN-0003](CONN-0003-privacy-boundary.md)).

## Evidence

- `packages/apple-connector/src/api/router.rs`: `openapi_router()` built on
  `OpenApiRouter::with_openapi(ApiDoc::openapi())`.
- `packages/apple-connector/src/api/doc.rs`: `exported_openapi_matches_committed_contract`.
