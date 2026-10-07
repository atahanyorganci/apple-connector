---
id: RAY-0002
status: proposed
issues:
  - https://github.com/atahanyorganci/apple-connector/issues/142
  - https://github.com/atahanyorganci/apple-connector/issues/145
  - https://github.com/atahanyorganci/apple-connector/issues/148
provenance:
  - https://github.com/atahanyorganci/apple-connector/pull/149
supersedes: []
superseded-by: []
---

# The extension competes where Raycast has nothing — Messages — and leaves Calendar to Raycast's core feature; every mutating AI tool asks for confirmation.

## Question

Raycast already ships a first-party Reminders extension and a Calendar core feature. Which
commands are worth building on this API?

## Options

- **Mirror every API domain as commands.**
- **Build only what Raycast lacks**, and expose the rest to Raycast AI as tools.

## Decision

Build what Raycast lacks:

- **Commands**: Search Messages, Browse Chats, Search Notes, Search Contacts, Search Reminders,
  Create Reminder, and Due Today (menu bar, every 10 minutes, disabled by default). No Calendar
  commands; no Contacts writes.
- **AI tools**: search-messages, search-reminders, create-reminder, search-notes,
  get-note-contents, search-contacts, list-events, create-event. The two mutating tools export
  `Tool.Confirmation`. Results are capped at 25 items and long text is truncated.
- **`ai.instructions`** states what schemas cannot: timestamps are Unix seconds, Messages is
  read-only, locked notes have no body, results are capped at 25, and Reminders priority is
  0/1/5/9.

## Consequences

- The Reminders value proposition in #145 — sections, flags, and tags that Raycast's extension
  cannot set — conflicts with the API, which rejects all three
  ([REC-0012](../../../../docs/decisions/REC-0012-reject-not-coerce.md)). The form sent them
  anyway, so every submission failed (#155); Create Reminder and the create-reminder tool now
  offer only fields EventKit can store, and `ai.instructions` tells the model so. What remains
  over Raycast's own extension is the URL field and the search scopes.
- AI tools need Raycast Pro.

## Evidence

- `packages/raycast-extension/package.json` `commands`, `tools`, `ai.instructions`.
- `packages/raycast-extension/src/tools/create-event.ts`, `create-reminder.ts`: `confirmation`.
- PR #149: nothing was run against a live server or inside Raycast, and the evals were not run.
