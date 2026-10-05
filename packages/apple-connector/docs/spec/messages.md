# Messages: `chat.db` mapping

Read-only. Source: `~/Library/Messages/chat.db` (`--messages-database`,
`APPLE_CONNECTOR_MESSAGES_DATABASE`; `APPLE_CONNECTOR_DATABASE` is accepted with a deprecation
warning). Required: startup fails without it.

## Tables

`message`, `chat`, `handle`, `chat_message_join`, `chat_handle_join`, `attachment`,
`message_attachment_join`.

## Identifiers

| API | Source |
| --- | --- |
| `ChatId` | `chat.ROWID` (integer) |
| `MessageId` | `message.guid` |
| `AttachmentId` | `attachment.guid` |
| `HandleId` | `handle.id` (a phone number or email address) |

## Timestamps

`message.date` and related columns are nanoseconds since 2001-01-01 UTC. `0` means unset and is
`null` (`messages/row.rs` `parse_apple_timestamp`); negative values are valid instants.

## Ordering

- Global messages: `message.date DESC, message.ROWID DESC`.
- Messages in a chat: `chat_message_join.message_date DESC, message_id DESC`.
- Chats: latest activity, computed with a window function over `chat_message_join`.

## Content

- `attributedBody` is a typedstream (`apple-typedstream`); its text and `__kIM*` attribute runs
  become the structured body. A blob that fails to decode is reported with a typed
  `AttributedBodyErrorDto` on that message ([CONN-0001](../decisions/CONN-0001-best-effort-decoding.md)).
- `GET /v1/messages` filters: `q`, `chat_id`, `sender`, `before`, `after`, `direction`,
  `transport`, `content_type` (`text`, `audio`, `attachment`, `reaction`, `group_event`,
  `app_balloon`, `share_play`, `share_my_location`, `system`, …), `has_attachments`. `q` matches
  plain text and decoded `attributedBody` text.
- `payload_data` app balloons are decoded into URL, Photos, Poll, and Digital Touch variants, with
  an unknown fallback; raw payloads are never returned.

## Attachments

`attachment.filename` is resolved against the attachment root (default: `Attachments` next to
`chat.db`; `--attachment-root`), canonicalized, and must stay inside it.
