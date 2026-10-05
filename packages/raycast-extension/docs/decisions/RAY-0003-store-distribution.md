---
id: RAY-0003
status: open
issues:
  - https://github.com/atahanyorganci/apple-connector/issues/142
provenance:
  - https://github.com/atahanyorganci/apple-connector/pull/149
supersedes: []
superseded-by: []
---

# How the extension is distributed, given that it needs a separately installed local server. _(undecided)_

## Question

The extension does nothing without `apple-connector` running locally with Full Disk Access and
EventKit/Contacts permissions. Raycast Store review may not accept an extension with that
requirement (#142, "Open risks").

## Options

- **Publish to the Raycast Store**, documenting the server as a prerequisite.
- **Self-publish** (install from source with `ray develop` / `ray build`).
- **Bundle the server** with the extension: a signed binary needing its own TCC grants.

## Decision

Not decided. The extension is currently built and run from source.

## Consequences

Until decided, the only documented way to run the extension is from source with `pnpm dev`
(README).

## Evidence

- #142, "Open risks": "Store distribution is unresolved."
