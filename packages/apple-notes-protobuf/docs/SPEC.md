# `apple-notes-protobuf`: decoder for Apple Notes note bodies and tables

Decodes the gzip-compressed protobuf documents Apple Notes stores in `ZICNOTEDATA.ZDATA`, plus the
legacy binary-plist bodies of older notes, into text, formatting runs, checklist items, and
embedded-object references; and decodes a table attachment's mergeable data
(`ZMERGEABLEDATA1`) into its cells. Read-only: there is no encoder.

## Kind

Codec.

## Public API

- `decode_plain_text(&[u8]) -> Result<String, DecodeError>` and
  `decode_plain_text_with_limits(&[u8], &Limits)`.
- `decode_note_body(&[u8]) -> DecodedNoteBody` and `decode_note_body_with_limits(&[u8], &Limits)`.
  These never return an error: a failure is reported in `DecodedNoteBody.decode_error` as text,
  with every other field empty.
- `decode_table(&[u8]) -> Result<NoteTable, DecodeError>` and
  `decode_table_with_limits(&[u8], &Limits)`: `NoteTable { rows, right_to_left }`, cell text row by
  row with columns in visual order.
- `DecodedNoteBody { text, runs, checklist_items, embedded, decode_error }`, `NoteRun` (its
  `attachment_identifier` names the embedded object its U+FFFC stands for),
  `ParagraphStyle`, `ParagraphStyleKind`, `ChecklistItem`, `EmbeddedObject`.
- `DecodeError`: `Empty`, `InvalidGzip`, `InvalidProtobuf`, `InvalidPlist`,
  `LimitExceeded { limit, actual, max }`.
- `Limits` with `Default`.

## Guarantees

- A body starting with `bplist00` is read as a legacy plist; anything else must start with the gzip
  magic. Empty input is `DecodeError::Empty`. Enforced by: `src/lib.rs` tests
  `rejects_empty_payload`, `detects_invalid_gzip`.
- The note text is field 2 of the note string, reached through the document's version data
  (field 2 → field 3). Enforced by: `tests/acnp_fixtures.rs` text assertions over the ACNP corpus
  and `fixtures/notes/bodies/plain-text.bin`.
- Run offsets and lengths are UTF-16 code units, and slicing by runs never splits a surrogate pair
  ([NOTES-L-0001](lessons/NOTES-L-0001-utf16-run-lengths.md)). Run lengths that cover more than the
  text are `InvalidProtobuf`. Enforced by: `tests/limits.rs::runs_are_sliced_in_utf16_code_units`,
  `rejects_run_lengths_beyond_the_note_text`; `tests/acnp_fixtures.rs::acnp_emoji_formatting_*`.
- Paragraph style 0 is `Title` and 103 is `Checklist`. Enforced by:
  `src/lib.rs::decodes_plain_text_fixture_structured`, `decodes_checklist_fixture`.
- Consecutive checklist runs that share a todo UUID form one `ChecklistItem` with trimmed text;
  items whose text is empty are dropped. Enforced by: `src/lib.rs::decodes_checklist_fixture`.
- Every budget in `Limits` returns `LimitExceeded` naming it. Enforced by: `tests/limits.rs`
  (`rejects_gzip_bomb_under_default_limits`, `rejects_decompressed_payload_over_absolute_cap`,
  `rejects_excessive_field_count`, `rejects_excessive_message_nesting`,
  `rejects_deeply_nested_legacy_plist`, `rejects_excessive_run_count`), and every real fixture
  decodes under the defaults (`real_fixtures_decode_under_default_limits`).
- No input panics `decode_note_body`. Enforced by: the `notes_body_decode` fuzz target.
- Tables decode from their mergeable data: the root `com.apple.notes.ICTable` entry's `crRows` and
  `crColumns` ordered sets give each cell's position, `cellColumns` gives its text, and
  `crTableColumnDirection` marks right-to-left tables, whose columns are reversed into visual
  order. Enforced by: `tests/acnp_fixtures.rs` (`acnp_table_simple`, `acnp_table_formats`,
  `acnp_table_right_to_left`, expectations from apple_cloud_notes_parser's table spec); live,
  ignored: `apple-connector/tests/spec.rs::live_note_tables_decode` (3 of 3 tables on macOS 27).
- A table over `max_table_cells` is `LimitExceeded`. Enforced by:
  `tests/limits.rs::rejects_tables_with_too_many_cells`.
- No input panics `decode_table`. Enforced by: the `notes_table_decode` fuzz target (about 2
  million clean executions when added).

### Not yet enforced

- The rest of the paragraph style mapping — 1 Heading, 4 Monospace, 100 Bullet list, 101 Dash
  list, 102 Numbered list, any other value kept as `Unknown(n)` — is in
  `ParagraphStyleKind::from_raw` with no test asserting it.

### Known bugs

None known.

## Limits and non-goals

| `Limits` field | Default |
| --- | --- |
| `max_decompressed` | 64 MiB |
| `max_compression_ratio` | 200, applied only above 1 MiB decompressed |
| `max_fields` | 262 144 protobuf fields per document |
| `max_message_depth` | 16 |
| `max_plist_depth` | 64 |
| `max_runs` | 131 072 attribute runs |
| `max_table_cells` | 262 144 cells per table (rows × columns) |

- No decryption: locked notes are the caller's concern (`apple-connector` never passes their bodies
  in).
- Embedded objects in a body are references only (`attachment_identifier`, `type_uti`). Tables are
  decoded separately from their attachment's data with `decode_table`; drawings and scans are not
  decoded.
- Table cells are plain text: their formatting runs and any objects embedded in a cell are
  dropped.
- The legacy plist path returns plain text only: the first non-blank string under `NS.string`,
  `Text`, `text`, `content`, or `ZCONTENT`, else the first one found by a depth-limited walk.
- `decode_error` is human-readable text, not a stable code.

## Platform and permissions

Any platform; no permissions.
