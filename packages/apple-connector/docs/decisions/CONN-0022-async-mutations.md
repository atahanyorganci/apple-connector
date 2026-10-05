---
id: CONN-0022
status: proposed
issues:
  - https://github.com/atahanyorganci/apple-connector/issues/105
provenance:
  - https://github.com/atahanyorganci/apple-connector/pull/139
  - https://github.com/atahanyorganci/apple-connector/pull/154
supersedes: [CONN-0016]
superseded-by: []
---

# A write reads SQLite once and answers 201/200 with the detail, or 202 with `sync_pending: true`.

## Question

[CONN-0016](CONN-0016-hydrate-polling.md) slept inside every write and still could not promise
the detail.

## Options

- **Longer polling.**
- **One read, honest status**: report what is known now and let the client poll the read API.

## Decision

One non-blocking read after the framework write. The response is a `SyncPending*DetailDto`
envelope: `{ id, sync_pending, detail? }`.

| Read found the row | Create | Update |
| --- | --- | --- |
| yes | `201`, `sync_pending: false`, `detail` set | `200`, `sync_pending: false`, `detail` set |
| no | `202`, `sync_pending: true`, no `detail` | `202`, `sync_pending: true`, no `detail` |

`id` is always a read-API id that `GET` resolves once the store catches up.

## Consequences

- Clients poll `GET` by `id` after a 202. The Raycast tools fall back to the requested values and
  report `syncPending`.
- Event ids are a read-API id only since PR #154 ([CONN-0023](CONN-0023-event-identifiers.md)).

## Evidence

- `packages/apple-connector/src/api/hydrate.rs`: `mutation_status`, test
  `mutation_status_returns_accepted_when_sync_pending`.
- `packages/apple-connector/tests/eventkit_integration.rs` (ignored, live):
  `http_created_event_id_resolves_through_get`.
- `packages/apple-eventkit/tests/identifier_probe.rs` (ignored, live, read-only): a reminder's
  `calendarItemExternalIdentifier`, lowercased, is its `ReminderId` (59/59 on macOS 27), which is
  what `hydrate_reminder` relies on.
