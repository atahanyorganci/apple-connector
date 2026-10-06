# `serde-icalendar`: RFC 5545 iCalendar reader and writer for single events

Reads the first `VEVENT` of an iCalendar document into a `CalendarEvent` and writes a
`CalendarEvent` as a one-event `VCALENDAR`. `apple-connector` uses it for the `/iCal` event routes,
and `serde-caldav` uses it for embedded `calendar-data`. Parsing is built on the `icalendar` crate.

## Kind

Codec.

## Public API

- `from_str(&str)`, `from_slice(&[u8])`, `parse_ics(&[u8])` → `Result<CalendarEvent>`.
- `from_reader(R)` and `from_reader_with_limit(R, limit)`.
- `to_string(&CalendarEvent) -> Result<String>`, `to_writer(W, &CalendarEvent)`.
- Model: `CalendarEvent`, `EventDateTime`
  ([ICAL-0001](decisions/ICAL-0001-datetime-model.md)), `EventStatus`, `Organizer`, `Attendee`,
  `Role`, `ParticipationStatus`, `CalendarUserType`, `Alarm`, `AlarmTrigger`, `TriggerRelation`,
  `ExtensionBag`.
- `Error`: `Parse(String)`, `Serialize(String)`, `Custom(String)`,
  `LimitExceeded { limit, actual, max }`.
- `MAX_INPUT_BYTES`: 16 MiB.

## Guarantees

- A `TZID` wall-clock time converts to the right UTC instant, including across DST, and `TZID`
  survives a round trip; UTC stays UTC and floating stays floating. Enforced by:
  `tests/datetime.rs` (`tzid_wall_time_converts_to_the_right_instant`,
  `tzid_conversion_respects_dst`, `tzid_survives_a_round_trip`, `utc_values_round_trip_as_utc`,
  `floating_times_stay_floating`).
- A DATE-valued `DTEND` is exclusive and is written exactly as held. Enforced by:
  `tests/datetime.rs::all_day_end_is_exclusive_and_round_trips`,
  `multi_day_all_day_events_keep_their_span`.
- Unknown time zones and times inside a DST gap are errors, not shifts. Enforced by:
  `tests/datetime.rs::unknown_start_time_zones_are_reported`,
  `tests/exdate.rs::a_dst_gap_is_reported_rather_than_shifted`.
- Every `EXDATE` value is kept: comma-separated lists, `TZID` applied to each value, and DATE-only
  values; malformed values are errors and never panics; every exception is written back. Enforced
  by: `tests/exdate.rs`, `tests/datetime.rs::every_exception_date_is_serialized`.
- `ATTENDEE`, `ORGANIZER` (with `CN`), and `VALARM` round-trip; `ROLE`, `PARTSTAT`, and `CUTYPE`
  use RFC tokens, and unknown tokens and unmodelled parameters are preserved. Enforced by:
  `tests/participants.rs` over Apple, Google, and Outlook invites.
- `X-` properties round-trip through `extensions`. Enforced by:
  `tests/spec.rs::only_x_properties_survive_outside_the_model`.
- Output lines end in CRLF. Enforced by: `tests/spec.rs::lines_end_with_crlf`.
- Every `TZID` the event uses gets one `VTIMEZONE`, placed before the `VEVENT`, with an
  observance for each offset change in the years its zoned date-times fall in; a UTC-only event
  gets none (RFC 5545 §3.2.19, §3.6.5). Enforced by: `tests/spec.rs`
  (`zoned_times_are_written_with_their_vtimezone`, `vtimezone_observances_follow_the_zone_rules`,
  `one_vtimezone_per_zone_and_none_without_zones`, `documents_with_a_vtimezone_parse_back`).
- `TRIGGER` durations are validated with checked arithmetic when writing, and the original spelling
  is written back. Enforced by: `tests/participants.rs::an_absurd_trigger_duration_is_rejected_not_a_panic`,
  `malformed_trigger_durations_are_rejected`
  ([ICAL-L-0001](lessons/ICAL-L-0001-iso8601-duration-panic.md)).
- Readers never read more than their limit. Enforced by: `tests/limits.rs`.
- No input panics `from_slice` followed by `to_string`. Enforced by: the `icalendar_parse` fuzz
  target.

### Known bugs

None known.

## Limits and non-goals

- One event: only the first `VEVENT` is read (`tests/spec.rs::only_the_first_vevent_is_parsed`);
  `VTODO`, `VJOURNAL`, `VFREEBUSY`, and `VTIMEZONE` components are ignored.
- Properties outside the model and not prefixed `X-` (`CATEGORIES`, `CLASS`, `TRANSP`, …) are
  dropped. `X-` properties keep their value but lose their parameters.
- `RRULE` is kept as a raw string; recurrence is not expanded or validated.
- `VTIMEZONE` observances are explicit, without `RRULE`. For a recurring event they reach ten years
  past its last listed date-time, so occurrences later than that use the last observance's
  offset. Covering more than 1000 years is a `LimitExceeded` error. A `VTIMEZONE` in the input is
  ignored; zones resolve through `chrono-tz`.
- `TRIGGER` durations are validated only when writing, so a document can parse and then fail to
  serialize (`tests/spec.rs::trigger_durations_are_validated_on_write_only`).
- An absolute `TRIGGER` is read as UTC whether or not it ends in `Z`.
- Errors carry a message, not a line or column.

## Platform and permissions

Any platform; no permissions.
