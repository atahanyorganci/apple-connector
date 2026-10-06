---
id: CONN-L-0005
status: graduated
observed-on: "axum 0.8 Json and Query extractors"
graduated-to: "scripts/check-api-error-leakage.sh; packages/apple-connector/tests/spec.rs"
provenance:
  - https://github.com/atahanyorganci/apple-connector/pull/141
  - https://github.com/atahanyorganci/apple-connector/commit/12a14eb
  - https://github.com/atahanyorganci/apple-connector/issues/160
---

# axum's `Json` and `Query` extractors answer a malformed request with plain text, before the handler runs.

## What happened

Removing `all` from the span enum made `{"span": "all"}` fail to deserialize. The response was
`422` with a plain-text body — outside the `ErrorCode` catalog and the
`{ "error": { … } }` envelope, so typed clients could not read it (PR #141).

## Why

An extractor's rejection is its own `IntoResponse`, produced before the handler and its error
mapping run.

## Rule

Every extractor that can reject is wrapped so its rejection becomes an `ApiError` with a typed
code.

## Graduated

- `packages/apple-connector/src/api/extract.rs` wraps `Json`, `Query`, and `Path` as `ApiJson`,
  `ApiQuery`, and `ApiPath`, and every handler uses them (#160).
- `scripts/check-api-error-leakage.sh` (flake check `workspace-api-error-leakage`) fails on a raw
  `Query(..)`, `Path(..)`, or `Json(..)` extractor in `api/handlers`.
- `packages/apple-connector/tests/spec.rs::malformed_query_and_path_parameters_are_typed_errors`:
  `GET /v1/messages?before=2024-01-01T00:00:00Z` and `GET /v1/chats/abc` answer
  `400 invalid_parameter` inside the envelope; `event_span_all_is_rejected_with_a_typed_error`
  covers bodies.
