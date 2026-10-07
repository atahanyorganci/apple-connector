---
name: spec
description: Spec is a living document at `packages/<crate>/docs/SPEC.md` that describes the contract between a crate (codec, framework wrapper, or application) and its users: public API, guarantees with the tests that enforce them, limits, and platform requirements.
---

# /spec

Each crate has a `packages/<crate>/docs/SPEC.md`: a living document describing the contract between the crate and its users. Implementation details (wire format specifics, SQL, schema mappings) go in `packages/<crate>/docs/spec/**/*.md`, linked from `SPEC.md`.

## Kinds of crate

Every crate is one of three kinds. The kind decides what the contract covers.

| Kind            | Crates                                                                                         | The contract covers                                                                                                                                                                                         |
| --------------- | ---------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Codec**       | `serde-vcard`, `serde-carddav`, `serde-caldav`, `serde-icalendar`, `apple-typedstream`, `apple-notes-protobuf` | Formats and RFC sections supported; round-trip guarantees and what is lossy; size limits (`*_with_limit`, `Limits`); the error type; never panics on any input (backed by its `fuzz/` target).                 |
| **Framework**   | `apple-eventkit`, `apple-contacts`                                                             | The store handle (`EventKitStore`, `ContactsStore`) and its worker-thread ownership; TCC authorization states; which fields are writable and which are rejected; the error enum; macOS-only by `compile_error!`. |
| **Application** | `apple-connector` (bins `apple-connector`, `export-openapi`), `raycast-extension`               | Entry points; CLI flags, configuration, and store auto-discovery; the domains served and their read vs write source; required permissions; network defaults (loopback, unauthenticated).                    |

## Rules

- **Don't duplicate rustdoc or OpenAPI.** Item-level behaviour lives in doc comments. The HTTP wire contract lives in `docs/openapi.json` and error codes in `docs/errors.md`; the spec links to them instead of restating them.
- **Every guarantee is checkable.** Each guarantee names what enforces it: a test (`tests/round_trip.rs::name`), a fuzz target, a fixture, or a `compile_error!`. A guarantee with nothing enforcing it is listed under "Not yet enforced" until it is. Graduated lessons land here.
- **Record disagreements as bugs.** When the code does not do what the spec (or a decision record) says, the spec keeps the intended behaviour and lists the gap under "Known bugs", with an `#[ignore = "bug: …"]` test that asserts the intended behaviour and fails today. The fixing PR removes the `ignore` and the entry.
- **State non-goals.** What the crate deliberately does not do (e.g. Messages is read-only; vCard 2.1 is not supported) is part of the contract.
- **Keep it current.** A PR that changes a crate's public API, guarantees, or limits updates its `SPEC.md` in the same PR.
- Skip sections that do not apply to the crate's kind.

## Template

```md
# `<crate>`: _summary_

_2–3 sentences: what it is, who uses it._

## Kind

_codec | framework | application_

## Public API

_Entry points (functions, store handles, binaries), the error type, and the key types. Link to rustdoc for details._

## Guarantees

- _Guarantee._ Enforced by: _test / fuzz target / fixture / `compile_error!`._

### Not yet enforced

- _Guarantee._ Tracked in: _issue._

### Known bugs

- _Intended behaviour, and how the code differs._ Proven by: _ignored test._

## Limits and non-goals

_Size limits, unsupported format features or fields, deliberate omissions._

## Platform and permissions

_Target platforms; TCC grants (Full Disk Access, Reminders, Calendars, Contacts) this crate needs._

## Application

_Applications only: binaries and commands, CLI flags and configuration, domains served (read source / write source), dependencies on other crates, links to `docs/openapi.json` and `docs/errors.md`._
```
