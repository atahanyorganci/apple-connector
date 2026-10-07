//! `VTIMEZONE` components for the zones an event's date-times use (RFC 5545 §3.6.5).
//!
//! RFC 5545 §3.2.19 requires a `VTIMEZONE` for every `TZID` a calendar object references. Each one
//! is built from the IANA database `chrono-tz` ships: an initial observance at the start of the
//! first year an event's zoned date-times fall in, then one observance per offset change up to the
//! end of the last such year. Observances are explicit (no `RRULE`), which is valid and exact for
//! the years they cover.

use std::collections::BTreeMap;

use chrono::{DateTime, Datelike, Duration, NaiveDate, Offset, TimeZone, Utc};
use chrono_tz::{OffsetComponents, OffsetName, Tz, TzOffset};

use crate::{
    error::{Error, Result},
    model::{CalendarEvent, EventDateTime},
};

/// How far past its last listed date-time a recurring event's observances reach. An `RRULE` can
/// produce occurrences after every date-time the event lists, and without observances for those
/// years a reader would apply the last listed offset to all of them.
const RECURRENCE_HORIZON_YEARS: i32 = 10;

/// Longest span of years one `VTIMEZONE` covers. Observances are found by stepping a day at a
/// time, so the span bounds the work a hostile input can cause.
const MAX_SPAN_YEARS: i32 = 1000;

/// One `VTIMEZONE` per distinct `TZID` in `event`'s start, end, and exception dates, as CRLF lines.
/// Empty when the event has no zoned date-times.
pub(crate) fn vtimezones(event: &CalendarEvent) -> Result<String> {
    let mut spans: BTreeMap<&str, (i32, i32)> = BTreeMap::new();
    let date_times = event
        .start
        .iter()
        .chain(event.end.iter())
        .chain(event.exception_dates.iter());
    for value in date_times {
        if let EventDateTime::Zoned { timestamp, tzid } = value {
            let year = timestamp.year();
            let span = spans.entry(tzid.as_str()).or_insert((year, year));
            span.0 = span.0.min(year);
            span.1 = span.1.max(year);
        }
    }

    let mut out = String::new();
    for (tzid, (first, last)) in spans {
        let last = if event.recurrence_rule.is_some() {
            last.saturating_add(RECURRENCE_HORIZON_YEARS)
        } else {
            last
        };
        out.push_str(&vtimezone(tzid, first, last)?);
    }
    Ok(out)
}

fn vtimezone(tzid: &str, first_year: i32, last_year: i32) -> Result<String> {
    let span = last_year.saturating_sub(first_year);
    if span > MAX_SPAN_YEARS {
        return Err(Error::LimitExceeded {
            limit: "VTIMEZONE span in years",
            actual: usize::try_from(span).unwrap_or(usize::MAX),
            max: usize::try_from(MAX_SPAN_YEARS).unwrap_or(usize::MAX),
        });
    }
    let zone: Tz = tzid
        .parse()
        .map_err(|_| Error::Serialize(format!("unknown time zone {tzid:?}")))?;
    let start = year_start(first_year)?;
    let end = year_start(last_year.saturating_add(1))?;

    let mut lines = vec!["BEGIN:VTIMEZONE".to_owned(), format!("TZID:{tzid}")];
    let initial = offset_at(&zone, start);
    lines.extend(observance(start, &initial, &initial));

    let mut previous = initial;
    let mut day = start;
    while day < end {
        let next = day + Duration::days(1);
        if !same_offset(&offset_at(&zone, next), &previous) {
            let at = first_change(&zone, day, next, &previous);
            let after = offset_at(&zone, at);
            lines.extend(observance(at, &previous, &after));
            previous = after;
        }
        day = next;
    }
    lines.push("END:VTIMEZONE".to_owned());
    Ok(lines.into_iter().map(|line| line + "\r\n").collect())
}

fn year_start(year: i32) -> Result<DateTime<Utc>> {
    NaiveDate::from_ymd_opt(year, 1, 1)
        .and_then(|date| date.and_hms_opt(0, 0, 0))
        .map(|midnight| Utc.from_utc_datetime(&midnight))
        .ok_or_else(|| Error::Serialize(format!("year {year} is out of range")))
}

fn offset_at(zone: &Tz, instant: DateTime<Utc>) -> TzOffset {
    zone.offset_from_utc_datetime(&instant.naive_utc())
}

fn same_offset(left: &TzOffset, right: &TzOffset) -> bool {
    left.fix() == right.fix() && left.dst_offset() == right.dst_offset()
}

/// The first second in `(before, after]` whose offset differs from `previous`, which is the offset
/// at `before`.
fn first_change(
    zone: &Tz,
    before: DateTime<Utc>,
    after: DateTime<Utc>,
    previous: &TzOffset,
) -> DateTime<Utc> {
    let (mut low, mut high) = (before, after);
    while (high - low) > Duration::seconds(1) {
        let mid = low + (high - low) / 2;
        if same_offset(&offset_at(zone, mid), previous) {
            low = mid;
        } else {
            high = mid;
        }
    }
    high
}

/// A `STANDARD` or `DAYLIGHT` observance starting at `at`. Its `DTSTART` is the wall-clock time
/// in the offset in force before the change, as RFC 5545 §3.6.5 requires.
fn observance(at: DateTime<Utc>, from: &TzOffset, to: &TzOffset) -> Vec<String> {
    let kind = if to.dst_offset().is_zero() {
        "STANDARD"
    } else {
        "DAYLIGHT"
    };
    let from_seconds = from.fix().local_minus_utc();
    let local = at.naive_utc() + Duration::seconds(i64::from(from_seconds));
    let mut lines = vec![
        format!("BEGIN:{kind}"),
        format!("DTSTART:{}", local.format("%Y%m%dT%H%M%S")),
        format!("TZOFFSETFROM:{}", utc_offset(from_seconds)),
        format!("TZOFFSETTO:{}", utc_offset(to.fix().local_minus_utc())),
    ];
    if let Some(name) = to.abbreviation() {
        lines.push(format!("TZNAME:{name}"));
    }
    lines.push(format!("END:{kind}"));
    lines
}

/// RFC 5545 §3.3.14 UTC offset: `+HHMM`, or `+HHMMSS` when there are seconds.
fn utc_offset(seconds: i32) -> String {
    let sign = if seconds < 0 { '-' } else { '+' };
    let total = seconds.unsigned_abs();
    let (hours, minutes, rest) = (total / 3600, total % 3600 / 60, total % 60);
    if rest == 0 {
        format!("{sign}{hours:02}{minutes:02}")
    } else {
        format!("{sign}{hours:02}{minutes:02}{rest:02}")
    }
}

#[cfg(test)]
mod tests {
    use super::utc_offset;

    #[test]
    fn offsets_use_rfc_5545_form() {
        assert_eq!(utc_offset(3 * 3600), "+0300");
        assert_eq!(utc_offset(-(5 * 3600)), "-0500");
        assert_eq!(utc_offset(5 * 3600 + 30 * 60), "+0530");
        assert_eq!(utc_offset(0), "+0000");
        assert_eq!(utc_offset(-(17 * 60 + 30)), "-001730");
    }
}
