---
id: EK-0006
status: proposed
issues:
  - https://github.com/atahanyorganci/apple-connector/issues/79
provenance:
  - https://github.com/atahanyorganci/apple-connector/pull/141
supersedes: []
superseded-by: []
---

# Calendar fields are computed in the timezone EventKit will read them in, and an all-day value is the local day containing the instant.

## Question

Reminder due dates are `NSDateComponents` without a timezone; EventKit reads them in the default
timezone ([EK-L-0001](../lessons/EK-L-0001-date-components-default-tz.md)). The crate split
instants into components in UTC, so a due of 2024-01-15 00:00 in Tokyo stored the 14th, and
2024-01-15 20:00 in Los Angeles stored the 16th. All-day events shifted by the same mechanism.

## Options

- **Pass a timezone on the components**: EventKit then treats the reminder as timed in that zone,
  not as floating.
- **Compute components in EventKit's own offset.**

## Decision

Compute in EventKit's offset. Every conversion reads `NSTimeZone.defaultTimeZone`'s offset at the
instant in question, so daylight saving is accounted for:

- **Timed** values keep the exact instant, expressed as local wall-clock components.
- **All-day** values are the local calendar day containing the instant; the time of day is
  discarded. All-day events are snapped to local midnight at both ends.

## Consequences

- The same request stores a different calendar day on machines in different timezones, by design:
  the instant names a day where the server runs.
- Moving the server between timezones moves how instants map to all-day dates.

## Evidence

- `packages/apple-eventkit/src/datetime.rs` module docs and tests
  `all_day_keeps_the_local_day_east_of_utc`, `all_day_keeps_the_local_day_west_of_utc`,
  `all_day_round_trips_to_local_midnight`, `timed_values_round_trip_in_both_directions`
  (fixed offsets for Tokyo, +9, and Los Angeles, −8).
- Not yet verified live: creating an all-day reminder with the system set to Asia/Tokyo and to
  America/Los_Angeles (PR #141 manual check).
