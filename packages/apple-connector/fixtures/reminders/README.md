# Reminders SQLite fixture

This directory contains a schema dump and empty copy of macOS Reminders.app's
Group Containers SQLite store for local development and tests.

| File | Purpose |
| --- | --- |
| `reminders.schema.sql` | Full schema: tables, indexes |
| `seed.sql` | Deterministic seed rows for tests |
| `reminders.db` | Generated locally from schema (gitignored) |
| `dump-schema.sh` | Refresh schema from the richest live Reminders store |
| `create-empty-db.sh` | Recreate `reminders.db` from `reminders.schema.sql` |

## Refresh from your Mac

Requires Full Disk Access and Reminders data at
`~/Library/Group Containers/group.com.apple.reminders/Container_v1/Stores/`.

```bash
./fixtures/reminders/dump-schema.sh
./fixtures/reminders/create-empty-db.sh
```

## Recreate the empty database

```bash
./fixtures/reminders/create-empty-db.sh
```

## Notes

- Multiple `Data-*.sqlite` files may exist; `dump-schema.sh` picks the store
  with the highest non-deleted reminder count.
- UUIDs are stored as 16-byte BLOBs; wire format uses dashed lowercase hex.
