# Apple Connector

Monorepo for reading and writing Apple platform data on macOS. The
[`packages/apple-connector`](packages/apple-connector/) crate exposes a hybrid
HTTP API over Messages, Reminders, Notes, Calendar, and Contacts. Reads use live
SQLite databases; Reminders, Calendar, and Contacts writes go through EventKit
and the Contacts framework.

See [`AGENTS.md`](AGENTS.md) for crate layout and contributor conventions.

## Requirements

- Apple Silicon Mac (`aarch64-darwin`)
- Nix with flakes enabled
- **Full Disk Access** for the terminal (Messages, Reminders, Notes, Calendar Group Containers, Contacts AddressBook)
- **Reminders**, **Calendars**, and **Contacts** permission for write routes (EventKit / Contacts framework)

Grant access in **System Settings → Privacy & Security → Full Disk Access**, then
restart the terminal. If macOS still denies access when running the compiled
binary directly, add `target/debug/apple-connector` there as well.

## Quick start

```bash
nix develop
cargo run -p apple-connector
```

By default the server binds to `127.0.0.1:3000`, opens
`~/Library/Messages/chat.db`, auto-discovers the Reminders store, opens the Notes
Group Container `NoteStore.sqlite` read-only, and serves the API documented at
`/openapi.json`. Browse and try endpoints interactively at `/docs`.

```bash
curl -s http://127.0.0.1:3000/healthz
curl -s 'http://127.0.0.1:3000/v1/messages?limit=5'
curl -s http://127.0.0.1:3000/openapi.json | jq .info.title
```

### Checks

```bash
nix fmt
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
nix flake check --no-write-lock-file
```

`nix flake check` runs cargo-deny (bans, licenses, sources), clippy, tests, the
fuzz smoke pass, the runtime-SQL and API-error-leakage scripts, and treefmt on
`aarch64-darwin`. Advisories need a network fetch, so run `cargo deny check`
from `nix develop` for those.

## License

MIT
