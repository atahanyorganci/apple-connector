---
id: EK-L-0001
status: graduated
observed-on: "macOS 26.5.2; EKReminder.dueDateComponents (NSDateComponents without a timeZone)"
graduated-to: "packages/apple-eventkit/src/datetime.rs"
provenance:
  - https://github.com/atahanyorganci/apple-connector/issues/79
  - https://github.com/atahanyorganci/apple-connector/pull/141
---

# EventKit reads `NSDateComponents` that have no time zone in the system's default time zone.

## What happened

Due dates were split into calendar fields in UTC. A due of 2024-01-15 00:00 in Tokyo was stored as
the 14th; 2024-01-15 20:00 in Los Angeles as the 16th. All-day events shifted the same way.

## Why

Components without a `timeZone` are floating: EventKit interprets them as wall-clock values in
`NSTimeZone.defaultTimeZone`. Components computed in UTC therefore name the wrong wall-clock time
everywhere but UTC, and near either end of the local day, the wrong day.

## Rule

Compute calendar fields in the offset EventKit will apply — `NSTimeZone.defaultTimeZone` at that
instant — and treat an all-day value as the local day containing the instant.

## Graduated

`packages/apple-eventkit/src/datetime.rs` tests, with fixed offsets standing in for Tokyo (+9)
and Los Angeles (−8): `all_day_keeps_the_local_day_east_of_utc`,
`all_day_keeps_the_local_day_west_of_utc`, `all_day_round_trips_to_local_midnight`,
`timed_values_round_trip_in_both_directions`. The end-to-end check — an all-day reminder created
with the system set to each zone — has not been run (PR #141 manual check).
