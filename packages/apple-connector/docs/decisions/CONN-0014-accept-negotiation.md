---
id: CONN-0014
status: superseded
issues:
  - https://github.com/atahanyorganci/apple-connector/issues/39
  - https://github.com/atahanyorganci/apple-connector/issues/45
provenance:
  - https://github.com/atahanyorganci/apple-connector/commit/90ab5a7
  - https://github.com/atahanyorganci/apple-connector/pull/48
supersedes: []
superseded-by: [CONN-0015]
---

# Event endpoints choose JSON, iCalendar, or CalDAV XML from the `Accept` header or `?format=`.

## Question

Calendar events are available as JSON DTOs, iCalendar, and CalDAV XML. How does a client ask for
one?

## Options

- **Content negotiation** on the existing routes: `Accept: text/calendar`,
  `Accept: application/caldav+xml`, or `?format=json|ics|caldav`.
- **Sibling routes** per format.

## Decision

Content negotiation through a `resolve_format()` helper on `GET /v1/events`,
`GET /v1/events/{id}`, and `GET /v1/calendars/{id}/events` (#45), shipped in `90ab5a7`.

## Consequences

One route answering three media types is awkward to describe in OpenAPI and for generated
clients to type.

Superseded the same day by [CONN-0015](CONN-0015-format-routes.md) (`7e7bc97`, still inside
PR #48). PR #48's description still describes the `Accept` design.

## Evidence

- `git log -S resolve_format`: introduced in `90ab5a7`, replaced in `7e7bc97`.
