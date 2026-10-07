---
id: RAY-0001
status: proposed
issues:
  - https://github.com/atahanyorganci/apple-connector/issues/143
provenance:
  - https://github.com/atahanyorganci/apple-connector/pull/149
supersedes: []
superseded-by: []
---

# The extension's API types come from `docs/openapi.json` through a dependency-free generator.

## Question

The extension calls 70 operations across 178 schemas. Hand-written types drift; the usual
generators did not run.

## Options

- **`openapi-typescript`**: drives the TypeScript compiler API, which the TypeScript 7 native port
  does not provide; it fails with
  `Cannot read properties of undefined (reading 'createKeywordTypeNode')`.
- **Hand-written types.**
- **A small generator** that walks the schema directly and implements only the constructs the
  contract uses.

## Decision

A small generator: `scripts/generate-api-client.mjs`, importing only `node:fs`, `node:path`, and
`node:url`. It emits `src/lib/api.gen.ts` with schema types, operations keyed by `operationId`, a
runtime route table, and value lists for string enums. `pnpm generate:check` fails when the output
is stale ([REC-0002](../../../../docs/decisions/REC-0002-generated-artifacts.md)).

## Consequences

- A construct the generator does not implement fails generation rather than producing a wrong type.
- The contract caught three wrong assumptions at compile time (PR #149): writes return a
  sync-pending envelope, containers and calendar accounts are not paginated, and calendars carry
  no writability flag.
- Binary responses must be documented as `application/octet-stream` (with the real type in a
  `Content-Type` header) to be typed `ArrayBuffer`. The contact photo had no `content` block and
  was typed `void` until #169.

## Evidence

- PR #149, "Generated, not hand-written": 178 schemas, 70 operations, two runs byte-identical.
- `scripts/generate-api-client.mjs` `contentType`: `application/octet-stream` → `ArrayBuffer`.
