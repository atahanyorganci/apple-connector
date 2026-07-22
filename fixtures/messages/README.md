# Messages `chat.db` fixture

This directory contains a schema dump and empty copy of macOS Messages.app's
`chat.db` SQLite database for local development and tests.

| File                 | Purpose                                                 |
| -------------------- | ------------------------------------------------------- |
| `chat.schema.sql`    | Full schema: tables, indexes, triggers                  |
| `chat.db`            | Generated locally from `chat.schema.sql` (gitignored)   |
| `dump-schema.sh`     | Refresh `chat.schema.sql` from a live Messages database |
| `create-empty-db.sh` | Recreate `chat.db` from `chat.schema.sql`               |

## Refresh from your Mac

Requires Full Disk Access and Messages.app data at
`~/Library/Messages/chat.db`.

```bash
./fixtures/messages/dump-schema.sh
./fixtures/messages/create-empty-db.sh
```

## Recreate the empty database

```bash
./fixtures/messages/create-empty-db.sh
```

## Notes

- The schema includes Messages-specific triggers that call custom SQLite
  functions provided by macOS (`verify_chat`, `delete_attachment_path`, etc.).
  Creating the schema works in plain SQLite; those functions only matter when
  triggers fire at runtime.
- `sqlite_sequence` and `sqlite_stat1` are omitted from the dump because they
  are SQLite-managed internal tables.
