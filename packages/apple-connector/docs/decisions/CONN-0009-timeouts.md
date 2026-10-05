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
| JSON request | 30 s | `504 request_timeout` |
| Media request (path ends in `/content`) | 300 s | `504 request_timeout` |

## Consequences

- Repository calls are wrapped in `run_timed_query`; `ApiError::from_sqlx` maps
  `PoolTimedOut` to `query_timeout` and every other database error to `internal_error`, without
  the driver text.
- **Known bug**: the media timeout is chosen by the `/content` suffix, so
  `/v1/events/{id}/attachments/{attachment_id}`, which streams bytes, gets the JSON timeout.

## Evidence

- `packages/apple-connector/src/db.rs`: `BUSY_TIMEOUT`, `ACQUIRE_TIMEOUT`, `QUERY_TIMEOUT`,
  `run_timed_query`.
- `packages/apple-connector/src/api/middleware.rs`: `JSON_REQUEST_TIMEOUT`,
  `MEDIA_REQUEST_TIMEOUT`, `request_timeout` (`ends_with("/content")`).
- `packages/apple-connector/src/api/error.rs` tests `from_sqlx_maps_pool_timeout_to_query_timeout`,
  `request_timeout_is_gateway_timeout`.
