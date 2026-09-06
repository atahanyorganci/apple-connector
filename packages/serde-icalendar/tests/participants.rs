//! Attendees, organizers and alarms survive a parse/serialize round trip.
//!
//! Fixtures mirror the shapes Apple Calendar, Google Calendar and Outlook emit.

use serde_icalendar::{
    Alarm, AlarmTrigger, Attendee, CalendarEvent, CalendarUserType, ParticipationStatus, Role,
    TriggerRelation, from_str, to_string,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn fixture(name: &str) -> Result<String, std::io::Error> {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    std::fs::read_to_string(path)
}

fn round_trip(event: &CalendarEvent) -> Result<CalendarEvent, serde_icalendar::Error> {
    let ics = to_string(event)?;
    from_str(&ics)
}

/// Undo RFC 5545 line folding so assertions can look for a whole value.
fn unfolded(ics: &str) -> String {
    ics.replace("\r\n ", "").replace("\n ", "")
}

#[test]
fn apple_invite_round_trips() -> TestResult {
    let event = from_str(&fixture("apple-invite.ics")?)?;

    let organizer = event.organizer.clone().ok_or("expected an organizer")?;
    assert_eq!(organizer.email, "organizer@example.com");
    assert_eq!(organizer.name.as_deref(), Some("Ada Lovelace"));

    assert_eq!(event.attendees.len(), 2);
    let first = &event.attendees[0];
    assert_eq!(first.email, "grace@example.com");
    assert_eq!(first.name.as_deref(), Some("Grace Hopper"));
    assert_eq!(first.role, Some(Role::ReqParticipant));
    assert_eq!(first.partstat, Some(ParticipationStatus::Accepted));
    assert_eq!(first.cutype, Some(CalendarUserType::Individual));
    assert_eq!(first.rsvp, Some(true));

    let second = &event.attendees[1];
    assert_eq!(second.role, Some(Role::OptParticipant));
    assert_eq!(second.partstat, Some(ParticipationStatus::NeedsAction));

    assert_eq!(event.alarms.len(), 1);
    let alarm = &event.alarms[0];
    assert_eq!(alarm.action.as_deref(), Some("DISPLAY"));
    assert_eq!(alarm.description.as_deref(), Some("Event reminder"));
    assert!(matches!(
        &alarm.trigger,
        Some(AlarmTrigger::Duration { value, related: None }) if value == "-PT15M"
    ));

    let reparsed = round_trip(&event)?;
    assert_eq!(reparsed.organizer, event.organizer);
    assert_eq!(reparsed.attendees, event.attendees);
    assert_eq!(reparsed.alarms, event.alarms);
    Ok(())
}

#[test]
fn google_invite_round_trips() -> TestResult {
    let event = from_str(&fixture("google-invite.ics")?)?;

    assert_eq!(event.attendees.len(), 2);
    assert_eq!(
        event.attendees[1].partstat,
        Some(ParticipationStatus::Declined)
    );
    assert_eq!(event.attendees[1].rsvp, Some(false));

    let alarm = event.alarms.first().ok_or("expected an alarm")?;
    assert_eq!(alarm.action.as_deref(), Some("EMAIL"));
    assert_eq!(alarm.summary.as_deref(), Some("Reminder"));
    assert_eq!(alarm.attendees.len(), 1);

    let reparsed = round_trip(&event)?;
    assert_eq!(reparsed.attendees, event.attendees);
    assert_eq!(reparsed.alarms, event.alarms);
    Ok(())
}

#[test]
fn outlook_invite_round_trips() -> TestResult {
    let event = from_str(&fixture("outlook-invite.ics")?)?;

    let chair = event
        .attendees
        .iter()
        .find(|attendee| attendee.role == Some(Role::Chair))
        .ok_or("expected a chair")?;
    assert_eq!(chair.cutype, Some(CalendarUserType::Individual));

    let room = event
        .attendees
        .iter()
        .find(|attendee| attendee.cutype == Some(CalendarUserType::Room))
        .ok_or("expected a room resource")?;
    assert_eq!(room.email, "room-42@example.com");

    let alarm = event.alarms.first().ok_or("expected an alarm")?;
    assert!(matches!(
        &alarm.trigger,
        Some(AlarmTrigger::Duration {
            related: Some(TriggerRelation::End),
            ..
        })
    ));
    assert_eq!(alarm.repeat, Some(2));
    assert_eq!(alarm.duration.as_deref(), Some("PT5M"));

    let reparsed = round_trip(&event)?;
    assert_eq!(reparsed.attendees, event.attendees);
    assert_eq!(reparsed.alarms, event.alarms);
    Ok(())
}

#[test]
fn role_and_partstat_use_rfc_tokens_not_debug_names() -> TestResult {
    let event = from_str(&fixture("apple-invite.ics")?)?;
    let ics = unfolded(&to_string(&event)?);

    assert!(ics.contains("ROLE=REQ-PARTICIPANT"), "{ics}");
    assert!(ics.contains("PARTSTAT=ACCEPTED"), "{ics}");
    assert!(ics.contains("PARTSTAT=NEEDS-ACTION"), "{ics}");
    // The old code wrote the Rust Debug spelling of the enum.
    assert!(!ics.contains("ReqParticipant"), "{ics}");
    assert!(!ics.contains("NeedsAction"), "{ics}");
    Ok(())
}

#[test]
fn organizer_cn_survives_a_round_trip() -> TestResult {
    let event = from_str(&fixture("apple-invite.ics")?)?;
    let ics = unfolded(&to_string(&event)?);
    assert!(ics.contains("CN=Ada Lovelace"), "{ics}");
    Ok(())
}

#[test]
fn unknown_tokens_and_parameters_are_preserved() -> TestResult {
    let event = from_str(&fixture("outlook-invite.ics")?)?;
    let attendee = event
        .attendees
        .iter()
        .find(|attendee| attendee.email == "vendor@example.com")
        .ok_or("expected the vendor attendee")?;

    assert_eq!(attendee.role, Some(Role::Other("X-VENDOR".to_owned())));
    assert_eq!(
        attendee.parameters.get("X-NUM-GUESTS").map(String::as_str),
        Some("3")
    );

    let ics = unfolded(&to_string(&event)?);
    assert!(ics.contains("ROLE=X-VENDOR"), "{ics}");
    assert!(ics.contains("X-NUM-GUESTS=3"), "{ics}");
    Ok(())
}

#[test]
fn delegation_and_membership_round_trip() -> TestResult {
    let event = from_str(&fixture("google-invite.ics")?)?;
    let delegate = event
        .attendees
        .iter()
        .find(|attendee| !attendee.delegated_from.is_empty())
        .ok_or("expected a delegated attendee")?;
    assert_eq!(delegate.delegated_from, vec!["boss@example.com".to_owned()]);

    let reparsed = round_trip(&event)?;
    assert_eq!(reparsed.attendees, event.attendees);
    Ok(())
}

#[test]
fn an_alarm_without_a_trigger_is_reported() -> TestResult {
    let event = CalendarEvent {
        uid: Some("no-trigger@example.com".to_owned()),
        alarms: vec![Alarm {
            action: Some("DISPLAY".to_owned()),
            description: Some("Nope".to_owned()),
            ..Alarm::default()
        }],
        ..CalendarEvent::default()
    };
    let error = to_string(&event)
        .err()
        .ok_or("expected a missing-trigger error")?;
    assert!(
        error.to_string().contains("TRIGGER"),
        "unexpected error: {error}"
    );
    Ok(())
}

#[test]
fn absolute_alarm_triggers_round_trip() -> TestResult {
    let event = from_str(&fixture("outlook-invite.ics")?)?;
    let absolute = event
        .alarms
        .iter()
        .find(|alarm| matches!(alarm.trigger, Some(AlarmTrigger::DateTime { .. })))
        .ok_or("expected an absolute trigger")?;

    let reparsed = round_trip(&event)?;
    assert!(reparsed.alarms.contains(absolute));
    Ok(())
}

#[test]
fn attendees_are_written_back_out() -> TestResult {
    // The serializer never emitted ATTENDEE at all, so anything parsed in was
    // lost on the way out.
    let event = CalendarEvent {
        uid: Some("attendees@example.com".to_owned()),
        attendees: vec![Attendee {
            email: "someone@example.com".to_owned(),
            name: Some("Some One".to_owned()),
            role: Some(Role::Chair),
            partstat: Some(ParticipationStatus::Tentative),
            ..Attendee::default()
        }],
        ..CalendarEvent::default()
    };
    let ics = unfolded(&to_string(&event)?);
    assert!(ics.contains("ATTENDEE"), "{ics}");
    assert!(ics.contains("mailto:someone@example.com"), "{ics}");

    let reparsed = from_str(&ics)?;
    assert_eq!(reparsed.attendees, event.attendees);
    Ok(())
}
