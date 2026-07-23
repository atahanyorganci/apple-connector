# Notes SQLite fixture

This directory contains a schema dump and empty copy of macOS Notes.app's
Group Containers SQLite store for local development and tests.

| File | Purpose |
| --- | --- |
| `notes.schema.sql` | Full schema: tables, indexes |
| `seed.sql` | Deterministic seed rows for tests |
| `bodies/` | Canonical gzip-compressed note body blobs referenced by `seed.sql` |
| `notes.db` | Generated locally from schema (gitignored) |
| `dump-schema.sh` | Refresh schema from `~/Library/Group Containers/group.com.apple.notes/NoteStore.sqlite` |
| `create-empty-db.sh` | Recreate `notes.db` from `notes.schema.sql` |

## Refresh from your Mac

Requires Full Disk Access and Notes data at
`~/Library/Group Containers/group.com.apple.notes/NoteStore.sqlite`.

```bash
./fixtures/notes/dump-schema.sh
./fixtures/notes/create-empty-db.sh
```

## Recreate the empty database

```bash
./fixtures/notes/create-empty-db.sh
```

## Load seed data

```bash
./fixtures/notes/create-empty-db.sh
sqlite3 packages/apple-connector/fixtures/notes/notes.db < packages/apple-connector/fixtures/notes/seed.sql
```

## Notes

- Note and folder UUIDs are plain text in `ZIDENTIFIER` (dashed uppercase in the live store; fixture seed uses lowercase).
- Timestamps use the Core Data epoch (seconds since 2001-01-01; Unix offset `978307200`).
- `seed.sql` embeds body blobs as hex sourced from `bodies/plain-text.bin` and `bodies/checklist.bin`.
