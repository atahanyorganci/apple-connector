# Calendar: `Calendar.sqlitedb` mapping

Reads from SQLite; writes through EventKit (`apple-eventkit`). Source:
`~/Library/Group Containers/group.com.apple.calendar/Calendar.sqlitedb` (`--calendar-database`,
`APPLE_CONNECTOR_CALENDAR_DATABASE`). Only the modern `CalendarItem` schema is supported; a
database without it stops startup.

## Tables

`CalendarItem`, `Calendar`, `Store`, `OccurrenceCache`, `Location`, `Participant`, `Recurrence`,
`Alarm`, `ExceptionDate`, `Attachment`, `AttachmentFile`.

## Identifiers

| API | Source | EventKit |
| --- | --- | --- |
| `EventId` | `lower(CalendarItem.UUID)` | `calendarItemIdentifier` is `UUID` as stored (uppercase); case-sensitive |
| — | `CalendarItem.unique_identifier` | `calendarItemExternalIdentifier` (iCalendar UID) |
| — | `CalendarItem.external_id` | none: the server's resource id |
| `CalendarId` | `lower(Calendar.UUID)` | `calendarIdentifier` is `UUID` as stored; case-sensitive |
| — | `Calendar.external_id` | none |

([CONN-0023](../decisions/CONN-0023-event-identifiers.md),
[CONN-L-0003](../lessons/CONN-L-0003-calendar-identifier-spaces.md))

## Rows

- Timestamps are Core Data seconds since 2001-01-01 UTC; `NULL` and `0` are unset.
- Birthday events generated from Contacts carry a `last_modified` of 1976-04-01, and a start in
  1604 when the birthday has no year. Both are Apple's values and are returned as stored
  ([CONN-L-0006](../lessons/CONN-L-0006-apple-date-sentinels.md)).
- Without `start`/`end`: `CalendarItem` rows ordered `last_modified DESC, ROWID DESC`.
- With `start` and/or `end`: occurrences from `OccurrenceCache`, start
  `COALESCE(occurrence_start_date, occurrence_date)`, grouped to one row per occurrence, ordered
  `start DESC, ROWID DESC` ([CONN-0013](../decisions/CONN-0013-occurrence-cache.md),
  [CONN-L-0001](../lessons/CONN-L-0001-occurrence-start-null.md)).
- Hidden events are excluded unless `include_hidden=true`; cancelled events (`status = 2`) unless
  `include_cancelled=true`.
- `q` is a `LIKE` match on `summary`.

## Formats

JSON on the base routes; iCalendar on `…/iCal` and CalDAV XML on `…/caldav`
([CONN-0015](../decisions/CONN-0015-format-routes.md)), through `serde-icalendar` and
`serde-caldav`.

## Writes

`POST /v1/calendars/{calendar_id}/events`, `PATCH`/`DELETE /v1/events/{id}` with `span`
(`this` default, `future`) and `occurrence_start` for recurring events. `status` is
`422 immutable_event_field`; `end < start`, including after merging a partial update, is
`422 event_end_before_start`. Birthday and subscription calendars are `403 calendar_read_only`.

## Attachments

`GET /v1/events/{id}/attachments/{attachment_id}` serves `AttachmentFile` bytes from under the
Calendar attachment root (`--calendar-attachment-root`).
