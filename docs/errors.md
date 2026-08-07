# API error codes

> Pre-1.0: codes and HTTP mappings may change without deprecation shims.

Failed requests return:

```json
{
  "error": {
    "code": "snake_case_id",
    "message": "human-readable summary",
    "details": { }
  }
}
```

`code` is always one of the `ErrorCode` values documented in OpenAPI (`components.schemas.ErrorCode`).
Internal SQL, filesystem paths, and framework diagnostics are never returned to clients.

| Code | Default message |
| --- | --- |
| `ambiguous_event_kit_match` | ambiguous EventKit match |
| `byte_range_not_satisfiable` | requested byte range is not satisfiable |
| `calendar_account_not_found` | calendar account not found |
| `calendar_database_unavailable` | Calendar database is unavailable |
| `calendar_not_found` | calendar not found |
| `calendar_read_only` | target calendar or list is read-only |
| `chat_not_found` | chat not found |
| `conflict` | conflict |
| `contact_not_found` | contact not found |
| `contact_photo_not_found` | contact photo not found |
| `contacts_access_denied` | Contacts access denied |
| `contacts_database_unavailable` | Contacts databases are unavailable |
| `contacts_unavailable` |  |
| `container_not_found` | container not found |
| `event_attachment_not_found` | event attachment not found |
| `event_attachment_unavailable` | event attachment is not available |
| `event_end_before_start` | end must be greater than or equal to start |
| `event_not_found` | event not found |
| `eventkit_access_denied` | EventKit access denied |
| `eventkit_unavailable` |  |
| `forbidden` | forbidden |
| `gateway_timeout` | upstream operation timed out |
| `group_not_found` | group not found |
| `internal_error` | internal server error |
| `invalid_cursor` | invalid cursor |
| `invalid_limit` | invalid limit |
| `invalid_parameter` | invalid parameter |
| `invalid_timestamp` | invalid timestamp |
| `message_attachment_not_found` | message attachment not found |
| `message_attachment_unavailable` | message attachment is not available |
| `message_not_found` | message not found |
| `messages_database_unavailable` | Messages database is unavailable |
| `method_not_allowed` | method not allowed |
| `note_attachment_not_found` | note attachment not found |
| `note_attachment_unavailable` | note attachment is not available |
| `note_folder_not_found` | note folder not found |
| `note_not_found` | note not found |
| `notes_database_unavailable` | Notes database is unavailable |
| `query_timeout` | database query timed out |
| `read_only_container` | target container is read-only |
| `reminder_attachment_not_found` | reminder attachment not found |
| `reminder_attachment_unavailable` | reminder attachment is not available |
| `reminder_list_not_found` | reminder list not found |
| `reminder_not_found` | reminder not found |
| `reminders_database_unavailable` | Reminders database is unavailable |
| `request_timeout` | request timed out |
| `resource_not_found` | resource not found |
| `route_not_found` | route not found |
| `service_unavailable` | service unavailable |
| `smart_list_read_only` | cannot write to smart reminder lists |
| `sqlite_sync_pending` | write succeeded but SQLite read path has not caught up yet |
| `unprocessable_entity` | unprocessable entity |
| `unsupported_alarm_kind` | unsupported alarm kind |
| `unsupported_query_parameter` | unsupported query parameter |
| `unsupported_reminder_field` | unsupported reminder field |
| `validation_error` | validation error |

Representative examples:

- **400** `invalid_limit` — limit outside 1..=200
- **404** `message_not_found` — unknown message GUID
- **422** `unsupported_alarm_kind` — Location/Unknown alarm kinds
- **503** `messages_database_unavailable` — Messages pool missing

