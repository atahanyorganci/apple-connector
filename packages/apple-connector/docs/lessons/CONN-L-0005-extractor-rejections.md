---
id: CONN-L-0005
status: active
observed-on: "axum 0.8 Json and Query extractors"
graduated-to: ""
provenance:
  - https://github.com/atahanyorganci/apple-connector/pull/141
  - https://github.com/atahanyorganci/apple-connector/commit/12a14eb
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

## Why it is not a test yet

It is enforced for mutations only. `ApiJson` and `ApiQuery` (`api/extract.rs`) are used by the
reminder, event, and contact mutation handlers, and
`event_span_all_is_rejected_with_a_typed_error` covers them. Read handlers still use axum's
`Query`: `GET /v1/messages?before=2024-01-01T00:00:00Z` answers
`Failed to deserialize query string: before: invalid digit found in string` as plain text.
`an_rfc3339_query_bound_is_a_typed_error` in `packages/apple-connector/tests/spec.rs` asserts the
rule and is ignored until the read handlers move to `ApiQuery`.
