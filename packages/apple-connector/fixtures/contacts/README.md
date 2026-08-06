# Contacts AddressBook fixture

This directory contains a schema dump and empty copy of macOS Contacts.app's
AddressBook SQLite store for local development and tests.

| File | Purpose |
| --- | --- |
| `contacts.schema.sql` | Full schema: tables, indexes |
| `seed.sql` | Deterministic seed rows for tests |
| `contacts.abcddb` | Generated locally from schema + seed (gitignored) |
| `dump-schema.sh` | Refresh schema from the richest live AddressBook source |
| `create-empty-db.sh` | Recreate `contacts.abcddb` from schema and seed |

## Refresh from your Mac

Requires Full Disk Access and Contacts data at
`~/Library/Application Support/AddressBook/Sources/*/AddressBook-v*.abcddb`.

```bash
./fixtures/contacts/dump-schema.sh
./fixtures/contacts/create-empty-db.sh
```

## Recreate the empty database

```bash
./fixtures/contacts/create-empty-db.sh
```

## Notes

- `dump-schema.sh` picks the AddressBook source with the highest contact count.
- UUIDs are stored as 16-byte BLOBs; wire format uses dashed lowercase hex.
