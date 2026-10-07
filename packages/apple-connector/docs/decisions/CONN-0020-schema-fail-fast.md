---
id: CONN-0020
status: proposed
issues:
  - https://github.com/atahanyorganci/apple-connector/issues/23
  - https://github.com/atahanyorganci/apple-connector/issues/96
  - https://github.com/atahanyorganci/apple-connector/issues/99
  - https://github.com/atahanyorganci/apple-connector/issues/100
provenance:
  - https://github.com/atahanyorganci/apple-connector/pull/29
  - https://github.com/atahanyorganci/apple-connector/pull/139
supersedes: [CONN-0012]
superseded-by: []
---

# Core Data entity metadata is read from each store at startup, and a store whose schema does not match stops startup.

## Question

Core Data assigns entity numbers (`Z_ENT`) and join-table names per database. Hardcoding them
works until Apple adds an entity. Unsupported schemas (legacy Calendar, [CONN-0012](CONN-0012-legacy-calendar-fallback.md))
produced false zeroes and request-time failures instead of an error.

## Options

- **Hardcode and hope.**
- **Discover lazily** and fall back to zero when an entity is missing.
- **Discover at startup and abort on mismatch.**

## Decision

Discover at startup and abort on mismatch. Reminders and Notes entity IDs come from
`Z_PRIMARYKEY` by name; Contacts entity IDs and relationship tables are discovered per source.
All are loaded by `AppState::warm_entity_id_caches` before the listener binds, and an error there
ends startup. Zero-ID fallbacks were removed. Legacy `ZCALENDARITEM` Calendar databases are
unsupported: the same gate runs `detect_schema_variant` on the Calendar pool.

## Consequences

- A macOS update that renames an entity stops the server with a schema error rather than serving
  empty lists.
- Calendar joined the gate only in #162; until then a legacy database passed startup and
  `/healthz` and failed each query.

## Evidence

- `packages/apple-connector/src/api/router.rs` `warm_entity_id_caches` (Reminders, Notes,
  Contacts); `src/lib.rs` aborts on its error.
- `packages/apple-connector/src/calendar/schema.rs` `detect_schema_variant`, called from
  `warm_entity_id_caches` and `calendar/inventory.rs`.
- `packages/apple-connector/tests/spec.rs`: `a_legacy_calendar_schema_fails_the_startup_gate`.
