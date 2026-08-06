use chrono::{Datelike, NaiveDate, TimeZone, Timelike, Utc};
use objc2_foundation::{
    NSCalendar, NSCalendarIdentifierGregorian, NSDate, NSDateComponentUndefined, NSDateComponents,
    NSTimeZone,
};

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

pub fn unix_to_date_components(
    secs: i64,
    all_day: bool,
) -> EventKitResult<objc2::rc::Retained<NSDateComponents>> {
    let dt = Utc
        .timestamp_opt(secs, 0)
        .single()
        .ok_or_else(|| EventKitError::ValidationFailed("invalid unix timestamp".into()))?;
    let components = NSDateComponents::new();
    components.setYear(i32_to_isize(dt.year())?);
    components.setMonth(u32_to_isize(dt.month())?);
    components.setDay(u32_to_isize(dt.day())?);
    if all_day {
        components.setHour(NSDateComponentUndefined);
        components.setMinute(NSDateComponentUndefined);
        components.setSecond(NSDateComponentUndefined);
    } else {
        components.setHour(u32_to_isize(dt.hour())?);
        components.setMinute(u32_to_isize(dt.minute())?);
        components.setSecond(u32_to_isize(dt.second())?);
    }
    Ok(components)
}

#[cfg_attr(not(test), allow(dead_code))]
pub fn date_components_to_unix(
    components: &NSDateComponents,
    all_day: bool,
) -> EventKitResult<i64> {
    let calendar = unsafe { NSCalendar::calendarWithIdentifier(NSCalendarIdentifierGregorian) }
        .ok_or_else(|| EventKitError::Framework("gregorian calendar unavailable".into()))?;
    let utc = NSTimeZone::timeZoneWithName(objc2_foundation::ns_string!("UTC"))
        .ok_or_else(|| EventKitError::Framework("UTC timezone unavailable".into()))?;
    calendar.setTimeZone(&utc);
    let date = calendar.dateFromComponents(components).ok_or_else(|| {
        EventKitError::ValidationFailed("could not convert date components to NSDate".into())
    })?;
    let secs = ns_date_to_unix(&date);
    if all_day {
        let dt = Utc
            .timestamp_opt(secs, 0)
            .single()
            .ok_or_else(|| EventKitError::ValidationFailed("invalid unix timestamp".into()))?;
        let date = NaiveDate::from_ymd_opt(dt.year(), dt.month(), dt.day())
            .ok_or_else(|| EventKitError::ValidationFailed("invalid date".into()))?;
        let midnight = date.and_hms_opt(0, 0, 0).ok_or_else(|| {
            EventKitError::ValidationFailed("invalid all-day midnight timestamp".into())
        })?;
        return Ok(Utc.from_utc_datetime(&midnight).timestamp());
    }
    Ok(secs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unix_round_trip_timed() -> Result<(), Box<dyn std::error::Error>> {
        let secs = 1_700_000_000_i64;
        let components = unix_to_date_components(secs, false)?;
        let back = date_components_to_unix(&components, false)?;
        assert_eq!(back, secs);
        Ok(())
    }

    #[test]
    fn unix_round_trip_all_day() -> Result<(), Box<dyn std::error::Error>> {
        let secs = 1_700_000_000_i64;
        let components = unix_to_date_components(secs, true)?;
        let back = date_components_to_unix(&components, true)?;
        let dt = Utc
            .timestamp_opt(back, 0)
            .single()
            .ok_or("invalid timestamp")?;
        assert_eq!(dt.hour(), 0);
        assert_eq!(dt.minute(), 0);
        Ok(())
    }
}
