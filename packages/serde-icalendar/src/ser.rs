use std::io::Write;

use chrono::{DateTime, NaiveDateTime, Utc};
use icalendar::{
    Alarm as IcsAlarm, Calendar, CalendarDateTime, Component, DatePerhapsTime, Event, EventLike,
    EventStatus as IcsStatus, Parameter, Property,
};

use crate::{
    error::{Error, Result},
    model::{CalendarEvent, EventDateTime, EventStatus},
};

pub fn to_writer<W, T>(mut writer: W, value: &T) -> Result<()>
where
    W: Write,
    T: serde::Serialize,
{
    let event: CalendarEvent = serde_json::from_value(
        serde_json::to_value(value).map_err(|e| Error::Serialize(e.to_string()))?,
    )
    .map_err(|e| Error::Serialize(e.to_string()))?;
    let ics = event_to_ics(&event)?;
    writer
        .write_all(ics.as_bytes())
        .map_err(|e| Error::Serialize(e.to_string()))
}

fn event_to_ics(event: &CalendarEvent) -> Result<String> {
    let mut ics_event = Event::new();
    if let Some(uid) = &event.uid {
        ics_event.uid(uid);
    }
    if let Some(summary) = &event.summary {
        ics_event.summary(summary);
    }
    if let Some(description) = &event.description {
        ics_event.description(description);
    }
    if let Some(location) = &event.location {
        ics_event.location(location);
    }
    if let Some(url) = &event.url {
        ics_event.url(url);
    }
    if let Some(status) = event.status {
        ics_event.status(match status {
            EventStatus::Confirmed => IcsStatus::Confirmed,
            EventStatus::Tentative => IcsStatus::Tentative,
            EventStatus::Cancelled => IcsStatus::Cancelled,
        });
    }
    if let Some(start) = &event.start {
        ics_event.starts(to_date_perhaps_time(start)?);
    }
    if let Some(end) = &event.end {
        // A DATE-valued DTEND is exclusive per RFC 5545 §3.8.2.2; it is written
        // exactly as the model carries it rather than shifted.
        ics_event.ends(to_date_perhaps_time(end)?);
    }
    if let Some(organizer) = &event.organizer {
        let mut prop = Property::new("ORGANIZER", format!("mailto:{}", organizer.email));
        if let Some(name) = &organizer.name {
            prop.append_parameter(Parameter::new("CN", name));
        }
        ics_event.append_property(prop);
    }
    if let Some(rrule) = &event.recurrence_rule {
        ics_event.append_property(Property::new("RRULE", rrule));
    }
    for exdate in &event.exception_dates {
        // EXDATE is multi-valued: append_property writes into a map keyed by
        // property name, so using it here kept only the last exception.
        ics_event.append_multi_property(exdate_property(exdate)?);
    }
    if let Some(sequence) = event.sequence {
        ics_event.sequence(sequence);
    }
    if let Some(extensions) = &event.extensions {
        for (key, value) in &extensions.properties {
            if key.starts_with("X-") {
                ics_event.append_property(Property::new(key, value));
            }
        }
    }
    for alarm in &event.alarms {
        if let Some(trigger) = &alarm.trigger
            && let Ok(duration) = iso8601::duration(trigger)
            && let Ok(chrono_duration) = chrono::Duration::from_std(duration.into())
        {
            ics_event.alarm(IcsAlarm::display(
                alarm.description.as_deref().unwrap_or("Reminder"),
                chrono_duration,
            ));
        }
    }

    let finished = ics_event.done();
    let mut calendar = Calendar::new();
    calendar.push(finished);
    Ok(calendar.to_string())
}

/// Recover the wall-clock time a zoned value denotes in its own zone.
fn local_in_zone(timestamp: DateTime<Utc>, tzid: &str) -> Result<NaiveDateTime> {
    let zone: chrono_tz::Tz = tzid
        .parse()
        .map_err(|_| Error::Serialize(format!("unknown time zone {tzid:?}")))?;
    Ok(timestamp.with_timezone(&zone).naive_local())
}

fn to_date_perhaps_time(value: &EventDateTime) -> Result<DatePerhapsTime> {
    match value {
        EventDateTime::Date { date } => Ok(DatePerhapsTime::Date(*date)),
        EventDateTime::Utc { timestamp } => {
            Ok(DatePerhapsTime::DateTime(CalendarDateTime::Utc(*timestamp)))
        }
        EventDateTime::Floating { local } => Ok(DatePerhapsTime::DateTime(
            CalendarDateTime::Floating(*local),
        )),
        EventDateTime::Zoned { timestamp, tzid } => {
            Ok(DatePerhapsTime::DateTime(CalendarDateTime::WithTimezone {
                date_time: local_in_zone(*timestamp, tzid)?,
                tzid: tzid.clone(),
            }))
        }
    }
}

fn exdate_property(value: &EventDateTime) -> Result<Property> {
    let property = match value {
        EventDateTime::Date { date } => {
            let mut property = Property::new("EXDATE", date.format("%Y%m%d").to_string());
            property.append_parameter(Parameter::new("VALUE", "DATE"));
            property.done()
        }
        EventDateTime::Utc { timestamp } => {
            Property::new("EXDATE", timestamp.format("%Y%m%dT%H%M%SZ").to_string())
        }
        EventDateTime::Floating { local } => {
            Property::new("EXDATE", local.format("%Y%m%dT%H%M%S").to_string())
        }
        EventDateTime::Zoned { timestamp, tzid } => {
            let local = local_in_zone(*timestamp, tzid)?;
            let mut property = Property::new("EXDATE", local.format("%Y%m%dT%H%M%S").to_string());
            property.append_parameter(Parameter::new("TZID", tzid));
            property.done()
        }
    };
    Ok(property)
}
