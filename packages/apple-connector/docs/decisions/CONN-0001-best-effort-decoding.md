---
id: CONN-0001
status: proposed
issues:
  - https://github.com/atahanyorganci/apple-connector/issues/6
  - https://github.com/atahanyorganci/apple-connector/issues/23
  - https://github.com/atahanyorganci/apple-connector/issues/26
  - https://github.com/atahanyorganci/apple-connector/issues/35
provenance:
  - https://github.com/atahanyorganci/apple-connector/pull/7
  - https://github.com/atahanyorganci/apple-connector/pull/29
  - https://github.com/atahanyorganci/apple-connector/pull/38
supersedes: []
superseded-by: []
---

# A blob that fails to decode is reported on its own record and never fails the request.

## Question

Every domain stores part of its content as an opaque blob: `attributedBody` typedstreams in
Messages, NSKeyedArchiver plists for reminder alarms, recurrence, and smart-list filters, and
gzip + protobuf note bodies. Apple changes these formats without notice, and one corrupt row can
sit in a page of good ones.

## Options

- **Fail the request**: one bad row makes a page of fifty unreadable.
- **Drop the field silently**: the client cannot tell "empty" from "could not read".
- **Report per record**: return what decoded, and say on that record what did not.

## Decision

Report per record. The page succeeds; the affected record carries a decode marker:

| Domain | Blob | Marker on the DTO |
| --- | --- | --- |
| Messages | `attributedBody` | typed `AttributedBodyErrorDto` (`invalid_typed_stream`, `not_attributed_string`, `missing_text`, `payload_too_large`) |
| Reminders | alarm and recurrence `ZDATECOMPONENTSDATA` | `decode_error: string` on the alarm / recurrence |
| Reminders | smart-list `ZFILTERDATA` | `decoded: false` on the filter |
| Notes | `ZICNOTEDATA.ZDATA` | `decode_error: string` on the body |

## Consequences

- Clients must check the marker before treating missing content as empty.
- The Reminders and Notes markers carry the decoder's message text. That text comes from this
  workspace's decoders, not from SQLite or a framework, but it is not a stable code.
- `MessageInventory` (a library function over a loaded history, not an HTTP endpoint) counts
  `attributedBody` decode failures, which is the regression check after a macOS update. The
  Reminders and Notes inventories do not count decode failures.

## Evidence

- `packages/apple-connector/src/api/dto/convert.rs` `attributed_body_error_to_dto`;
  `dto/reminder.rs` (`decoded`, `decode_error`); `dto/note.rs` (`decode_error`).
- `packages/apple-notes-protobuf/src/lib.rs` `decode_note_body_with_limits`: "one corrupt note
  never fails a page of otherwise valid ones".
- `packages/apple-connector/src/messages/inventory.rs` test
  `counts_unknown_empty_replies_sentinel_and_decode_errors`.
