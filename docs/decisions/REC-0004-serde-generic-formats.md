---
id: REC-0004
status: superseded
issues:
  - https://github.com/atahanyorganci/apple-connector/issues/41
  - https://github.com/atahanyorganci/apple-connector/issues/43
  - https://github.com/atahanyorganci/apple-connector/issues/58
  - https://github.com/atahanyorganci/apple-connector/issues/59
provenance:
  - https://github.com/atahanyorganci/apple-connector/pull/48
  - https://github.com/atahanyorganci/apple-connector/pull/65
supersedes: []
superseded-by: [REC-0013]
---

# The interchange crates are Serde formats generic over `T`, like `serde_json`.

## Question

Calendar and Contacts needed iCalendar, CalDAV, vCard, and CardDAV output. Each format got its
own crate; the question was the shape of their public API.

## Options

- **Serde data formats**: `to_string<T: Serialize>`, `from_str<T: Deserialize>`, custom
  `Serializer`/`Deserializer`, following `serde_json`.
- **Concrete codecs**: functions that read and write one model type.

## Decision

Serde data formats (#41: "a Serde format crate for RFC 5545 iCalendar text, following the
`serde_json` pattern"; #43, #58, #59 the same).

## Consequences

In practice each crate parsed one model and bridged to the caller's `T` through
`serde_json::Value`, in both directions. CardDAV routed on `std::any::type_name::<T>()`.
Callers serialized each list item separately, so paged CardDAV and CalDAV responses carried N XML
declarations and N roots (PR #140).

Superseded by [REC-0013](REC-0013-typed-format-apis.md) (#94, PR #140).

## Evidence

- #94: "Remove generic APIs that parse one model, bridge through `serde_json::Value`, and pretend
  to deserialize arbitrary T."
- PR #140, "#94 typed APIs" and "#92 CardDAV dispatch".
