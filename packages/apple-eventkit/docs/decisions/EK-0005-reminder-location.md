---
id: EK-0005
status: proposed
issues:
  - https://github.com/atahanyorganci/apple-connector/issues/53
  - https://github.com/atahanyorganci/apple-connector/issues/77
provenance:
  - https://github.com/atahanyorganci/apple-connector/pull/141
supersedes: []
superseded-by: []
---

# Reminders take a location string only; coordinates are rejected until proximity alarms exist.

## Question

The reminder API accepted latitude and longitude but stored only the location text (#77).

## Options

- **Store coordinates through a proximity `EKAlarm` with a structured location**: the real
  geofence, but a different feature with its own inputs (radius, arrive/leave).
- **Reject coordinates on reminders.**

## Decision

Reject. `structuredLocation` exists on `EKEvent` and `EKAlarm`, not on `EKCalendarItem` or
`EKReminder`; EventKit even defines `EKErrorReminderLocationsNotSupported`.
`ReminderLocationInput` has only `title`, and the HTTP layer rejects
`location.latitude`/`location.longitude` on reminders with `422 unsupported_reminder_field`.
Events keep structured locations with coordinates.

## Consequences

Geofenced reminders are a follow-up: a proximity alarm input, not a location field.

## Evidence

- `packages/apple-eventkit/src/reminder.rs` `ReminderLocationInput` doc comment.
- `packages/apple-connector/src/api/eventkit_convert.rs` `reject_reminder_coordinates`.
