# Apple Connector

Monorepo for reading Apple platform data on macOS. The first package,
[`packages/apple-connector`](packages/apple-connector/), loads and classifies
Messages.app rows from `~/Library/Messages/chat.db`.

## Requirements

- Apple Silicon Mac (`aarch64-darwin`)
- Messages.app signed in
- Nix with flakes enabled
- Full Disk Access for the terminal that runs the program

Grant access in **System Settings → Privacy & Security → Full Disk Access**, then
restart the terminal. If macOS still denies access when running the compiled
binary directly, add `target/debug/apple-connector` there as well.

## Run

```bash
nix develop
cargo run -p apple-connector
```

The program opens the Messages database read-only and prints all loaded messages.
If it reports that the database cannot be opened, Full Disk Access has not been
applied to the process.

## Fixtures

An empty Messages schema and database live in
[`packages/apple-connector/fixtures/messages/`](packages/apple-connector/fixtures/messages/).
Use them for local development without reading your real `~/Library/Messages/chat.db`.

```bash
./packages/apple-connector/fixtures/messages/create-empty-db.sh
cp packages/apple-connector/.env.example packages/apple-connector/.env
cargo install sqlx-cli --version 0.9.0 --no-default-features --features sqlite
cargo sqlx prepare -p apple-connector
rsync -a --delete .sqlx/ packages/apple-connector/sqlx/
```

Commit the updated `packages/apple-connector/sqlx/` directory. Nix builds use
`SQLX_OFFLINE=true` with `SQLX_OFFLINE_DIR=packages/apple-connector/sqlx` and do
not need a database connection.

## SQL queries

Queries in `packages/apple-connector/src/` are verified at compile time with SQLx
`query_as!`. After changing SQL, run the fixture steps above to refresh the offline
cache.

## License

MIT
