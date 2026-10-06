---
id: CONN-0021
status: proposed
issues:
  - https://github.com/atahanyorganci/apple-connector/issues/102
  - https://github.com/atahanyorganci/apple-connector/issues/129
  - https://github.com/atahanyorganci/apple-connector/issues/130
  - https://github.com/atahanyorganci/apple-connector/issues/137
  - https://github.com/atahanyorganci/apple-connector/issues/138
provenance:
  - https://github.com/atahanyorganci/apple-connector/pull/139
  - https://github.com/atahanyorganci/apple-connector/pull/141
supersedes: [CONN-0007]
superseded-by: []
---

# Errors carry a granular, documented `ErrorCode`, and backend error text never reaches clients.

## Question

[CONN-0007](CONN-0007-coarse-error-codes.md)'s seven codes forced clients to parse messages, and
handlers sent SQL and framework text in them.

## Options

- **Keep coarse codes, scrub messages.**
- **A registry of granular codes**, each with a fixed HTTP status and default message.

## Decision

A registry. `ErrorCode` (`api/error_codes.rs`) is a flat enum of snake_case codes; each has
`http_status()` and `default_message()`. Every error is
`{ "error": { "code", "message", "details" } }`, with `details` carrying structured context such as
the field or entity ID. Handlers build errors with `ApiError::new`, `with_message`, or
`with_details`; database errors go through `ApiError::from_sqlx`, which maps timeouts to
`query_timeout` and everything else to `internal_error` without the driver text. Every code is in
OpenAPI (`components.schemas.ErrorCode`) and in `docs/errors.md`. `scripts/check-api-error-leakage.sh`
fails CI on `ApiError::internal(...to_string())` and on the old coarse helpers.

## Consequences

- Clients branch on `code`, never on `message`.
- Every handler extracts through `ApiJson`, `ApiQuery`, and `ApiPath`, so a malformed body, query
  string, or path segment is a typed error. Read handlers used axum's `Query` until #160; the
  leakage script now bans the raw extractors.
- Known bugs, each with a failing ignored test:
  - The framework mappers still emit coarse codes for some outcomes: `resource_not_found`,
    `unprocessable_entity`, `gateway_timeout`, and `validation_error` (body rejections).
  - EventKit and Contacts `ValidationFailed` carry `NSError.localizedDescription`, and the mappers
    copy it into `message`, so framework text reaches clients. The leakage script does not catch
    this pattern.

## Evidence

- `packages/apple-connector/src/api/error.rs`, `error_codes.rs`, `eventkit_convert.rs`
  `map_eventkit_error`, `contacts_convert.rs` `map_contacts_error`, `extract.rs`.
- Tests (ignored, fail today): `framework_validation_text_is_not_returned_to_clients` and
  `framework_errors_map_to_granular_codes` in both `eventkit_convert.rs` and
  `contacts_convert.rs`.
- `packages/apple-connector/tests/spec.rs`: `malformed_query_and_path_parameters_are_typed_errors`.
