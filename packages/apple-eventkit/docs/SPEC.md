# `apple-eventkit`: EventKit writes for Reminders and Calendar

Creates, updates, and deletes reminders and calendar events through `EKEventStore`, and reports
Reminders and Calendars authorization. `apple-connector` uses it for every Reminders and Calendar
write; reads never go through it ([REC-0005](../../../docs/decisions/REC-0005-hybrid-read-write.md)).
All Objective-C and `unsafe` code for EventKit is confined here.

## Kind

Framework.

## Public API

- `EventKitStore`, a cloneable handle (`new()`); clones share one worker and one authorization
  snapshot.
  - Authorization: `auth_status()`, `refresh_auth_status()`, `request_access()` →
    `AccessRequestOutcome { reminders, events }`, `ensure_reminders_access()`,
    `ensure_events_access()`.
  - Reminders: `create_reminder(ReminderListResolveHint, CreateReminderInput)`,
    `update_reminder(api_id, external_id, UpdateReminderInput)`,
    `delete_reminder(api_id, external_id)` → `SavedReminder { external_id, calendar_item_id }`.
  - Events: `create_event(CalendarResolveHint, CreateEventInput)`,
    `update_event(api_id, external_id, occurrence_start, UpdateEventInput)`,
    `delete_event(api_id, external_id, DeleteEventInput)` → `SavedEvent`.
- Inputs: `DueInput`, `ReminderLocationInput`, `LocationInput`, `AlarmInput`/`AlarmKind`
  (`Absolute`, `Relative`), `RecurrenceInput`/`RecurrenceFrequency`, `EventSpan` (`This`,
  `Future`).
- `AuthStatus` (`NotDetermined`, `Restricted`, `Denied`, `Authorized`, `WriteOnly`, `Unavailable`),
  `EntityAuthStatus`, `AuthOutcome` (`AlreadyGranted`, `Granted`, `Denied`, `Restricted`,
  `TimedOut`, `Unavailable`, `Failed`).
- `EventKitError`: `NotFound`, `AccessDenied`, `ReadOnlyCalendar`, `ValidationFailed(String)` (this
  crate's own message), `Rejected { code, description }` (EventKit's refusal), `EndBeforeStart`, `AmbiguousMatch(String)`, `UnsupportedPlatform`, `Framework(String)`,
  `Timeout`.

## Guarantees

- The `EKEventStore` is created on, owned by, and only used from one dedicated thread; calls are
  strictly serial, and a timeout abandons the result, never a borrow of the store
  ([REC-0014](../../../docs/decisions/REC-0014-framework-worker.md)). Enforced by:
  `src/worker.rs` tests (`jobs_never_overlap` runs a non-`Send` `RefCell` resource).
- Budgets: 30 s per operation; 400 s for a job that may show a permission prompt, with the prompt
  itself bounded at 120 s. Exceeding one is `Timeout`. Enforced by: `src/store.rs`, `src/auth.rs`
  constants.
- Write inputs can only express what EventKit stores: events have no `status`
  ([EK-0003](decisions/EK-0003-event-status.md)), reminder locations have no coordinates
  ([EK-0005](decisions/EK-0005-reminder-location.md)), and spans are `This` or `Future`
  ([EK-0004](decisions/EK-0004-event-span.md)). Enforced by: the input types.
- An update's start and end are merged with the stored event before the range is checked, so a
  start-only or end-only update cannot invert it; `end < start` is `EndBeforeStart`. Enforced by:
  `src/event.rs` tests (`moving_start_past_the_stored_end_is_rejected`,
  `moving_end_before_the_stored_start_is_rejected`, …); live:
  `partial_update_cannot_invert_the_stored_range` in `apple-connector/tests/eventkit_integration.rs`.
- Calendar fields are computed in the offset EventKit applies, and an all-day value is the local
  day containing the instant ([EK-0006](decisions/EK-0006-date-components-timezone.md)). Enforced
  by: `src/datetime.rs` tests.
- Reminder priority must be 0–9. Enforced by: `validate_priority` in `src/reminder.rs`.
- `EKErrorDomain` errors are classified: read-only and immutable-target codes →
  `ReadOnlyCalendar`; `EventStoreNotAuthorized` → `AccessDenied`; `NoCalendar` /
  `CalendarHasNoSource` → `NotFound`; `DatesInverted` → `EndBeforeStart`; other invalid-input codes →
  `Rejected { code, description }`, so Apple's text is never a `ValidationFailed` message (#158);
  everything else → `Framework`. Enforced by: `src/error.rs` tests.
- An access request reports what happened to each entity, keeping granted, denied, restricted,
  timed out, unavailable, and failed apart. Enforced by: `src/auth.rs` tests
  (`a_prompt_result_keeps_denial_and_timeout_apart`, …).
- Item lookup: `calendarItemWithIdentifier:` with the API id, then
  `calendarItemsWithExternalIdentifier:` with the API id and the supplied external id; for events
  also `eventWithIdentifier:`. No match is `NotFound`; several are `AmbiguousMatch`, and for events
  `occurrence_start` selects one occurrence. Enforced by: code (`src/item_lookup.rs`); live:
  `apple-connector/tests/eventkit_integration.rs`.
- The identifier semantics this lookup relies on hold on the running macOS: event and calendar
  lookups are case-sensitive, reminder lookups are not
  ([CONN-L-0003](../../apple-connector/docs/lessons/CONN-L-0003-calendar-identifier-spaces.md)).
  Enforced by: `tests/identifier_probe.rs` (ignored, live, read-only).
- The crate does not build off macOS. Enforced by: `compile_error!` in `src/lib.rs`
  ([REC-0015](../../../docs/decisions/REC-0015-macos-only.md)).

### Not yet enforced

- No live test confirms the all-day behaviour with the system set to a non-UTC zone (PR #141
  manual check: Asia/Tokyo and America/Los_Angeles).
- No test covers a denied permission end to end; the outcome mapping is unit-tested only.

### Known bugs

- Calendar resolution never matches the identifiers it is given. `resolve_event_calendar` tries
  `calendarWithIdentifier:` with the hint's `external_id` (the server's id) and then its `api_id`
  (lowercased), which the probe shows never resolve on macOS 27, so it falls back to a
  case-insensitive title match: two calendars with the same title are `AmbiguousMatch`. Proven by:
  `tests/identifier_probe.rs` (0/18 calendars by lowercased UUID, 0/14 by `external_id`) together
  with `src/calendar_resolve.rs`. Tracked in #156.

## Limits and non-goals

- Reminders cannot be given sections, subtasks, tags, attachments, flags, or coordinates; events
  cannot be given a status, attendees, or invitations.
- Alarms are absolute or relative only; location (proximity) alarms are not supported.
- Recurrence is one rule: frequency, interval, and an optional count or end date.
- `PATCH` alarms replace all existing alarms.
- Birthday and subscription calendars, smart lists, and calendars that disallow modification are
  `ReadOnlyCalendar` before any write.
- No reads: EventKit is never queried for data the API returns.

## Platform and permissions

macOS only. Calendars and Reminders TCC grants for the process (prompted at startup when not yet
determined). `WriteOnly` access counts as granted (`ensure_*_authorized`, `outcome_for_status`).
