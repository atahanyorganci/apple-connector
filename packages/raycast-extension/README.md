# Apple Connector for Raycast

Search and edit Apple Messages, Reminders, Notes, Contacts and Calendar from
Raycast, backed by the [apple-connector](../../) HTTP API.

> **Requires a running server.** This extension talks to `apple-connector` over
> HTTP on your own machine. Start it with `cargo run -p apple-connector` before
> using any command.

## Setup

1. Start the server: `cargo run -p apple-connector`
2. Grant the server **Full Disk Access** (Messages, Reminders, Notes, Calendar
   and Contacts stores) and the **Reminders**, **Calendars** and **Contacts**
   permissions for writes.
3. Set **Server URL** in the extension preferences if you did not use the
   default `http://127.0.0.1:3000`.

Commands render an actionable empty state when the server is unreachable or a
permission is missing, so you should not have to guess which step was skipped.

## Commands

| Command             | Mode     | What it does                                                     |
| ------------------- | -------- | ---------------------------------------------------------------- |
| Search Messages     | view     | Full-text search over message text and decoded attributed bodies |
| Browse Chats        | view     | Conversation list, drilling into history                         |
| Search Notes        | view     | Search titles, snippets and decoded body; Markdown preview       |
| Search Contacts     | view     | Search with container and group scoping, photo and vCard         |
| Search Reminders    | view     | Search with open, completed, flagged and overdue scopes          |
| Create Reminder     | view     | Section, flag, URL, tags, priority, due date and recurrence      |
| Reminders Due Today | menu-bar | Overdue and due-today counts; complete in one click              |

The menu bar command ships disabled — enable it in the extension preferences.

## AI tools

The extension exposes eight tools to Raycast AI, spanning all five domains.
That makes cross-domain requests possible in one turn, for example _"what did
Sam say about the lease, and remind me to follow up Friday"_.

Both mutating tools (`create-reminder`, `create-event`) always confirm before
writing.

> AI Extensions require Raycast Pro. Tool calling needs a model from Bring Your
> Own Key or Custom Providers, both of which are Pro-exclusive.

## Limitations

- **Messages is read-only.** The API exposes no send endpoint, so reply actions
  hand off to Messages.app rather than sending.
- **Locked notes never expose body text.** The server returns neither decoded
  text nor ciphertext for them.
- **Calendar has no dedicated commands.** Raycast's built-in Calendar covers
  that ground; the API is used only by the AI tools.

## Development

```bash
pnpm dev              # run in Raycast with hot reload
pnpm generate         # regenerate src/lib/api.gen.ts from docs/openapi.json
pnpm generate:check   # fail if the generated client is stale
```

`src/lib/api.gen.ts` is generated from the committed OpenAPI contract and must
never be edited by hand. Regenerate it after changing the API, and commit the
result.
