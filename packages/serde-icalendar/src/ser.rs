use std::io::Write;

use icalendar::{
    Alarm as IcsAlarm, Calendar, Component, Event, EventLike, EventStatus as IcsStatus, Parameter,
    Property,
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
        apply_start(&mut ics_event, start);
    }
    if let Some(end) = &event.end {
        apply_end(&mut ics_event, end);
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
        ics_event.append_property(Property::new("EXDATE", format_datetime(exdate)));
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

fn apply_start(event: &mut Event, dt: &EventDateTime) {
    if dt.all_day {
        event.starts(dt.timestamp.date_naive());
    } else {
        event.starts(dt.timestamp);
    }
}

fn apply_end(event: &mut Event, dt: &EventDateTime) {
    if dt.all_day {
        event.ends(dt.timestamp.date_naive());
    } else {
        event.ends(dt.timestamp);
    }
}

fn format_datetime(dt: &EventDateTime) -> String {
    if dt.all_day {
        dt.timestamp.format("%Y%m%d").to_string()
    } else {
        dt.timestamp.format("%Y%m%dT%H%M%SZ").to_string()
    }
}
