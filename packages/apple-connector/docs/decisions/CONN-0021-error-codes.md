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
- A framework "not found" answers with the code of the entity the request addressed
  (`reminder_not_found`, `container_not_found`, …): EventKit and Contacts do not say what was
  missing, so the mappers take it from the call site. Framework validation is
  `eventkit_invalid_input`/`contacts_invalid_input`, framework timeouts
  `eventkit_timeout`/`contacts_timeout`, and body rejections `malformed_request_body` (400),
  `invalid_request_body` (422), or `unsupported_media_type` (415). The last coarse codes went in
  #159.
- `docs/errors.md` is checked against the enum by a test.
- A framework refusal arrives as `Rejected { code, description }`. The response carries this
  project's message and `details.framework_code`; Apple's `localizedDescription` goes to the log
  at `warn`. `ValidationFailed` is reserved for messages the framework crates write themselves.
  Until #158, Apple's text was copied into the 422 `message`.

## Evidence

- `packages/apple-connector/src/api/error.rs`, `error_codes.rs`, `eventkit_convert.rs`
  `map_eventkit_error`, `contacts_convert.rs` `map_contacts_error`, `extract.rs`.
- Tests: `framework_errors_map_to_granular_codes` in `eventkit_convert.rs` and
  `contacts_convert.rs`; `errors_md_lists_every_code` in `error_codes.rs`; body rejection tests in
  `extract.rs`. `framework_validation_text_is_not_returned_to_clients` in both convert files;
  `validation_codes_are_rejections_not_crate_messages` in `apple-eventkit/src/error.rs` and
  `apple-contacts/src/error.rs`.
- `packages/apple-connector/tests/spec.rs`: `malformed_query_and_path_parameters_are_typed_errors`.
