# Calendar SQLite fixture

This directory contains a schema dump and empty copy of macOS Calendar.app's
Group Containers SQLite store for local development and tests.

| File | Purpose |
| --- | --- |
| `calendar.schema.sql` | Full schema: tables, indexes |
| `seed.sql` | Deterministic seed rows for tests (applied by Rust `CalendarFixtureDb`) |
| `calendar.db` | Generated locally from schema (gitignored) |
| `dump-schema.sh` | Refresh schema from live Calendar database |
| `create-empty-db.sh` | Recreate `calendar.db` from `calendar.schema.sql` |

## Refresh from your Mac

Requires Full Disk Access and Calendar data at
`~/Library/Group Containers/group.com.apple.calendar/Calendar.sqlitedb`.

```bash
./fixtures/calendar/dump-schema.sh
./fixtures/calendar/create-empty-db.sh
```

## Recreate the empty database

```bash
./fixtures/calendar/create-empty-db.sh
```

## Notes

- Seed rows are applied in Rust via `CalendarFixtureDb::seeded()`; the shell
  script creates a schema-only database for SQLx offline preparation.
- `sqlite_sequence` and `sqlite_stat1` are omitted from the dump because they
  are SQLite-managed internal tables.
