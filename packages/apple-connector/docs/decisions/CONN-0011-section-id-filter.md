---
id: CONN-0011
status: superseded
issues:
  - https://github.com/atahanyorganci/apple-connector/issues/22
provenance:
  - https://github.com/atahanyorganci/apple-connector/pull/29
supersedes: []
superseded-by: [REC-0012]
---

# Reminder list and search endpoints accept a `section_id` filter.

## Question

Reminders sections are readable from SQLite (membership lives as JSON on the list row, #22) but
not through EventKit. Should clients be able to filter by section?

## Options

- **Expose a `section_id` filter** on list and global reminder queries.
- **Expose `section_id` on the DTO only.**

## Decision

Expose the filter (#22: "Filter: `section_id` on list/global reminder queries").

## Consequences

The parameter was parsed and documented but never bound in SQL, so it was a silent no-op (#98).

Superseded by [REC-0012](../../../../docs/decisions/REC-0012-reject-not-coerce.md): #98 removed the
parameter rather than keep accepting it. `section_id` on reminder DTOs is unchanged.

## Evidence

- #98 comment: "removed the no-op `section_id` list/search query parameter. Response `section_id`
  on reminder DTOs is unchanged."
