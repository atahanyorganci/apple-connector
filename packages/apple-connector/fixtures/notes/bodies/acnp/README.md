# ACNP note-body fixtures

Binary note-body blobs vendored from
[threeplanetssoftware/apple_cloud_notes_parser](https://github.com/threeplanetssoftware/apple_cloud_notes_parser)
(`spec/data/exported_blobs/`), MIT licensed.

Pinned upstream commit: see `.upstream-commit`.

These fixtures drive `apple-notes-protobuf` tests in
`packages/apple-notes-protobuf/tests/acnp_fixtures.rs`. Expected assertions live
in `manifest.toml`.

## Included blobs

| Blob | Role |
| --- | --- |
| `simple_note_protobuf_gzipped.bin` | Minimal gzip-wrapped note (`Title`) |
| `simple_note_protobuf.bin` | Uncompressed protobuf reference for the same note |
| `color_formatting_gzipped.bin` | Colored / bold checklist text |
| `wide_characters_gzipped.bin` | CJK wide characters |
| `html_gzipped.bin` | Literal `<HTML>` in note text |
| `emoji_formatting_{1,2,3}_gzipped.bin` | Emoji + formatting / links |
| `url_gzipped.bin` | Link attribute runs |
| `text_decorations_gzipped.bin` | Bold / italic / underline / strikethrough |
| `block_quotes_gzipped.bin` | Block quotes and monospace |
| `list_indents_gzipped.bin` | Nested lists (upstream Ruby test is pending) |
| `table_gzipped.bin`, `table_formats_gzipped.bin`, `right_to_left_table_gzipped.bin` | Embedded tables (not decoded yet; tests ignored) |
| `ZSERVERRECORDDATA.bin`, `ZSERVERSHAREDATA.bin` | CloudKit server/share blobs (not note-body decode; not tested) |

## Refresh from upstream

```bash
COMMIT=$(cat packages/apple-connector/fixtures/notes/bodies/acnp/.upstream-commit)
# or: COMMIT=$(curl -sL https://api.github.com/repos/threeplanetssoftware/apple_cloud_notes_parser/commits/master | jq -r .sha)
BASE="https://raw.githubusercontent.com/threeplanetssoftware/apple_cloud_notes_parser/${COMMIT}/spec/data/exported_blobs"
cd packages/apple-connector/fixtures/notes/bodies/acnp
for f in *.bin; do
  curl -fsSL -o "$f" "$BASE/$f"
done
echo "$COMMIT" > .upstream-commit
```
