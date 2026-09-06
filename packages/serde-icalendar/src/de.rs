use chrono::{DateTime, NaiveDate, NaiveDateTime, TimeZone, Utc};
use icalendar::{Calendar, CalendarDateTime, Component, DatePerhapsTime, EventLike};

use crate::{
    error::{Error, Result},
    model::{CalendarEvent, EventDateTime, EventStatus, ExtensionBag, Organizer},
};

pub fn parse_ics(input: &[u8]) -> Result<CalendarEvent> {
    let text = std::str::from_utf8(input).map_err(|e| Error::Parse(e.to_string()))?;
    let calendar: Calendar = text
        .parse::<Calendar>()
        .map_err(|e: String| Error::Parse(e))?;
    let ics_event = calendar
        .events()
        .next()
        .ok_or_else(|| Error::Parse("no VEVENT found".to_owned()))?;

    Ok(CalendarEvent {
        uid: ics_event.get_uid().map(str::to_owned),
        summary: ics_event.get_summary().map(str::to_owned),
        description: ics_event.get_description().map(str::to_owned),
        location: ics_event.get_location().map(str::to_owned),
        url: ics_event.get_url().map(str::to_owned),
        status: ics_event.get_status().map(|status| match status {
            icalendar::EventStatus::Confirmed => EventStatus::Confirmed,
            icalendar::EventStatus::Tentative => EventStatus::Tentative,
            icalendar::EventStatus::Cancelled => EventStatus::Cancelled,
        }),
        start: ics_event.get_start().map(date_perhaps_time_to_event),
        end: ics_event.get_end().map(date_perhaps_time_to_event),
        organizer: ics_event
            .property_value("ORGANIZER")
            .and_then(parse_organizer),
        attendees: ics_event
            .get_attendees()
            .into_iter()
            .map(|attendee| crate::model::Attendee {
                email: attendee
                    .cal_address
                    .strip_prefix("mailto:")
                    .or_else(|| attendee.cal_address.strip_prefix("MAILTO:"))
                    .unwrap_or(attendee.cal_address.as_str())
                    .to_owned(),
                name: attendee.cn.clone(),
                role: attendee.role.map(|role| format!("{role:?}")),
                partstat: attendee.part_stat.map(|part| format!("{part:?}")),
                rsvp: attendee.rsvp,
            })
            .collect(),
        alarms: Vec::new(),
        recurrence_rule: ics_event.property_value("RRULE").map(str::to_owned),
        exception_dates: parse_exception_dates(ics_event)?,
        sequence: ics_event.get_sequence(),
        extensions: Some(parse_extensions(ics_event)),
    })
}

fn date_perhaps_time_to_event(dt: DatePerhapsTime) -> EventDateTime {
    match dt {
        DatePerhapsTime::Date(date) => EventDateTime {
            timestamp: Utc.from_utc_datetime(&date.and_hms_opt(0, 0, 0).unwrap_or_default()),
            all_day: true,
            tzid: None,
        },
        DatePerhapsTime::DateTime(calendar_dt) => {
            let (timestamp, tzid) = match calendar_dt {
                CalendarDateTime::Utc(value) => (value, None),
                CalendarDateTime::Floating(value) => (Utc.from_utc_datetime(&value), None),
                CalendarDateTime::WithTimezone { date_time, tzid } => {
                    (Utc.from_utc_datetime(&date_time), Some(tzid))
                }
            };
            EventDateTime {
                timestamp,
                all_day: false,
                tzid,
            }
        }
    }
}

/// Collect every EXDATE value on the event.
///
/// RFC 5545 allows one EXDATE property to carry a comma-separated list, and the
/// TZID parameter applies to every value in that list. A value that cannot be
/// parsed is an error rather than a silently dropped exception, so callers can
/// tell "no exceptions" apart from "we could not read the exceptions".
fn parse_exception_dates(event: &icalendar::Event) -> Result<Vec<EventDateTime>> {
    let Some(properties) = event.multi_properties().get("EXDATE") else {
        return Ok(Vec::new());
    };

    let mut dates = Vec::new();
    for property in properties {
        let tzid = property
            .params()
            .get("TZID")
            .map(|parameter| parameter.value().to_owned());
        for value in property.value().split(',') {
            let value = value.trim();
            if value.is_empty() {
                continue;
            }
            dates.push(parse_exdate(value, tzid.clone())?);
        }
    }
    Ok(dates)
}

fn parse_exdate(value: &str, tzid: Option<String>) -> Result<EventDateTime> {
    // DATE form: 20240101
    if !value.contains('T') {
        let date = NaiveDate::parse_from_str(value, "%Y%m%d")
            .map_err(|error| Error::Parse(format!("invalid EXDATE value {value:?}: {error}")))?;
        let midnight = date
            .and_hms_opt(0, 0, 0)
            .ok_or_else(|| Error::Parse(format!("invalid EXDATE date {value:?}")))?;
        return Ok(EventDateTime {
            timestamp: Utc.from_utc_datetime(&midnight),
            all_day: true,
            tzid,
        });
    }

    // UTC form: 20240101T120000Z. The trailing Z wins over any TZID parameter.
    if let Some(without_zulu) = value.strip_suffix('Z') {
        let naive = parse_ics_naive(without_zulu, value)?;
        return Ok(EventDateTime {
            timestamp: Utc.from_utc_datetime(&naive),
            all_day: false,
            tzid: None,
        });
    }

    // Zoned or floating form: 20240101T120000
    let naive = parse_ics_naive(value, value)?;
    let timestamp = match tzid.as_deref() {
        Some(zone) => resolve_zoned(naive, zone)?,
        None => Utc.from_utc_datetime(&naive),
    };
    Ok(EventDateTime {
        timestamp,
        all_day: false,
        tzid,
    })
}

fn parse_ics_naive(value: &str, original: &str) -> Result<NaiveDateTime> {
    NaiveDateTime::parse_from_str(value, "%Y%m%dT%H%M%S")
        .map_err(|error| Error::Parse(format!("invalid EXDATE value {original:?}: {error}")))
}

/// Resolve a wall-clock time in a named zone to the UTC instant it denotes.
///
/// Ambiguous local times (a DST fall-back) resolve to the earlier offset;
/// non-existent ones (a spring-forward gap) are an error rather than a silent
/// shift.
fn resolve_zoned(naive: NaiveDateTime, tzid: &str) -> Result<DateTime<Utc>> {
    let zone: chrono_tz::Tz = tzid
        .parse()
        .map_err(|_| Error::Parse(format!("unknown time zone {tzid:?}")))?;
    let local = zone.from_local_datetime(&naive);
    local
        .single()
        .or_else(|| local.earliest())
        .map(|resolved| resolved.with_timezone(&Utc))
        .ok_or_else(|| {
            Error::Parse(format!(
                "local time {naive} does not exist in time zone {tzid}"
            ))
        })
}

fn parse_organizer(value: &str) -> Option<Organizer> {
    let email = value
        .strip_prefix("mailto:")
        .or_else(|| value.strip_prefix("MAILTO:"))
        .unwrap_or(value)
        .to_owned();
    Some(Organizer { email, name: None })
}

fn parse_extensions(event: &icalendar::Event) -> ExtensionBag {
    use std::collections::BTreeMap;
    let mut properties = BTreeMap::new();
    for (key, property) in event.properties() {
        if key.starts_with("X-") {
            properties.insert(key.clone(), property.value().to_owned());
        }
    }
    ExtensionBag { properties }
}
