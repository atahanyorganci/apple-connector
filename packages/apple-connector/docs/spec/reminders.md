# Reminders: Core Data store mapping

Reads from SQLite; writes through EventKit (`apple-eventkit`). Source: the chosen
`Data-*.sqlite` under `~/Library/Group Containers/group.com.apple.reminders/Container_v1/Stores/`
([CONN-0010](../decisions/CONN-0010-store-discovery.md)).

## Tables and entities

`ZREMCDREMINDER`, `ZREMCDBASELIST`, `ZREMCDBASESECTION`, `ZREMCDOBJECT` (polymorphic: alarms,
alarm triggers, recurrence rules, hashtags), `ZREMCDHASHTAGLABEL`, `ZREMCDSAVEDATTACHMENT`.
Entity numbers for `REMCDAlarm`, `REMCDAlarmDateTrigger`, `REMCDAlarmTimeIntervalTrigger`,
`REMCDAlarmLocationTrigger`, `REMCDRecurrenceRule`, `REMCDHashtag`, and `REMCDSmartList` are read
from `Z_PRIMARYKEY` at startup; a missing one stops startup
([CONN-0020](../decisions/CONN-0020-schema-fail-fast.md)).

## Identifiers

`ReminderId`, `ReminderListId`, `SectionId`, and `ReminderAttachmentId` are `ZIDENTIFIER` (a
16-byte blob) formatted as a lowercase dashed UUID. EventKit resolves a reminder or list by this
UUID in either case, and a reminder's `calendarItemExternalIdentifier` is the same UUID
([CONN-L-0003](../lessons/CONN-L-0003-calendar-identifier-spaces.md)).

## Rows

- Every query requires `ZMARKEDFORDELETION = 0`; deleted reminders are never returned and there is
  no `include_deleted`.
- Timestamps are Core Data seconds since 2001-01-01 UTC; `NULL` and `0` are unset.
- Global and per-list order: `ZLASTMODIFIEDDATE DESC, Z_PK DESC`. Subtasks: `ZICSDISPLAYORDER ASC,
  Z_PK ASC`.
- Section membership is stored on the list row as JSON in
  `ZMEMBERSHIPSOFREMINDERSINSECTIONSASDATA` (`memberships[].memberID` → `groupID`).
- A list is smart when `ZSMARTLISTTYPE` is set; its `ZFILTERDATA` is decoded best-effort
  (`decoded: false` on failure).
- Priority is `ZPRIORITY`, 0–9: 0 none, 1 high, 5 medium, 9 low.

## Writes

`POST /v1/reminder-lists/{list_id}/reminders`, `PATCH`/`DELETE /v1/reminders/{id}`. Smart lists are
`403 smart_list_read_only`. `section_id`, `parent_id`, `tags`, `attachments`, `flagged`, and
location coordinates are `422 unsupported_reminder_field`
([REC-0012](../../../../docs/decisions/REC-0012-reject-not-coerce.md)).

## Attachments

Files resolve under the store's sibling support directory (`--reminders-attachment-root`) and must
stay inside it.
