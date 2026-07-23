use super::{
    pagination::PageMetaDto,
    reminder::{
        AlarmDto, AlarmKindDto, RecurrenceDto, ReminderAttachmentDetailDto,
        ReminderAttachmentKindDto, ReminderAttachmentSummaryDto, ReminderDetailDto,
        ReminderListDetailDto, ReminderListKindDto, ReminderListPageDto, ReminderListSummaryDto,
        ReminderPageDto, ReminderSummaryDto, SectionSummaryDto, SmartFilterDto, due_to_dto,
    },
};
use crate::reminders::{
    Alarm, AlarmKind, AttachmentKind, ListKind, Reminder, ReminderAttachment, ReminderList,
    ReminderSummary, Section, SmartFilter,
};

pub fn reminder_list_page_to_dto(
    items: Vec<ReminderList>,
    has_more: bool,
    next_cursor: Option<String>,
    limit: u32,
) -> ReminderListPageDto {
    ReminderListPageDto {
        items: items.iter().map(reminder_list_summary_to_dto).collect(),
        page: PageMetaDto {
            limit,
            has_more,
            next_cursor,
        },
    }
}

pub fn reminder_page_to_dto(
    items: Vec<ReminderSummary>,
    has_more: bool,
    next_cursor: Option<String>,
    limit: u32,
) -> ReminderPageDto {
    ReminderPageDto {
        items: items.iter().map(reminder_summary_to_dto).collect(),
        page: PageMetaDto {
            limit,
            has_more,
            next_cursor,
        },
    }
}

pub fn reminder_list_summary_to_dto(list: &ReminderList) -> ReminderListSummaryDto {
    ReminderListSummaryDto {
        id: list.id.clone(),
        row_id: list.row_id,
        name: list.name.clone(),
        kind: list_kind_to_dto(list.kind.clone()),
        smart_list_type: list.smart_list_type.clone(),
    }
}

pub fn reminder_list_detail_to_dto(list: &ReminderList) -> ReminderListDetailDto {
    ReminderListDetailDto {
        id: list.id.clone(),
        row_id: list.row_id,
        name: list.name.clone(),
        kind: list_kind_to_dto(list.kind.clone()),
        smart_list_type: list.smart_list_type.clone(),
        sharing_status: list.sharing_status,
        shared_owner_name: list.shared_owner_name.clone(),
        shared_owner_address: list.shared_owner_address.clone(),
        filter: smart_filter_to_dto(&crate::reminders::assembly::smart_filter_from_data(
            list.filter_data.as_deref(),
        )),
        sections: list.sections.iter().map(section_to_dto).collect(),
    }
}

pub fn reminder_summary_to_dto(reminder: &ReminderSummary) -> ReminderSummaryDto {
    ReminderSummaryDto {
        id: reminder.id.clone(),
        row_id: reminder.row_id,
        title: reminder.title.clone(),
        completed: reminder.completed,
        flagged: reminder.flagged,
        priority: priority_to_raw(&reminder.priority),
        list_id: reminder.list_id.clone(),
        list_row_id: reminder.list_row_id,
        list_name: reminder.list_name.clone(),
        parent_id: reminder.parent_id.clone(),
        section_id: reminder.section_id.clone(),
        due: reminder.due.as_ref().map(due_to_dto),
        last_modified_at: reminder
            .last_modified_at
            .map(super::common::timestamp_to_rfc3339),
        tags: reminder.tags.clone(),
    }
}

pub fn reminder_detail_to_dto(reminder: &Reminder) -> ReminderDetailDto {
    ReminderDetailDto {
        id: reminder.id.clone(),
        row_id: reminder.row_id,
        title: reminder.title.clone(),
        notes: reminder.notes.clone(),
        completed: reminder.completed,
        flagged: reminder.flagged,
        priority: priority_to_raw(&reminder.priority),
        list_id: reminder.list_id.clone(),
        list_row_id: reminder.list_row_id,
        list_name: reminder.list_name.clone(),
        parent_id: reminder.parent_id.clone(),
        section_id: reminder.section_id.clone(),
        due: reminder.due.as_ref().map(due_to_dto),
        completion_at: reminder
            .completion_at
            .map(super::common::timestamp_to_rfc3339),
        created_at: reminder.created_at.map(super::common::timestamp_to_rfc3339),
        last_modified_at: reminder
            .last_modified_at
            .map(super::common::timestamp_to_rfc3339),
        subtasks: reminder
            .subtasks
            .iter()
            .map(reminder_summary_to_dto)
            .collect(),
        tags: reminder.tags.clone(),
        alarms: reminder.alarms.iter().map(alarm_to_dto).collect(),
        recurrence: reminder.recurrence.as_ref().map(recurrence_to_dto),
        attachments: reminder
            .attachments
            .iter()
            .map(reminder_attachment_summary_to_dto)
            .collect(),
    }
}

pub fn reminder_attachment_detail_to_dto(
    attachment: &ReminderAttachment,
    reminder_id: String,
) -> ReminderAttachmentDetailDto {
    ReminderAttachmentDetailDto {
        id: attachment.id.clone(),
        row_id: attachment.row_id,
        filename: attachment.filename.clone(),
        uti: attachment.uti.clone(),
        sha512: attachment.sha512.clone(),
        kind: attachment_kind_to_dto(&attachment.kind),
        reminder_id,
        modified_at: attachment
            .modified_at
            .map(super::common::timestamp_to_rfc3339),
    }
}

fn section_to_dto(section: &Section) -> SectionSummaryDto {
    SectionSummaryDto {
        id: section.id.clone(),
        display_name: section.display_name.clone(),
        canonical_name: section.canonical_name.clone(),
    }
}

fn list_kind_to_dto(kind: ListKind) -> ReminderListKindDto {
    match kind {
        ListKind::Standard => ReminderListKindDto::Standard,
        ListKind::Smart => ReminderListKindDto::Smart,
    }
}

fn smart_filter_to_dto(filter: &SmartFilter) -> SmartFilterDto {
    SmartFilterDto {
        decoded: filter.decoded,
        raw: filter.raw.clone(),
    }
}

fn priority_to_raw(priority: &crate::reminders::Priority) -> i64 {
    match priority {
        crate::reminders::Priority::None => 0,
        crate::reminders::Priority::Low => 9,
        crate::reminders::Priority::Medium => 5,
        crate::reminders::Priority::High => 1,
    }
}

fn alarm_to_dto(alarm: &Alarm) -> AlarmDto {
    AlarmDto {
        kind: alarm_kind_to_dto(&alarm.kind),
        title: alarm.title.clone(),
        latitude: alarm.latitude,
        longitude: alarm.longitude,
        radius: alarm.radius,
        time_interval: alarm.time_interval,
        decode_error: alarm.decode_error.clone(),
    }
}

fn alarm_kind_to_dto(kind: &AlarmKind) -> AlarmKindDto {
    match kind {
        AlarmKind::Absolute => AlarmKindDto::Absolute,
        AlarmKind::Relative => AlarmKindDto::Relative,
        AlarmKind::Location => AlarmKindDto::Location,
        AlarmKind::Unknown => AlarmKindDto::Unknown,
    }
}

fn recurrence_to_dto(rule: &crate::reminders::RecurrenceRule) -> RecurrenceDto {
    RecurrenceDto {
        frequency: rule.frequency,
        interval: rule.interval,
        occurrence_count: rule.occurrence_count,
        decode_error: rule.decode_error.clone(),
    }
}

fn reminder_attachment_summary_to_dto(
    attachment: &ReminderAttachment,
) -> ReminderAttachmentSummaryDto {
    ReminderAttachmentSummaryDto {
        id: attachment.id.clone(),
        row_id: attachment.row_id,
        filename: attachment.filename.clone(),
        uti: attachment.uti.clone(),
        sha512: attachment.sha512.clone(),
        kind: attachment_kind_to_dto(&attachment.kind),
    }
}

fn attachment_kind_to_dto(kind: &AttachmentKind) -> ReminderAttachmentKindDto {
    match kind {
        AttachmentKind::File => ReminderAttachmentKindDto::File,
        AttachmentKind::Image => ReminderAttachmentKindDto::Image,
        AttachmentKind::Audio => ReminderAttachmentKindDto::Audio,
        AttachmentKind::Unknown(_) => ReminderAttachmentKindDto::Unknown,
    }
}
