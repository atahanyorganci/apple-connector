use std::collections::BTreeMap;

use chrono::{DateTime, NaiveDate, NaiveDateTime, TimeZone, Utc};
use icalendar::{
    Calendar, CalendarDateTime, Component, DatePerhapsTime, EventLike, Parameter, Property,
};

use crate::{
    error::{Error, Result},
    model::{
        Alarm, AlarmTrigger, Attendee, CalendarEvent, CalendarUserType, EventDateTime, EventStatus,
        ExtensionBag, Organizer, ParticipationStatus, Role, TriggerRelation,
    },
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
        start: ics_event
            .get_start()
            .map(date_perhaps_time_to_event)
            .transpose()?,
        end: ics_event
            .get_end()
            .map(date_perhaps_time_to_event)
            .transpose()?,
        organizer: ics_event.properties().get("ORGANIZER").map(parse_organizer),
        attendees: parse_attendees(ics_event.multi_properties().get("ATTENDEE")),
        alarms: parse_alarms(ics_event)?,
        recurrence_rule: ics_event.property_value("RRULE").map(str::to_owned),
        exception_dates: parse_exception_dates(ics_event)?,
        sequence: ics_event.get_sequence(),
        extensions: Some(parse_extensions(ics_event)),
    })
}

fn date_perhaps_time_to_event(dt: DatePerhapsTime) -> Result<EventDateTime> {
    match dt {
        DatePerhapsTime::Date(date) => Ok(EventDateTime::date(date)),
        DatePerhapsTime::DateTime(CalendarDateTime::Utc(value)) => Ok(EventDateTime::utc(value)),
        DatePerhapsTime::DateTime(CalendarDateTime::Floating(value)) => {
            Ok(EventDateTime::floating(value))
        }
        // A wall-clock time in a named zone is not a UTC instant. Reading it as
        // one shifts the event by the zone's offset.
        DatePerhapsTime::DateTime(CalendarDateTime::WithTimezone { date_time, tzid }) => {
            let timestamp = resolve_zoned(date_time, &tzid)?;
            Ok(EventDateTime::zoned(timestamp, tzid))
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
        return Ok(EventDateTime::date(date));
    }

    // UTC form: 20240101T120000Z. The trailing Z wins over any TZID parameter.
    if let Some(without_zulu) = value.strip_suffix('Z') {
        let naive = parse_ics_naive(without_zulu, value)?;
        return Ok(EventDateTime::utc(Utc.from_utc_datetime(&naive)));
    }

    // Zoned or floating form: 20240101T120000
    let naive = parse_ics_naive(value, value)?;
    match tzid {
        Some(zone) => {
            let timestamp = resolve_zoned(naive, &zone)?;
            Ok(EventDateTime::zoned(timestamp, zone))
        }
        None => Ok(EventDateTime::floating(naive)),
    }
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

/// Parameters handled by a dedicated model field, so the catch-all bag does not
/// duplicate them.
const KNOWN_ADDRESS_PARAMS: &[&str] = &[
    "CN",
    "CUTYPE",
    "DELEGATED-FROM",
    "DELEGATED-TO",
    "DIR",
    "LANGUAGE",
    "MEMBER",
    "PARTSTAT",
    "ROLE",
    "RSVP",
    "SENT-BY",
];

fn strip_mailto(value: &str) -> &str {
    value
        .strip_prefix("mailto:")
        .or_else(|| value.strip_prefix("MAILTO:"))
        .unwrap_or(value)
}

fn param<'a>(property: &'a Property, key: &str) -> Option<&'a str> {
    property.params().get(key).map(Parameter::value)
}

/// Parameters we do not model, kept verbatim so a round trip loses nothing.
fn unmodelled_params(property: &Property) -> BTreeMap<String, String> {
    property
        .params()
        .iter()
        .filter(|(key, _)| !KNOWN_ADDRESS_PARAMS.contains(&key.as_str()))
        .map(|(key, value)| (key.clone(), value.value().to_owned()))
        .collect()
}

/// RFC 5545 requires each address in MEMBER, DELEGATED-FROM and DELEGATED-TO to
/// be a quoted string in a comma-separated list.
fn address_list(property: &Property, key: &str) -> Vec<String> {
    param(property, key)
        .into_iter()
        .flat_map(|raw| raw.split(','))
        .map(|entry| strip_mailto(entry.trim().trim_matches('"')).to_owned())
        .filter(|entry| !entry.is_empty())
        .collect()
}

fn parse_organizer(property: &Property) -> Organizer {
    Organizer {
        email: strip_mailto(property.value()).to_owned(),
        name: param(property, "CN").map(str::to_owned),
        sent_by: param(property, "SENT-BY")
            .map(strip_mailto)
            .map(str::to_owned),
        dir: param(property, "DIR").map(str::to_owned),
        language: param(property, "LANGUAGE").map(str::to_owned),
        parameters: unmodelled_params(property),
    }
}

fn parse_attendees(properties: Option<&Vec<Property>>) -> Vec<Attendee> {
    properties
        .map(|properties| properties.iter().map(parse_attendee).collect())
        .unwrap_or_default()
}

fn parse_attendee(property: &Property) -> Attendee {
    Attendee {
        email: strip_mailto(property.value()).to_owned(),
        name: param(property, "CN").map(str::to_owned),
        role: param(property, "ROLE").map(Role::from_token),
        partstat: param(property, "PARTSTAT").map(ParticipationStatus::from_token),
        cutype: param(property, "CUTYPE").map(CalendarUserType::from_token),
        rsvp: param(property, "RSVP").map(|value| value.eq_ignore_ascii_case("TRUE")),
        delegated_from: address_list(property, "DELEGATED-FROM"),
        delegated_to: address_list(property, "DELEGATED-TO"),
        members: address_list(property, "MEMBER"),
        sent_by: param(property, "SENT-BY")
            .map(strip_mailto)
            .map(str::to_owned),
        dir: param(property, "DIR").map(str::to_owned),
        language: param(property, "LANGUAGE").map(str::to_owned),
        parameters: unmodelled_params(property),
    }
}

/// VALARM components arrive as nested `Other` components rather than as
/// properties, which is why they were previously dropped wholesale.
fn parse_alarms(event: &icalendar::Event) -> Result<Vec<Alarm>> {
    let mut alarms = Vec::new();
    for component in event.components() {
        if !component.component_kind().eq_ignore_ascii_case("VALARM") {
            continue;
        }
        let properties = component.properties();
        let repeat = match properties.get("REPEAT") {
            Some(property) => Some(property.value().parse::<u32>().map_err(|error| {
                Error::Parse(format!(
                    "invalid VALARM REPEAT {:?}: {error}",
                    property.value()
                ))
            })?),
            None => None,
        };

        alarms.push(Alarm {
            action: properties
                .get("ACTION")
                .map(|property| property.value().to_owned()),
            trigger: properties.get("TRIGGER").map(parse_trigger).transpose()?,
            description: properties
                .get("DESCRIPTION")
                .map(|property| property.value().to_owned()),
            summary: properties
                .get("SUMMARY")
                .map(|property| property.value().to_owned()),
            duration: properties
                .get("DURATION")
                .map(|property| property.value().to_owned()),
            repeat,
            attendees: parse_attendees(component.multi_properties().get("ATTENDEE")),
        });
    }
    Ok(alarms)
}

fn parse_trigger(property: &Property) -> Result<AlarmTrigger> {
    if param(property, "VALUE").is_some_and(|value| value.eq_ignore_ascii_case("DATE-TIME")) {
        let value = property.value();
        let naive = NaiveDateTime::parse_from_str(value.trim_end_matches('Z'), "%Y%m%dT%H%M%S")
            .map_err(|error| Error::Parse(format!("invalid VALARM TRIGGER {value:?}: {error}")))?;
        return Ok(AlarmTrigger::DateTime {
            timestamp: Utc.from_utc_datetime(&naive),
        });
    }

    let related = match param(property, "RELATED") {
        Some(value) if value.eq_ignore_ascii_case("END") => Some(TriggerRelation::End),
        Some(value) if value.eq_ignore_ascii_case("START") => Some(TriggerRelation::Start),
        Some(value) => {
            return Err(Error::Parse(format!(
                "invalid VALARM TRIGGER RELATED {value:?}"
            )));
        }
        None => None,
    };
    Ok(AlarmTrigger::Duration {
        value: property.value().to_owned(),
        related,
    })
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
