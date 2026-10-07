---
id: REC-0013
status: proposed
issues:
  - https://github.com/atahanyorganci/apple-connector/issues/94
  - https://github.com/atahanyorganci/apple-connector/issues/92
provenance:
  - https://github.com/atahanyorganci/apple-connector/pull/140
supersedes: [REC-0004]
superseded-by: []
---

# The `serde-*` interchange crates are concrete codecs over their own model types.

## Question

[REC-0004](REC-0004-serde-generic-formats.md) made the four interchange crates look like Serde
formats, but each parsed one model and bridged to `T` through `serde_json::Value`.

## Options

- **Real Serde formats**: write true `Serializer`/`Deserializer` implementations for iCalendar,
  vCard, and the DAV XML shapes. Large, and the formats do not map cleanly onto Serde's data model.
- **Concrete codecs**: `from_str(&str) -> Result<Model>`, `to_string(&Model) -> Result<String>`,
  plus explicit multistatus entry points.

## Decision

Concrete codecs. `serde-icalendar` reads and writes `CalendarEvent`; `serde-vcard` reads and
writes `VCard`; `serde-caldav` and `serde-carddav` read and write `CalDavMultistatus` /
`CardDavMultistatus` with explicit `parse_multistatus` and `multistatus_to_string`. The models
still derive `Serialize`/`Deserialize` so callers can put them in JSON. `apple-typedstream` is the
exception: it is a real Serde format and keeps `from_slice<T>`.

## Consequences

- Multistatus documents are serialized once, so a page is one XML document (fixed the N roots).
- The crate names and `Cargo.toml` descriptions still say "Serde Serializer and Deserializer";
  the descriptions are stale (recorded under Known bugs in each crate's spec).

## Evidence

- `packages/serde-*/src/lib.rs` public items.
- PR #140, "#94 typed APIs".
