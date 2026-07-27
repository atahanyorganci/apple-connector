use super::{
    calendar::{
        AvailabilityDto, CalendarAccountDto, CalendarAccountPageDto, CalendarDetailDto,
        CalendarPageDto, CalendarSummaryDto, EventAlarmDto, EventAttachmentDetailDto,
        EventAttachmentSummaryDto, EventClassDto, EventDetailDto, EventLocationDto, EventPageDto,
        EventParticipantDto, EventStatusDto, EventSummaryDto, InvitationStatusDto, PrivacyLevelDto,
        RecurrenceRuleDto, StoreTypeDto,
    },
    common::timestamp_to_unix,
    pagination::PageMetaDto,
};
use crate::{
    apple_types::{CalendarAccountId, CalendarAttachmentId, CalendarId, EventId},
    calendar::{
        CalendarAccount, CalendarDetail, CalendarSummary, EventAttachment, EventDetail,
        EventLocation, EventParticipant, EventSummary, RecurrenceRule,
        enums::{Availability, EventClass, EventStatus, InvitationStatus, PrivacyLevel, StoreType},
        model::EventAlarm,
    },
};

pub fn calendar_account_page_to_dto(items: Vec<CalendarAccount>) -> CalendarAccountPageDto {
    CalendarAccountPageDto {
        items: items.iter().map(calendar_account_to_dto).collect(),
    }
}

pub fn calendar_account_to_dto(account: &CalendarAccount) -> CalendarAccountDto {
    CalendarAccountDto {
        id: CalendarAccountId::new(account.id.clone()),
        row_id: account.row_id,
        name: account.name.clone(),
        store_type: store_type_to_dto(account.store_type),
        disabled: account.disabled,
    }
}

pub fn calendar_page_to_dto(
    items: Vec<CalendarSummary>,
    has_more: bool,
    next_cursor: Option<String>,
    limit: u32,
) -> CalendarPageDto {
    CalendarPageDto {
        items: items.iter().map(calendar_summary_to_dto).collect(),
        page: PageMetaDto {
            limit,
            has_more,
            next_cursor,
        },
    }
}

pub fn calendar_summary_to_dto(calendar: &CalendarSummary) -> CalendarSummaryDto {
    CalendarSummaryDto {
        id: CalendarId::new(calendar.id.clone()),
        row_id: calendar.row_id,
        title: calendar.title.clone(),
        color: calendar.color.clone(),
        account_id: CalendarAccountId::new(calendar.account_id.clone()),
        account_row_id: calendar.account_row_id,
    }
}

pub fn calendar_detail_to_dto(calendar: &CalendarDetail) -> CalendarDetailDto {
    CalendarDetailDto {
        summary: calendar_summary_to_dto(&calendar.summary),
        notes: calendar.notes.clone(),
        sharing_status: calendar.sharing_status,
    }
}

pub fn event_page_to_dto(
    items: Vec<EventSummary>,
    has_more: bool,
    next_cursor: Option<String>,
    limit: u32,
) -> EventPageDto {
    EventPageDto {
        items: items.iter().map(event_summary_to_dto).collect(),
        page: PageMetaDto {
            limit,
            has_more,
            next_cursor,
        },
    }
}

pub fn event_summary_to_dto(event: &EventSummary) -> EventSummaryDto {
    EventSummaryDto {
        id: EventId::new(event.id.clone()),
        row_id: event.row_id,
        calendar_id: CalendarId::new(event.calendar_id.clone()),
        calendar_row_id: event.calendar_row_id,
        summary: event.summary.clone(),
        start: event.start.map(timestamp_to_unix),
        end: event.end.map(timestamp_to_unix),
        all_day: event.all_day,
        status: event_status_to_dto(event.status),
        hidden: event.hidden,
        is_recurring: event.is_recurring,
        occurrence_start: event.occurrence_start.map(timestamp_to_unix),
        occurrence_end: event.occurrence_end.map(timestamp_to_unix),
        event_class: event_class_to_dto(event.event_class),
    }
}

pub fn event_detail_to_dto(event: &EventDetail) -> EventDetailDto {
    EventDetailDto {
        summary: event_summary_to_dto(&event.summary),
        description: event.description.clone(),
        url: event.url.clone(),
        location: event.location.as_ref().map(location_to_dto),
        organizer: event.organizer.as_ref().map(participant_to_dto),
        attendees: event.attendees.iter().map(participant_to_dto).collect(),
        recurrence: event.recurrence.as_ref().map(recurrence_to_dto),
        exception_dates: event
            .exception_dates
            .iter()
            .copied()
            .map(timestamp_to_unix)
            .collect(),
        alarms: event.alarms.iter().map(alarm_to_dto).collect(),
        attachments: event
            .attachments
            .iter()
            .map(attachment_summary_to_dto)
            .collect(),
        conference_url: event.conference_url.clone(),
        travel_time_seconds: event.travel_time_seconds,
        invitation_status: invitation_status_to_dto(event.invitation_status),
        availability: availability_to_dto(event.availability),
        privacy_level: privacy_level_to_dto(event.privacy_level),
        series_id: event.series_id.as_ref().map(|id| EventId::new(id.clone())),
        series_row_id: event.series_row_id,
        original_start: event.original_start.map(timestamp_to_unix),
        last_modified: event.last_modified.map(timestamp_to_unix),
        creation_date: event.creation_date.map(timestamp_to_unix),
        has_structured_data: event.structured_data.is_some(),
        has_app_link: event.app_link.is_some(),
    }
}

pub fn attachment_detail_to_dto(attachment: &EventAttachment) -> EventAttachmentDetailDto {
    EventAttachmentDetailDto {
        summary: attachment_summary_to_dto(attachment),
        local_path: attachment.local_path.clone(),
    }
}

fn attachment_summary_to_dto(attachment: &EventAttachment) -> EventAttachmentSummaryDto {
    EventAttachmentSummaryDto {
        id: CalendarAttachmentId::new(attachment.id.clone()),
        row_id: attachment.row_id,
        filename: attachment.filename.clone(),
        format: attachment.format.clone(),
        file_size: attachment.file_size,
    }
}

fn location_to_dto(location: &EventLocation) -> EventLocationDto {
    EventLocationDto {
        title: location.title.clone(),
        address: location.address.clone(),
        latitude: location.latitude,
        longitude: location.longitude,
    }
}

fn participant_to_dto(participant: &EventParticipant) -> EventParticipantDto {
    EventParticipantDto {
        id: participant.id.clone(),
        email: participant.email.clone(),
        phone_number: participant.phone_number.clone(),
        name: participant.name.clone(),
        is_self: participant.is_self,
        status: invitation_status_to_dto(participant.status),
        role: participant.role,
        comment: participant.comment.clone(),
    }
}

fn recurrence_to_dto(recurrence: &RecurrenceRule) -> RecurrenceRuleDto {
    RecurrenceRuleDto {
        frequency: recurrence.frequency,
        interval: recurrence.interval,
        count: recurrence.count,
        end_date: recurrence.end_date.map(timestamp_to_unix),
        specifier: recurrence.specifier.clone(),
        raw_specifier: recurrence.raw_specifier.clone(),
    }
}

fn alarm_to_dto(alarm: &EventAlarm) -> EventAlarmDto {
    EventAlarmDto {
        id: alarm.id.clone(),
        trigger_interval_seconds: alarm.trigger_interval_seconds,
        trigger_date: alarm.trigger_date.map(timestamp_to_unix),
        alarm_type: alarm.alarm_type,
        disabled: alarm.disabled,
    }
}

fn store_type_to_dto(store_type: StoreType) -> StoreTypeDto {
    match store_type {
        StoreType::Local => StoreTypeDto::Local,
        StoreType::CalDav => StoreTypeDto::CalDav,
        StoreType::Exchange => StoreTypeDto::Exchange,
        StoreType::Subscription => StoreTypeDto::Subscription,
        StoreType::Birthday => StoreTypeDto::Birthday,
    }
}

fn event_status_to_dto(status: EventStatus) -> EventStatusDto {
    match status {
        EventStatus::Confirmed => EventStatusDto::Confirmed,
        EventStatus::Tentative => EventStatusDto::Tentative,
        EventStatus::Cancelled => EventStatusDto::Cancelled,
    }
}

fn invitation_status_to_dto(status: InvitationStatus) -> InvitationStatusDto {
    match status {
        InvitationStatus::Unknown => InvitationStatusDto::Unknown,
        InvitationStatus::Accepted => InvitationStatusDto::Accepted,
        InvitationStatus::Declined => InvitationStatusDto::Declined,
        InvitationStatus::Tentative => InvitationStatusDto::Tentative,
        InvitationStatus::NeedsAction => InvitationStatusDto::NeedsAction,
    }
}

fn availability_to_dto(availability: Availability) -> AvailabilityDto {
    match availability {
        Availability::Busy => AvailabilityDto::Busy,
        Availability::Free => AvailabilityDto::Free,
        Availability::Tentative => AvailabilityDto::Tentative,
        Availability::Unavailable => AvailabilityDto::Unavailable,
    }
}

fn privacy_level_to_dto(level: PrivacyLevel) -> PrivacyLevelDto {
    match level {
        PrivacyLevel::Default => PrivacyLevelDto::Default,
        PrivacyLevel::Public => PrivacyLevelDto::Public,
        PrivacyLevel::Private => PrivacyLevelDto::Private,
    }
}

fn event_class_to_dto(class: EventClass) -> EventClassDto {
    match class {
        EventClass::Standard => EventClassDto::Standard,
        EventClass::Birthday => EventClassDto::Birthday,
        EventClass::SpecialDay => EventClassDto::SpecialDay,
    }
}
