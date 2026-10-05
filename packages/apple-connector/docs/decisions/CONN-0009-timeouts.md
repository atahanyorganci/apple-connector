---
id: CONN-0009
status: proposed
issues:
  - https://github.com/atahanyorganci/apple-connector/issues/9
  - https://github.com/atahanyorganci/apple-connector/issues/101
  - https://github.com/atahanyorganci/apple-connector/issues/131
provenance:
  - https://github.com/atahanyorganci/apple-connector/pull/15
  - https://github.com/atahanyorganci/apple-connector/pull/139
supersedes: []
superseded-by: []
---

# Every query and every request has a bound, and running out of it is a 504 with its own code.

## Question

The stores are live SQLite files held open by Apple's apps, which take write locks. A query can
wait on a lock; a request can wait on a framework prompt. Without bounds, requests hang; with the
wrong mapping, a timeout looks like an internal error (#101: query timeouts surfaced as 500).

## Options

- **Rely on SQLite's busy timeout alone.**
- **Bound each layer and map each bound to a status.**

## Decision

Bound each layer:

| Bound | Value | Outcome |
| --- | --- | --- |
| SQLite busy timeout | 5 s | waits for Apple's write locks |
| Pool acquire | 5 s | `504 query_timeout` |
| Query (`run_timed_query`) | 15 s | `504 query_timeout` |
| Request, until the response head | 30 s | `504 request_timeout` |
| Request on a path ending in `/content`, until the response head | 300 s | `504 request_timeout` |
| Response body | unbounded | — |

## Consequences

- Repository calls are wrapped in `run_timed_query`; `ApiError::from_sqlx` maps
  `PoolTimedOut` to `query_timeout` and every other database error to `internal_error`, without
  the driver text.
- The request timeout wraps the handler until it returns its response head; the body is streamed
  afterwards, outside the middleware. Downloads are therefore never cut off, which is what #13
  asked for ("Keep media streaming outside the short JSON/database response timeout").
- The `/content` suffix that selects the 300 s budget only matters for a handler that is slow to
  produce its head. Event attachments (`/v1/events/{id}/attachments/{attachment_id}`) have no
  such suffix and get 30 s, which covers their bounded query and file checks.

## Evidence

- `packages/apple-connector/src/db.rs`: `BUSY_TIMEOUT`, `ACQUIRE_TIMEOUT`, `QUERY_TIMEOUT`,
  `run_timed_query`.
- `packages/apple-connector/src/api/middleware.rs`: `JSON_REQUEST_TIMEOUT`,
  `MEDIA_REQUEST_TIMEOUT`, `request_timeout` (`ends_with("/content")`).
- `packages/apple-connector/src/api/error.rs` tests `from_sqlx_maps_pool_timeout_to_query_timeout`,
  `request_timeout_is_gateway_timeout`.
- `packages/apple-connector/src/api/middleware.rs` test
  `request_timeout_does_not_bound_the_response_body`: the response comes back while its body is
  still pending.
