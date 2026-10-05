---
id: CONN-0006
status: proposed
issues:
  - https://github.com/atahanyorganci/apple-connector/issues/11
  - https://github.com/atahanyorganci/apple-connector/issues/20
  - https://github.com/atahanyorganci/apple-connector/issues/33
provenance:
  - https://github.com/atahanyorganci/apple-connector/pull/15
  - https://github.com/atahanyorganci/apple-connector/pull/29
  - https://github.com/atahanyorganci/apple-connector/pull/38
supersedes: []
superseded-by: []
---

# Text search over decoded content is a bounded scan that returns a resume point.

## Question

Message text often exists only inside `attributedBody`, and note text only inside the
gzip + protobuf body. SQL cannot search either, and the databases are read-only, so no index can
be added.

## Options

- **Build a full-text index**: needs a writable copy and a sync process.
- **Scan everything per request**: unbounded latency on large histories.
- **Bounded scan**: examine a fixed number of candidate rows per request, return matches, and hand
  back a cursor at the scan position even when the page is short or empty.

## Decision

Bounded scan where content must be decoded: Messages, Reminders, and Notes scan at most 500
candidate rows per request (Messages and Notes in chunks of 100). The search cursor records the
scan position and the filter snapshot. Metadata filters are applied in SQL first. Calendar and
Contacts search columns SQL can read, so they use `LIKE` with no scan budget. `q` is at most 256
characters everywhere.

## Consequences

- A sparse search can return an empty page with `has_more: true`. Clients keep following the
  cursor rather than treating an empty page as the end.
- `LIKE` patterns are built as `%q%` without an escape clause, so `%` and `_` in `q` are
  wildcards for Calendar and Contacts.

## Evidence

- `packages/apple-connector/src/messages/search.rs` (`MESSAGE_SCAN_BUDGET` 500,
  `CANDIDATE_CHUNK_SIZE` 100), `notes/search.rs` (same), `reminders/search.rs`
  (`REMINDER_SCAN_BUDGET` 500).
- `packages/apple-connector/src/api/params.rs`: `MAX_SEARCH_QUERY_LEN` 256.
- `packages/apple-connector/tests/spec.rs`: `contact_search_treats_percent_as_a_wildcard`.
