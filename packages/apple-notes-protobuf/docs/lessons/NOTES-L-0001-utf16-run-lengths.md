---
id: NOTES-L-0001
status: graduated
observed-on: "Apple Notes gzip+protobuf note bodies (ACNP fixture corpus); AttributeRun.length"
graduated-to: "packages/apple-notes-protobuf/tests/limits.rs; packages/apple-connector/src/notes/markdown.rs"
provenance:
  - https://github.com/atahanyorganci/apple-connector/issues/84
  - https://github.com/atahanyorganci/apple-connector/pull/140
  - https://github.com/atahanyorganci/apple-connector/issues/174
---

# Notes attribute run lengths count UTF-16 code units, not Unicode scalar values.

## What happened

Validating run lengths against the note text (#84) failed on the three emoji fixtures: every run
after an emoji was misaligned, so checklist items and formatting ranges sliced the wrong text.

## Why

Apple encodes `AttributeRun.length` in UTF-16 code units, as `NSString` measures length. An emoji
outside the Basic Multilingual Plane is one `char` in Rust and two code units in UTF-16.

## Rule

Every run offset and length is measured in UTF-16 code units, and slicing the text by runs never
splits a surrogate pair.

## Graduated

- `packages/apple-notes-protobuf/tests/limits.rs`: `runs_are_sliced_in_utf16_code_units`.
- `packages/apple-notes-protobuf/tests/acnp_fixtures.rs`: `acnp_emoji_formatting_1`, `_2`, `_3`.
- `NoteRun.start` and `NoteRun.length` are documented as UTF-16 code units.
- Every consumer of runs must follow the same rule. The `/contents` Markdown renderer in
  `apple-connector` sliced by `char` until #174:
  `packages/apple-connector/src/notes/markdown.rs::runs_after_an_emoji_keep_their_text`.
