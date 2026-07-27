use super::{
    enums::{Availability, EventClass, EventStatus, InvitationStatus, PrivacyLevel, StoreType},
    model::{
        CalendarAccount, CalendarDetail, CalendarSummary, EventAlarm, EventAttachment, EventDetail,
        EventLocation, EventParticipant, EventSummary, RecurrenceRule,
    },
    row::{
        AlarmRow, AttachmentRow, CalendarRow, EventRow, LocationRow, ParticipantRow, RecurrenceRow,
        StoreRow, microdegrees_to_degrees, parse_core_data_timestamp,
    },
};

pub fn account_from_row(row: StoreRow) -> CalendarAccount {
    CalendarAccount {
        row_id: row.row_id,
        id: row
            .external_id
            .unwrap_or_else(|| format!("store-{}", row.row_id)),
        name: row.name,
        store_type: StoreType::from_raw(row.store_type),
        disabled: row.disabled.is_some_and(|v| v != 0),
    }
}

pub fn calendar_summary_from_row(row: CalendarRow) -> CalendarSummary {
    CalendarSummary {
        row_id: row.row_id,
        id: row.id,
        title: row.title,
        color: row.color,
        account_row_id: row.store_id,
        account_id: row.account_id,
    }
}

pub fn calendar_detail_from_row(row: CalendarRow) -> CalendarDetail {
    CalendarDetail {
        summary: calendar_summary_from_row(row.clone()),
        notes: row.notes,
        sharing_status: row.sharing_status,
    }
}

pub fn event_summary_from_row(row: EventRow) -> EventSummary {
    let start = parse_core_data_timestamp(row.occurrence_start.or(row.start_date));
    let end = parse_core_data_timestamp(row.occurrence_end.or(row.end_date));
    EventSummary {
        row_id: row.row_id,
        id: row.id,
        calendar_row_id: row.calendar_row_id,
        calendar_id: row.calendar_id,
        summary: row.summary,
        start: parse_core_data_timestamp(row.start_date),
        end: parse_core_data_timestamp(row.end_date),
        all_day: row.all_day.is_some_and(|v| v != 0),
        status: EventStatus::from_raw(row.status),
        hidden: row.hidden.is_some_and(|v| v != 0),
        is_recurring: row.has_recurrences.is_some_and(|v| v != 0),
        occurrence_start: start,
        occurrence_end: end,
        event_class: EventClass::from_row(
            row.entity_type,
            row.birthday_id,
            row.special_day.as_deref(),
        ),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn event_detail_from_row(
    row: EventRow,
    location: Option<LocationRow>,
    organizer: Option<ParticipantRow>,
    attendees: Vec<ParticipantRow>,
    recurrence: Option<RecurrenceRow>,
    exception_dates: Vec<f64>,
    alarms: Vec<AlarmRow>,
    attachments: Vec<AttachmentRow>,
) -> EventDetail {
    let summary = event_summary_from_row(row.clone());
    EventDetail {
        summary,
        description: row.description,
        url: row.url,
        location: location.map(location_from_row),
        organizer: organizer.map(participant_from_row),
        attendees: attendees.into_iter().map(participant_from_row).collect(),
        recurrence: recurrence.map(recurrence_from_row),
        exception_dates: exception_dates
            .into_iter()
            .filter_map(|date| parse_core_data_timestamp(Some(date)))
            .collect(),
        alarms: alarms.into_iter().map(alarm_from_row).collect(),
        attachments: attachments.into_iter().map(attachment_from_row).collect(),
        conference_url: row.conference_url,
        travel_time_seconds: row.travel_time,
        invitation_status: InvitationStatus::from_raw(row.invitation_status),
        availability: Availability::from_raw(row.availability),
        privacy_level: PrivacyLevel::from_raw(row.privacy_level),
        series_id: row.series_id,
        series_row_id: row.orig_item_id.filter(|id| *id > 0),
        original_start: parse_core_data_timestamp(row.orig_date),
        last_modified: parse_core_data_timestamp(row.last_modified),
        creation_date: parse_core_data_timestamp(row.creation_date),
        structured_data: row.structured_data,
        app_link: row.app_link,
    }
}

fn location_from_row(row: LocationRow) -> EventLocation {
    EventLocation {
        title: row.title,
        address: row.address,
        latitude: microdegrees_to_degrees(row.latitude),
        longitude: microdegrees_to_degrees(row.longitude),
    }
}

fn participant_from_row(row: ParticipantRow) -> EventParticipant {
    let email = row.email.clone();
    EventParticipant {
        id: row.id,
        email: row.email,
        phone_number: row.phone_number,
        name: email,
        is_self: row.is_self.is_some_and(|v| v != 0),
        status: InvitationStatus::from_raw(row.status),
        role: row.role,
        comment: row.comment,
    }
}

fn recurrence_from_row(row: RecurrenceRow) -> RecurrenceRule {
    RecurrenceRule {
        frequency: row.frequency.unwrap_or(0),
        interval: row.interval.unwrap_or(1),
        count: row.count.filter(|c| *c > 0),
        end_date: parse_core_data_timestamp(row.end_date),
        specifier: row.specifier.clone(),
        raw_specifier: row.specifier.unwrap_or_default(),
    }
}

fn alarm_from_row(row: AlarmRow) -> EventAlarm {
    EventAlarm {
        id: row.id,
        trigger_interval_seconds: row.trigger_interval,
        trigger_date: parse_core_data_timestamp(row.trigger_date),
        alarm_type: row.alarm_type.unwrap_or(0),
        disabled: row.disabled.is_some_and(|v| v != 0),
    }
}

pub fn attachment_from_row(row: AttachmentRow) -> EventAttachment {
    EventAttachment {
        row_id: row.row_id,
        id: row.id,
        filename: row.filename,
        format: row.format,
        file_size: row.file_size,
        local_path: row.local_path,
    }
}
