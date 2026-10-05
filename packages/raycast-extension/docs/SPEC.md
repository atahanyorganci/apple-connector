# `raycast-extension`: Raycast commands and AI tools over the apple-connector API

A Raycast extension (TypeScript, pnpm) that talks to a locally running `apple-connector` server:
searching Messages, Notes, Contacts, and Reminders, creating reminders, a due-today menu bar, and
eight Raycast AI tools. It has no access to Apple data of its own.

## Kind

Application.

## Public API

- **Commands** (`package.json`): Search Messages, Browse Chats, Search Notes, Search Contacts,
  Search Reminders, Create Reminder (all `view`), and Due Today (`menu-bar`, every 10 minutes,
  disabled by default) ([RAY-0002](decisions/RAY-0002-command-scope.md)).
- **AI tools** (`src/tools/`): search-messages, search-reminders, create-reminder, search-notes,
  get-note-contents, search-contacts, list-events, create-event.
- **Preference** `baseUrl`, default `http://127.0.0.1:3000`.
- **Scripts**: `generate`, `generate:check`, `dev`, `build`, `lint`, `format`, `typecheck`.

## Guarantees

- API types, operation routes, and enum value lists are generated from `docs/openapi.json`
  ([RAY-0001](decisions/RAY-0001-generated-client.md)). Enforced by: `pnpm generate:check`.
- Every failure is read as the server's `{ error: { code, message, details } }` envelope and
  branched on `code`; a body that is not an envelope becomes code `"unknown"`, and a server that
  cannot be reached is a distinct `ServerUnreachableError`. Enforced by: code (`src/lib/errors.ts`).
- Codes a user can act on (`eventkit_access_denied`, `contacts_access_denied`,
  `eventkit_unavailable`, `contacts_unavailable`, `*_database_unavailable`) get specific
  remediation text. Enforced by: code (`remediationFor`).
- Timestamps are converted between Unix seconds and `Date` only in `src/lib/time.ts`.
- Lists use keyset pagination: `page.next_cursor` is passed back as `cursor` while
  `page.has_more` is true. Enforced by: code (`src/lib/hooks.ts`).
- Smart lists are never offered as reminder targets, in the form or the AI tool. Enforced by: code
  (`list.kind !== "smart"`).
- Creating a reminder sends only fields EventKit can store: no `flagged`, `tags`, `section_id`,
  `parent_id`, or `attachments`, which the API rejects with `422 unsupported_reminder_field`
  ([REC-0012](../../../docs/decisions/REC-0012-reject-not-coerce.md)). Enforced by: the request
  bodies in `src/create-reminder.tsx` and `src/tools/create-reminder.ts`; the server contract by
  `packages/apple-connector/tests/spec.rs::flagged_false_is_still_an_unsupported_reminder_field`.
- Both mutating AI tools export `Tool.Confirmation`; tool results are capped at 25 items and long
  text is truncated (500 characters by default, 4000 for note contents). Enforced by: code
  (`src/lib/ai.ts`, `src/tools/*`).

### Not yet enforced

- Nothing here has run against a live server or inside Raycast, and the evals in `package.json`
  have not been executed (`ray evals` needs Raycast Pro) — PR #149.
- There are no automated tests; `generate:check`, `lint`, `format`, and `typecheck` are the checks,
  and none of them is part of `nix flake check`.

### Known bugs

- **`pnpm check` fails**: the root `package.json` defines `format`, so turbo resolves `"format"` in
  `//#check`'s `dependsOn` to `//#format`, which has no `turbo.json` entry
  (`missing_root_task_in_turbo_json`). Proven by: running `pnpm check`. Tracked in #170.
- **Contact photos are fetched by URL**, because the generated client types the photo operation's
  response as `void` (`apple-connector` known bug). Tracked in #169.

## Limits and non-goals

- No sending messages: replies hand off to Messages.app.
- New reminders cannot be flagged, tagged, put in a section, or nested; search still shows those
  fields.
- No Calendar commands (Raycast's Calendar feature covers it) and no contact or group writes.
- Locked notes render as locked, never as empty.
- AI tools require Raycast Pro.
- Distribution is undecided ([RAY-0003](decisions/RAY-0003-store-distribution.md)); run it with
  `pnpm dev`.

## Platform and permissions

macOS with Raycast. Needs a reachable `apple-connector` server; Apple permissions belong to the
server process, not to Raycast.

## Application

- Entry points: one module per command in `src/*.tsx` and per tool in `src/tools/*.ts`.
- Configuration: the `baseUrl` preference.
- `ai.instructions`: timestamps are Unix seconds, Messages is read-only, locked notes have no body,
  results are capped at 25, Reminders priority is 0 none, 1 high, 5 medium, 9 low.
- Depends on the HTTP contract in [`docs/openapi.json`](../../../docs/openapi.json) and the codes in
  [`docs/errors.md`](../../../docs/errors.md).
