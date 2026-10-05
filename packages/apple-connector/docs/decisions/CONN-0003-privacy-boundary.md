---
id: CONN-0003
status: proposed
issues:
  - https://github.com/atahanyorganci/apple-connector/issues/8
  - https://github.com/atahanyorganci/apple-connector/issues/9
  - https://github.com/atahanyorganci/apple-connector/issues/13
  - https://github.com/atahanyorganci/apple-connector/issues/14
  - https://github.com/atahanyorganci/apple-connector/issues/32
  - https://github.com/atahanyorganci/apple-connector/issues/36
provenance:
  - https://github.com/atahanyorganci/apple-connector/pull/15
  - https://github.com/atahanyorganci/apple-connector/pull/38
supersedes: []
superseded-by: []
---

# Responses never expose filesystem paths, raw payloads, or ciphertext, and request logs carry only route, status, and latency.

## Question

The data is personal. Logs end up in terminals and log files; responses end up in clients. What
may cross each boundary?

## Options

- **Log requests in full** for debugging: query strings carry search terms, senders, and phone
  numbers.
- **Expose raw payloads and paths** so clients can do their own decoding: leaks the user's
  home-directory layout and undecoded private data.
- **A fixed, minimal surface.**

## Decision

A fixed, minimal surface:

- **Request logs**: the matched route template, the status code, and the latency in
  milliseconds. Never the URI, query string, headers, or body.
- **Responses**: API DTOs are separate types from domain models. They carry no stored or
  canonical file path, no raw blob, and no database path; attachments are addressed by id and
  served through a content URL.
- **Locked notes**: the body is never decoded, and neither plaintext nor ciphertext is returned.
  `GET /v1/notes/{id}/contents` returns front matter with an empty body.

## Consequences

- Startup diagnostics are outside this boundary: a configured database path that does not exist
  is logged with the path, and store discovery logs the chosen path at `debug` level. These are
  the operator's own configuration, not request data.
- New DTO fields are reviewed against this list; a field that carries a path is a bug.

## Evidence

- `packages/apple-connector/src/api/middleware.rs`: `trace_request` logs `route`, `status`,
  `latency_ms`; test `trace_middleware_does_not_log_request_uri`.
- `packages/apple-connector/src/notes/decode/mod.rs`: `decode_notedata` returns an empty body when
  `is_locked`; test `locked_notes_never_decode_body`; handler test
  `get_note_contents_locked_note_has_empty_body`.
- `packages/apple-connector/src/lib.rs`: `warn!(path = %path.display(), ...)` for missing configured
  paths.
