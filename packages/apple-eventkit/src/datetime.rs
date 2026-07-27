use chrono::{Datelike, NaiveDate, TimeZone, Timelike, Utc};
use objc2_foundation::{
    NSCalendar, NSCalendarIdentifierGregorian, NSDate, NSDateComponentUndefined, NSDateComponents,
    NSTimeZone,
};

use crate::error::{EventKitError, EventKitResult};

pub fn unix_to_ns_date(secs: i64) -> EventKitResult<objc2::rc::Retained<NSDate>> {
    let dt = Utc
        .timestamp_opt(secs, 0)
        .single()
        .ok_or_else(|| EventKitError::ValidationFailed("invalid unix timestamp".into()))?;
    let interval = dt.timestamp() as f64;
    Ok(NSDate::dateWithTimeIntervalSince1970(interval))
}

pub fn ns_date_to_unix(date: &NSDate) -> i64 {
    date.timeIntervalSince1970() as i64
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
    components.setYear(dt.year() as isize);
    components.setMonth(dt.month() as isize);
    components.setDay(dt.day() as isize);
    if all_day {
        components.setHour(NSDateComponentUndefined);
        components.setMinute(NSDateComponentUndefined);
        components.setSecond(NSDateComponentUndefined);
    } else {
        components.setHour(dt.hour() as isize);
        components.setMinute(dt.minute() as isize);
        components.setSecond(dt.second() as isize);
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
        return Ok(Utc
            .from_utc_datetime(&date.and_hms_opt(0, 0, 0).expect("midnight"))
            .timestamp());
    }
    Ok(secs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unix_round_trip_timed() {
        let secs = 1_700_000_000_i64;
        let components = unix_to_date_components(secs, false).expect("components");
        let back = date_components_to_unix(&components, false).expect("unix");
        assert_eq!(back, secs);
    }

    #[test]
    fn unix_round_trip_all_day() {
        let secs = 1_700_000_000_i64;
        let components = unix_to_date_components(secs, true).expect("components");
        let back = date_components_to_unix(&components, true).expect("unix");
        let dt = Utc.timestamp_opt(back, 0).single().expect("dt");
        assert_eq!(dt.hour(), 0);
        assert_eq!(dt.minute(), 0);
    }
}
