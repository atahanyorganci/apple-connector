---
id: CONN-0005
status: proposed
issues:
  - https://github.com/atahanyorganci/apple-connector/issues/10
  - https://github.com/atahanyorganci/apple-connector/issues/20
  - https://github.com/atahanyorganci/apple-connector/issues/33
  - https://github.com/atahanyorganci/apple-connector/issues/45
  - https://github.com/atahanyorganci/apple-connector/issues/97
provenance:
  - https://github.com/atahanyorganci/apple-connector/pull/15
  - https://github.com/atahanyorganci/apple-connector/pull/29
  - https://github.com/atahanyorganci/apple-connector/pull/139
  - https://github.com/atahanyorganci/apple-connector/pull/154
supersedes: []
superseded-by: []
---

# Lists use keyset pagination only, with opaque cursors bound to the filters that produced them.

## Question

The stores are live: rows arrive while a client pages. Offsets skip or repeat rows when that
happens, and Messages histories reach hundreds of thousands of rows.

## Options

- **Offset/limit**: simple; unstable under inserts; slow at depth.
- **Keyset**: stable under inserts and fast at any depth; cursors must encode the sort key.

## Decision

Keyset only, no offsets:

- `limit` defaults to 50, maximum 200, on every paginated endpoint.
- Responses carry `page.has_more` and `page.next_cursor`.
- A cursor is `v1.` followed by base64url JSON of the sort key. It is opaque to clients;
  malformed or wrong-version cursors are `400 invalid_cursor`.
- A cursor produced under a set of filters is valid only for those filters; reusing it with
  different filters is `400 invalid_cursor` ("cursor does not match the active filters").
- Contacts span several AddressBook sources; its cursor records the source and that source's row,
  and sources are consumed in ascending `source_id` order.

## Consequences

- Filter binding is enforced for Messages, Reminders, Notes, and Events. **Contacts cursors are
  not bound to their filters** — recorded as a known bug in the spec.
- Filtered event listings broke this once: the handler decoded a filter-bound cursor while the
  repository encoded a filter-free one, so page 2 was always `invalid_cursor` (fixed in PR #154).
- `/v1/containers` and `/v1/calendar-accounts` are not paginated.

## Evidence

- `docs/openapi.json`: all 24 `limit` parameters are `default: 50`, `maximum: 200`.
- `packages/apple-connector/src/api/cursor.rs`: `encode`, `decode`, `decode_*_search_cursor`,
  `ContactListCursor` (no filter snapshot).
- `packages/apple-connector/tests/spec.rs`: `contact_cursors_are_bound_to_their_filters`
  (ignored, fails today).
