# Apple Connector

Proof of concept for reading the five most recent Messages.app messages directly
from `~/Library/Messages/chat.db`.

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
cargo run
```

The program opens the Messages database read-only and prints the newest five
message rows. If it reports that the database cannot be opened, Full Disk Access
has not been applied to the process.

## Fixtures

An empty Messages schema and database live in [`fixtures/messages/`](fixtures/messages/).
Use them for local development without reading your real `~/Library/Messages/chat.db`.

```bash
./fixtures/messages/create-empty-db.sh
```

## License

MIT
