use std::{collections::BTreeMap, io::Write};

use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use icalendar::{
    Alarm as IcsAlarm, Calendar, CalendarDateTime, Component, DatePerhapsTime, Event, EventLike,
    EventStatus as IcsStatus, Parameter, Property, Trigger,
};

use crate::{
    error::{Error, Result},
    model::{
        Alarm, AlarmTrigger, Attendee, CalendarEvent, EventDateTime, EventStatus, Organizer,
        TriggerRelation,
    },
};

pub fn to_writer<W: Write>(mut writer: W, event: &CalendarEvent) -> Result<()> {
    let ics = event_to_ics(event)?;
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
        ics_event.append_property(organizer_property(organizer));
    }
    for attendee in &event.attendees {
        // ATTENDEE is multi-valued; a map keyed by property name would keep
        // only the last one.
        ics_event.append_multi_property(attendee_property(attendee));
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
        ics_event.alarm(build_alarm(alarm)?);
    }

    let finished = ics_event.done();
    let mut calendar = Calendar::new();
    calendar.push(finished);
    Ok(calendar.to_string())
}

fn cal_address(email: &str) -> String {
    if email.contains(':') {
        email.to_owned()
    } else {
        format!("mailto:{email}")
    }
}

/// RFC 5545 §§3.2.4, 3.2.5 and 3.2.11 require each address in these list
/// parameters to be individually quoted.
fn quoted_address_list(addresses: &[String]) -> String {
    addresses
        .iter()
        .map(|address| format!("\"{}\"", cal_address(address)))
        .collect::<Vec<_>>()
        .join(",")
}

fn apply_common_address_params(
    property: &mut Property,
    name: Option<&str>,
    sent_by: Option<&str>,
    dir: Option<&str>,
    language: Option<&str>,
    parameters: &BTreeMap<String, String>,
) {
    if let Some(name) = name {
        property.append_parameter(Parameter::new("CN", name));
    }
    if let Some(sent_by) = sent_by {
        property.append_parameter(Parameter::new("SENT-BY", &cal_address(sent_by)));
    }
    if let Some(dir) = dir {
        property.append_parameter(Parameter::new("DIR", dir));
    }
    if let Some(language) = language {
        property.append_parameter(Parameter::new("LANGUAGE", language));
    }
    for (key, value) in parameters {
        property.append_parameter(Parameter::new(key, value));
    }
}

fn organizer_property(organizer: &Organizer) -> Property {
    let mut property = Property::new("ORGANIZER", cal_address(&organizer.email));
    apply_common_address_params(
        &mut property,
        organizer.name.as_deref(),
        organizer.sent_by.as_deref(),
        organizer.dir.as_deref(),
        organizer.language.as_deref(),
        &organizer.parameters,
    );
    property.done()
}

fn attendee_property(attendee: &Attendee) -> Property {
    let mut property = Property::new("ATTENDEE", cal_address(&attendee.email));
    if let Some(role) = &attendee.role {
        property.append_parameter(Parameter::new("ROLE", role.as_token()));
    }
    if let Some(partstat) = &attendee.partstat {
        property.append_parameter(Parameter::new("PARTSTAT", partstat.as_token()));
    }
    if let Some(cutype) = &attendee.cutype {
        property.append_parameter(Parameter::new("CUTYPE", cutype.as_token()));
    }
    if let Some(rsvp) = attendee.rsvp {
        property.append_parameter(Parameter::new("RSVP", if rsvp { "TRUE" } else { "FALSE" }));
    }
    if !attendee.members.is_empty() {
        property.append_parameter(Parameter::new(
            "MEMBER",
            &quoted_address_list(&attendee.members),
        ));
    }
    if !attendee.delegated_to.is_empty() {
        property.append_parameter(Parameter::new(
            "DELEGATED-TO",
            &quoted_address_list(&attendee.delegated_to),
        ));
    }
    if !attendee.delegated_from.is_empty() {
        property.append_parameter(Parameter::new(
            "DELEGATED-FROM",
            &quoted_address_list(&attendee.delegated_from),
        ));
    }
    apply_common_address_params(
        &mut property,
        attendee.name.as_deref(),
        attendee.sent_by.as_deref(),
        attendee.dir.as_deref(),
        attendee.language.as_deref(),
        &attendee.parameters,
    );
    property.done()
}

fn invalid_duration(value: &str, reason: &str) -> Error {
    Error::Serialize(format!("invalid duration {value:?}: {reason}"))
}

/// Consume one digit run, rejecting overflow instead of panicking on it.
fn read_digits(
    characters: &mut std::iter::Peekable<std::str::Chars<'_>>,
    value: &str,
) -> Result<()> {
    let mut seen = 0_usize;
    let mut magnitude = 0_u64;
    while let Some(digit) = characters
        .peek()
        .and_then(|character| character.to_digit(10))
    {
        characters.next();
        seen += 1;
        magnitude = magnitude
            .checked_mul(10)
            .and_then(|scaled| scaled.checked_add(u64::from(digit)))
            .ok_or_else(|| invalid_duration(value, "component is out of range"))?;
    }
    if seen == 0 {
        return Err(invalid_duration(value, "component has no digits"));
    }
    Ok(())
}

/// Validate an RFC 5545 duration (§3.3.6), including the leading sign used by
/// alarms that fire before their event.
///
/// The value is only checked, never rewritten: the original spelling is written
/// back verbatim so a round trip does not turn `-PT15M` into `-PT900S`.
///
/// This is hand-rolled rather than delegated to `iso8601::duration`, which
/// unwraps internally when a digit run overflows — a TRIGGER of
/// `-P5444444444444444444444444444D` aborted the process instead of being
/// rejected. Found by the `icalendar_parse` fuzz target.
fn validate_duration(value: &str) -> Result<()> {
    let rest = value
        .strip_prefix(['-', '+'])
        .unwrap_or(value)
        .strip_prefix('P')
        .ok_or_else(|| invalid_duration(value, "missing the P designator"))?;

    let mut characters = rest.chars().peekable();
    let mut in_time = false;
    let mut components = 0_usize;

    while let Some(&character) = characters.peek() {
        if character == 'T' {
            characters.next();
            if in_time {
                return Err(invalid_duration(value, "repeated T designator"));
            }
            in_time = true;
            continue;
        }

        read_digits(&mut characters, value)?;
        let unit = characters
            .next()
            .ok_or_else(|| invalid_duration(value, "component has no unit"))?;
        let expected = if in_time {
            matches!(unit, 'H' | 'M' | 'S')
        } else {
            matches!(unit, 'D' | 'W')
        };
        if !expected {
            return Err(invalid_duration(value, "unexpected unit designator"));
        }
        components += 1;
    }

    if components == 0 {
        return Err(invalid_duration(value, "no duration components"));
    }
    Ok(())
}

fn trigger_property(trigger: &AlarmTrigger) -> Result<Property> {
    let property = match trigger {
        AlarmTrigger::Duration { value, related } => {
            validate_duration(value)?;
            let mut property = Property::new("TRIGGER", value);
            match related {
                Some(TriggerRelation::Start) => {
                    property.append_parameter(Parameter::new("RELATED", "START"));
                }
                Some(TriggerRelation::End) => {
                    property.append_parameter(Parameter::new("RELATED", "END"));
                }
                None => {}
            }
            property.done()
        }
        AlarmTrigger::DateTime { timestamp } => {
            let mut property =
                Property::new("TRIGGER", timestamp.format("%Y%m%dT%H%M%SZ").to_string());
            property.append_parameter(Parameter::new("VALUE", "DATE-TIME"));
            property.done()
        }
    };
    Ok(property)
}

fn build_alarm(alarm: &Alarm) -> Result<IcsAlarm> {
    let trigger = alarm
        .trigger
        .as_ref()
        .ok_or_else(|| Error::Serialize("VALARM is missing a TRIGGER".to_owned()))?;

    // `audio` is the only public constructor that does not force a DESCRIPTION.
    // Both the ACTION and the placeholder TRIGGER it writes are replaced below,
    // which is what lets the exact trigger spelling survive.
    let mut ics_alarm = IcsAlarm::audio(Trigger::Duration(Duration::zero(), None));
    ics_alarm.append_property(trigger_property(trigger)?);
    if let Some(action) = &alarm.action {
        ics_alarm.append_property(Property::new("ACTION", action));
    }
    if let Some(description) = &alarm.description {
        ics_alarm.append_property(Property::new("DESCRIPTION", description));
    }
    if let Some(summary) = &alarm.summary {
        ics_alarm.append_property(Property::new("SUMMARY", summary));
    }
    if let Some(duration) = &alarm.duration {
        ics_alarm.append_property(Property::new("DURATION", duration));
    }
    if let Some(repeat) = alarm.repeat {
        ics_alarm.append_property(Property::new("REPEAT", repeat.to_string()));
    }
    for attendee in &alarm.attendees {
        ics_alarm.append_multi_property(attendee_property(attendee));
    }
    Ok(ics_alarm.done())
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
