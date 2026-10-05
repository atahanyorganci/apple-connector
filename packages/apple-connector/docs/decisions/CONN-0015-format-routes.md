---
id: CONN-0015
status: proposed
issues:
  - https://github.com/atahanyorganci/apple-connector/issues/45
  - https://github.com/atahanyorganci/apple-connector/issues/61
provenance:
  - https://github.com/atahanyorganci/apple-connector/commit/7e7bc97
  - https://github.com/atahanyorganci/apple-connector/pull/48
  - https://github.com/atahanyorganci/apple-connector/pull/65
supersedes: [CONN-0014]
superseded-by: []
---

# Interchange formats are served from sibling suffix routes; the base routes are JSON only.

## Question

[CONN-0014](CONN-0014-accept-negotiation.md) put three media types behind one route.

## Options

- **Keep `Accept` negotiation.**
- **Sibling routes**, one media type per route.

## Decision

Sibling routes. The base route returns JSON; a suffix selects the format:

| Base | iCalendar / vCard | CalDAV / CardDAV |
| --- | --- | --- |
| `/v1/events`, `/v1/events/{id}`, `/v1/calendars/{id}/events` | `…/iCal` (`text/calendar`) | `…/caldav` (`application/caldav+xml`) |
| `/v1/contacts`, `/v1/contacts/{id}`, `/v1/groups/{id}/contacts` | `…/vcard` (`text/vcard`) | `…/carddav` (`application/carddav+xml`) |

The `Accept` header is ignored, and `POST /v1/events/parse` from #45 was not kept.

## Consequences

- Each operation has exactly one response media type, which generated clients type cleanly.
- The suffixes are inconsistently cased: `/iCal` for Calendar, `/vcard` for Contacts.
- Writes accept JSON only. #56 and #63 planned vCard and CardDAV request bodies for contact
  writes, and commit `33709df` says it added them, but no handler ever read them.
- An unused `preferred_format` helper for `Accept` remains in `api/handlers/contacts.rs`
  behind `#[allow(dead_code)]`.

## Evidence

- `docs/openapi.json` paths; every base route's 200 response has only `application/json`.
- `docs/openapi.json`: every write operation's request body is `application/json` only.
- `git show 33709df -- packages/apple-connector/src/api/handlers/contact_mutations.rs` has no
  vCard or CardDAV body handling.
