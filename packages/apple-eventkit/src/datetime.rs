//! Conversions between Unix instants and the calendar values EventKit stores.
//!
//! EventKit reads timezone-less `NSDateComponents` in the *default* timezone. Components derived
//! from UTC therefore land on the wrong wall-clock time everywhere outside UTC, and — for an
//! instant near either end of the local day — on the wrong calendar day. Every conversion here is
//! parameterised by the offset EventKit itself will apply, read from `NSTimeZone` at the instant
//! in question so daylight saving is accounted for.
//!
//! The semantics this fixes in place:
//!
//! - An **all-day** item is the local calendar day *containing* the supplied instant. The time of
//!   day is discarded, and the day the caller asked for is the day that gets stored.
//! - A **timed** item preserves the exact instant, expressed as local wall-clock components
//!   because that is how EventKit reads them back.

use chrono::{Datelike, FixedOffset, LocalResult, NaiveDate, TimeZone, Timelike, Utc};
use objc2_foundation::{NSDate, NSDateComponentUndefined, NSDateComponents, NSTimeZone};

use crate::error::{EventKitError, EventKitResult};

fn f64_to_i64_secs(value: f64) -> i64 {
    if !value.is_finite() {
        return 0;
    }
    if value >= i64::MAX as f64 {
        return i64::MAX;
    }
    if value <= i64::MIN as f64 {
        return i64::MIN;
    }
    #[expect(clippy::cast_possible_truncation)]
    {
        value.trunc() as i64
    }
}

fn i32_to_isize(value: i32) -> EventKitResult<isize> {
    isize::try_from(value).map_err(|_| EventKitError::Framework("i32 does not fit in isize".into()))
}

fn u32_to_isize(value: u32) -> EventKitResult<isize> {
    isize::try_from(value).map_err(|_| EventKitError::Framework("u32 does not fit in isize".into()))
}

fn isize_to_i32(value: isize) -> EventKitResult<i32> {
    i32::try_from(value)
        .map_err(|_| EventKitError::ValidationFailed("date component out of range".into()))
}

fn isize_to_u32(value: isize) -> EventKitResult<u32> {
    u32::try_from(value)
        .map_err(|_| EventKitError::ValidationFailed("date component out of range".into()))
}

/// Calendar fields as EventKit will read them: already in the target timezone.
///
/// `time` is `None` for an all-day value, which is what leaves the hour, minute, and second
/// components undefined.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CalendarFields {
    pub year: i32,
    pub month: u32,
    pub day: u32,
    pub time: Option<(u32, u32, u32)>,
}

/// The UTC offset EventKit will apply to timezone-less components at `secs`.
pub fn local_offset_for(secs: i64) -> EventKitResult<FixedOffset> {
    let date = unix_to_ns_date(secs)?;
    let seconds = NSTimeZone::defaultTimeZone().secondsFromGMTForDate(&date);
    let seconds = isize_to_i32(seconds)?;
    FixedOffset::east_opt(seconds)
        .ok_or_else(|| EventKitError::Framework("system timezone offset is out of range".into()))
}

/// Splits an instant into the calendar fields that describe it in `offset`.
pub(crate) fn calendar_fields_in(
    secs: i64,
    all_day: bool,
    offset: FixedOffset,
) -> EventKitResult<CalendarFields> {
    let local = offset
        .timestamp_opt(secs, 0)
        .single()
        .ok_or_else(|| EventKitError::ValidationFailed("invalid unix timestamp".into()))?;
    Ok(CalendarFields {
        year: local.year(),
        month: local.month(),
        day: local.day(),
        time: if all_day {
            None
        } else {
            Some((local.hour(), local.minute(), local.second()))
        },
    })
}

/// Rebuilds the instant that `fields` names in `offset`. All-day fields resolve to local midnight.
pub(crate) fn unix_from_calendar_fields_in(
    fields: CalendarFields,
    offset: FixedOffset,
) -> EventKitResult<i64> {
    let (hour, minute, second) = fields.time.unwrap_or((0, 0, 0));
    let naive = NaiveDate::from_ymd_opt(fields.year, fields.month, fields.day)
        .and_then(|date| date.and_hms_opt(hour, minute, second))
        .ok_or_else(|| EventKitError::ValidationFailed("invalid calendar date".into()))?;
    match offset.from_local_datetime(&naive) {
        LocalResult::Single(local) => Ok(local.timestamp()),
        // A fixed offset never leaves a local time ambiguous or absent, but the mapping is
        // fallible in general and must not be assumed away.
        _ => Err(EventKitError::ValidationFailed(
            "calendar date does not exist in the local timezone".into(),
        )),
    }
}

/// The instant at which the local calendar day containing `secs` begins.
pub fn start_of_local_day(secs: i64) -> EventKitResult<i64> {
    let offset = local_offset_for(secs)?;
    let fields = calendar_fields_in(secs, true, offset)?;
    let midnight = unix_from_calendar_fields_in(fields, offset)?;

    // In a zone whose offset changes across midnight, the offset in force at the instant we
    // started from is not the one in force at midnight; one refinement settles it, and we keep
    // the result only if it still names the day the caller asked for.
    let refined_offset = local_offset_for(midnight)?;
    if refined_offset == offset {
        return Ok(midnight);
    }
    let refined = unix_from_calendar_fields_in(fields, refined_offset)?;
    let refined_fields = calendar_fields_in(refined, true, local_offset_for(refined)?)?;
    if refined_fields == fields {
        Ok(refined)
    } else {
        Ok(midnight)
    }
}

pub fn unix_to_ns_date(secs: i64) -> EventKitResult<objc2::rc::Retained<NSDate>> {
    let dt = Utc
        .timestamp_opt(secs, 0)
        .single()
        .ok_or_else(|| EventKitError::ValidationFailed("invalid unix timestamp".into()))?;
    let interval = dt.timestamp() as f64;
    Ok(NSDate::dateWithTimeIntervalSince1970(interval))
}

pub fn ns_date_to_unix(date: &NSDate) -> i64 {
    f64_to_i64_secs(date.timeIntervalSince1970())
}

pub fn retained_date_to_unix(date: &objc2::rc::Retained<NSDate>) -> i64 {
    ns_date_to_unix(date.as_ref())
}

/// Builds the components EventKit stores for a due date, in the timezone it will read them in.
pub fn unix_to_date_components(
    secs: i64,
    all_day: bool,
) -> EventKitResult<objc2::rc::Retained<NSDateComponents>> {
    let offset = local_offset_for(secs)?;
    let fields = calendar_fields_in(secs, all_day, offset)?;

    let components = NSDateComponents::new();
    components.setYear(i32_to_isize(fields.year)?);
    components.setMonth(u32_to_isize(fields.month)?);
    components.setDay(u32_to_isize(fields.day)?);
    match fields.time {
        Some((hour, minute, second)) => {
            components.setHour(u32_to_isize(hour)?);
            components.setMinute(u32_to_isize(minute)?);
            components.setSecond(u32_to_isize(second)?);
        }
        None => {
            components.setHour(NSDateComponentUndefined);
            components.setMinute(NSDateComponentUndefined);
            components.setSecond(NSDateComponentUndefined);
        }
    }
    Ok(components)
}

#[cfg_attr(not(test), allow(dead_code))]
pub fn date_components_to_unix(
    components: &NSDateComponents,
    all_day: bool,
) -> EventKitResult<i64> {
    let fields = CalendarFields {
        year: isize_to_i32(components.year())?,
        month: isize_to_u32(components.month())?,
        day: isize_to_u32(components.day())?,
        time: if all_day {
            None
        } else {
            Some((
                isize_to_u32(components.hour())?,
                isize_to_u32(components.minute())?,
                isize_to_u32(components.second())?,
            ))
        },
    };

    // The offset wanted is the one in force on that local day, so finding it needs an instant on
    // the day itself. Noon UTC is far enough from either boundary to name the right day under any
    // real-world offset.
    let noon = CalendarFields {
        time: Some((12, 0, 0)),
        ..fields
    };
    let utc = FixedOffset::east_opt(0)
        .ok_or_else(|| EventKitError::Framework("zero timezone offset is invalid".into()))?;
    let probe = unix_from_calendar_fields_in(noon, utc)?;
    unix_from_calendar_fields_in(fields, local_offset_for(probe)?)
}

#[cfg(test)]
mod tests {
    use chrono::FixedOffset;

    use super::*;

    /// Explicit offsets rather than the `TZ` environment variable, which is `unsafe` to set in
    /// edition 2024 and would race across test threads.
    fn offset(hours: i32) -> Result<FixedOffset, Box<dyn std::error::Error>> {
        Ok(FixedOffset::east_opt(hours * 3600).ok_or("offset out of range")?)
    }

    fn tokyo() -> Result<FixedOffset, Box<dyn std::error::Error>> {
        offset(9)
    }

    fn los_angeles() -> Result<FixedOffset, Box<dyn std::error::Error>> {
        offset(-8)
    }

    /// 2024-01-15T00:00+09:00 is 2024-01-14T15:00Z. Deriving the day from UTC stored it as the
    /// 14th; deriving it locally keeps the 15th the caller asked for.
    #[test]
    fn all_day_keeps_the_local_day_east_of_utc() -> Result<(), Box<dyn std::error::Error>> {
        let instant = tokyo()?
            .with_ymd_and_hms(2024, 1, 15, 0, 0, 0)
            .single()
            .ok_or("ambiguous local time")?
            .timestamp();
        let fields = calendar_fields_in(instant, true, tokyo()?)?;
        assert_eq!((fields.year, fields.month, fields.day), (2024, 1, 15));
        assert_eq!(fields.time, None);
        Ok(())
    }

    /// 2024-01-15T20:00-08:00 is 2024-01-16T04:00Z — the UTC day is already tomorrow.
    #[test]
    fn all_day_keeps_the_local_day_west_of_utc() -> Result<(), Box<dyn std::error::Error>> {
        let instant = los_angeles()?
            .with_ymd_and_hms(2024, 1, 15, 20, 0, 0)
            .single()
            .ok_or("ambiguous local time")?
            .timestamp();
        let fields = calendar_fields_in(instant, true, los_angeles()?)?;
        assert_eq!((fields.year, fields.month, fields.day), (2024, 1, 15));
        Ok(())
    }

    #[test]
    fn all_day_round_trips_to_local_midnight() -> Result<(), Box<dyn std::error::Error>> {
        for offset in [tokyo()?, los_angeles()?] {
            let instant = offset
                .with_ymd_and_hms(2024, 3, 9, 23, 30, 0)
                .single()
                .ok_or("ambiguous local time")?
                .timestamp();
            let fields = calendar_fields_in(instant, true, offset)?;
            let midnight = unix_from_calendar_fields_in(fields, offset)?;
            let back = calendar_fields_in(midnight, false, offset)?;
            assert_eq!((back.year, back.month, back.day), (2024, 3, 9));
            assert_eq!(back.time, Some((0, 0, 0)));
        }
        Ok(())
    }

    /// A timed value keeps its instant: the components describe local wall-clock, and reading
    /// them back in the same offset returns the second we started from.
    #[test]
    fn timed_values_round_trip_in_both_directions() -> Result<(), Box<dyn std::error::Error>> {
        let secs = 1_700_000_000_i64;
        for offset in [tokyo()?, los_angeles()?] {
            let fields = calendar_fields_in(secs, false, offset)?;
            assert_eq!(unix_from_calendar_fields_in(fields, offset)?, secs);
        }
        Ok(())
    }

    /// The wall-clock a user sees differs by offset even though the instant does not — which is
    /// exactly what deriving components from UTC used to get wrong.
    #[test]
    fn timed_components_are_local_wall_clock() -> Result<(), Box<dyn std::error::Error>> {
        let secs = tokyo()?
            .with_ymd_and_hms(2024, 6, 1, 9, 30, 0)
            .single()
            .ok_or("ambiguous local time")?
            .timestamp();
        assert_eq!(
            calendar_fields_in(secs, false, tokyo()?)?.time,
            Some((9, 30, 0))
        );
        // Seventeen hours behind Tokyo, so the same instant is the previous afternoon.
        let pacific = calendar_fields_in(secs, false, los_angeles()?)?;
        assert_eq!(pacific.time, Some((16, 30, 0)));
        assert_eq!((pacific.month, pacific.day), (5, 31));
        Ok(())
    }

    #[test]
    fn start_of_day_precedes_the_instant_it_contains() -> Result<(), Box<dyn std::error::Error>> {
        let secs = 1_700_000_000_i64;
        let start = start_of_local_day(secs)?;
        assert!(start <= secs);
        assert!(secs - start < 24 * 3600);
        let offset = local_offset_for(start)?;
        assert_eq!(
            calendar_fields_in(start, false, offset)?.time,
            Some((0, 0, 0))
        );
        Ok(())
    }

    #[test]
    fn unix_round_trip_timed() -> Result<(), Box<dyn std::error::Error>> {
        let secs = 1_700_000_000_i64;
        let components = unix_to_date_components(secs, false)?;
        assert_eq!(date_components_to_unix(&components, false)?, secs);
        Ok(())
    }

    #[test]
    fn unix_round_trip_all_day() -> Result<(), Box<dyn std::error::Error>> {
        let secs = 1_700_000_000_i64;
        let components = unix_to_date_components(secs, true)?;
        let back = date_components_to_unix(&components, true)?;
        assert_eq!(back, start_of_local_day(secs)?);
        Ok(())
    }
}
