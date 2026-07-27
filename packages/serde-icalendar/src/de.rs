use chrono::{DateTime, NaiveDate, TimeZone, Utc};
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
        exception_dates: ics_event
            .multi_properties()
            .get("EXDATE")
            .map(|properties| {
                properties
                    .iter()
                    .flat_map(|property| parse_exdate(property.value()))
                    .collect()
            })
            .unwrap_or_default(),
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

fn parse_exdate(value: &str) -> Option<EventDateTime> {
    let all_day = !value.contains('T');
    let timestamp = if all_day {
        let date = NaiveDate::parse_from_str(value, "%Y%m%d").ok()?;
        Utc.from_utc_datetime(&date.and_hms_opt(0, 0, 0)?)
    } else {
        DateTime::parse_from_rfc3339(&format_ics_to_rfc3339(value))
            .ok()?
            .with_timezone(&Utc)
    };
    Some(EventDateTime {
        timestamp,
        all_day,
        tzid: None,
    })
}

fn format_ics_to_rfc3339(value: &str) -> String {
    if value.ends_with('Z') {
        let trimmed = value.trim_end_matches('Z');
        format!(
            "{}-{}-{}T{}:{}:{}Z",
            &trimmed[0..4],
            &trimmed[4..6],
            &trimmed[6..8],
            &trimmed[9..11],
            &trimmed[11..13],
            &trimmed[13..15]
        )
    } else {
        value.to_owned()
    }
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
