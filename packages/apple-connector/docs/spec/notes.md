# Notes: `NoteStore.sqlite` mapping

Read-only. Source: `~/Library/Group Containers/group.com.apple.notes/NoteStore.sqlite`
(`--notes-database`, `APPLE_CONNECTOR_NOTES_DATABASE`).

## Tables and entities

Notes, folders, attachments, accounts, and hashtags are all rows of `ZICCLOUDSYNCINGOBJECT`,
distinguished by `Z_ENT`. The entity numbers for `ICNote`, `ICFolder`, `ICAttachment`, `ICAccount`,
and `ICHashtag` are read from `Z_PRIMARYKEY` at startup; a missing one stops startup
([CONN-0020](../decisions/CONN-0020-schema-fail-fast.md)). Bodies are in `ZICNOTEDATA.ZDATA`.

## Identifiers

`NoteId`, `NoteFolderId`, `NoteAttachmentId`: `ZIDENTIFIER`, a text UUID.

## Rows

- Timestamps are Core Data seconds since 2001-01-01 UTC; `NULL` and `0` are unset.
- Order: `ZMODIFICATIONDATE1 DESC, Z_PK DESC`.
- A note is deleted when `ZMARKEDFORDELETION = 1` or its folder has `ZFOLDERTYPE = 1` (Recently
  Deleted). Deleted notes are hidden unless `include_deleted=true`; the Recently Deleted folder is
  not listed.
- `ZFOLDERTYPE = 2` is a smart folder (`kind: smart`).

## Bodies

- Decoded with `apple-notes-protobuf`: text, formatting runs, checklist items, and embedded-object
  references. A failure is `decode_error` on the body
  ([CONN-0001](../decisions/CONN-0001-best-effort-decoding.md)).
- Locked notes (`is_locked`) are never decoded; neither plaintext nor ciphertext is returned
  ([CONN-0003](../decisions/CONN-0003-privacy-boundary.md)).
- `GET /v1/notes/{id}/contents` renders Markdown with YAML front matter; locked notes and decode
  failures have an empty body.
- `q` matches title, snippet, and decoded body text, within the bounded scan.
- Tables are decoded from their attachment's `ZMERGEABLEDATA1` into `body.embedded[].table`
  (`rows`, `right_to_left`, or `decode_error`), and `/contents` renders each as a Markdown table
  where its placeholder stands, first row as the header. Drawings and scans are not decoded.

## Attachments

Files resolve under `Accounts/<account>/` in the Notes group container
(`--notes-attachment-root`), and must stay inside that account's directory.
