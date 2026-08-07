use std::collections::HashMap;

use super::{
    entities::{EntityIds, is_alarm_ent},
    model::{
        Alarm, AlarmKind, AttachmentKind, Due, ListKind, RecurrenceRule, Reminder,
        ReminderAttachment, ReminderList, ReminderSummary, Section, SmartFilter,
    },
    row::{AttachmentRow, ListRow, ObjectRow, ReminderRow, parse_core_data_timestamp},
    sections::{decode_smart_filter, parse_section_memberships},
};
use crate::apple_types::{
    ReminderAttachmentId, ReminderId, ReminderListId, ReminderPriority, RowId, SectionId,
};

pub fn list_kind_from_ent(
    ent: i64,
    smart_list_type: Option<&str>,
    smart_list_ent: i64,
) -> ListKind {
    if ent == smart_list_ent || smart_list_type.is_some() {
        ListKind::Smart
    } else {
        ListKind::Standard
    }
}

pub fn list_from_row(row: ListRow, sections: Vec<Section>, smart_list_ent: i64) -> ReminderList {
    ReminderList {
        row_id: RowId::new(row.row_id),
        id: ReminderListId::new(row.id),
        name: row.name,
        kind: list_kind_from_ent(row.ent, row.smart_list_type.as_deref(), smart_list_ent),
        smart_list_type: row.smart_list_type,
        sharing_status: row.sharing_status,
        shared_owner_name: row.shared_owner_name,
        shared_owner_address: row.shared_owner_address,
        filter_data: row.filter_data,
        membership_data: row.membership_data,
        sections,
        last_modified_at: parse_core_data_timestamp(row.last_modified_date),
    }
}

pub fn list_summary_from_row(row: ListRow, smart_list_ent: i64) -> ReminderList {
    list_from_row(row, Vec::new(), smart_list_ent)
}

pub fn reminder_summary_from_row(
    row: ReminderRow,
    section_id: Option<SectionId>,
    tags: Vec<String>,
) -> ReminderSummary {
    let due = due_from_row(&row);
    ReminderSummary {
        row_id: RowId::new(row.row_id),
        id: ReminderId::new(row.id),
        title: row.title,
        completed: row.completed,
        flagged: row.flagged,
        priority: ReminderPriority::try_new(row.priority)
            .unwrap_or_else(|_| ReminderPriority::none()),
        list_row_id: RowId::new(row.list_row_id),
        list_id: ReminderListId::new(row.list_id),
        list_name: row.list_name.unwrap_or_else(|| "Untitled".to_owned()),
        parent_id: row.parent_id.map(ReminderId::new),
        section_id,
        due,
        last_modified_at: parse_core_data_timestamp(row.last_modified_date),
        tags,
    }
}

pub fn reminder_from_row(
    row: ReminderRow,
    section_id: Option<SectionId>,
    subtasks: Vec<ReminderSummary>,
    tags: Vec<String>,
    alarms: Vec<Alarm>,
    recurrence: Option<RecurrenceRule>,
    attachments: Vec<ReminderAttachment>,
) -> Reminder {
    let due = due_from_row(&row);
    Reminder {
        row_id: RowId::new(row.row_id),
        id: ReminderId::new(row.id),
        title: row.title,
        notes: row.notes,
        completed: row.completed,
        flagged: row.flagged,
        priority: ReminderPriority::try_new(row.priority)
            .unwrap_or_else(|_| ReminderPriority::none()),
        list_row_id: RowId::new(row.list_row_id),
        list_id: ReminderListId::new(row.list_id),
        list_name: row.list_name.unwrap_or_else(|| "Untitled".to_owned()),
        parent_row_id: row.parent_row_id.map(RowId::new),
        parent_id: row.parent_id.map(ReminderId::new),
        section_id,
        display_order: row.display_order,
        due,
        completion_at: parse_core_data_timestamp(row.completion_date),
        created_at: parse_core_data_timestamp(row.creation_date),
        last_modified_at: parse_core_data_timestamp(row.last_modified_date),
        subtasks,
        tags,
        alarms,
        recurrence,
        attachments,
    }
}

fn due_from_row(row: &ReminderRow) -> Option<Due> {
    parse_core_data_timestamp(row.due_date).map(|at| Due {
        at,
        all_day: row.all_day,
    })
}

pub fn attachment_from_row(row: AttachmentRow) -> ReminderAttachment {
    ReminderAttachment {
        row_id: RowId::new(row.row_id),
        id: ReminderAttachmentId::new(row.id),
        filename: row.filename,
        uti: row.uti,
        sha512: row.sha512,
        kind: attachment_kind_from_raw(row.kind_raw.as_deref()),
        reminder_row_id: RowId::new(row.reminder_row_id),
        modified_at: parse_core_data_timestamp(row.modified_at),
    }
}

pub fn attachment_kind_from_raw(raw: Option<&str>) -> AttachmentKind {
    match raw {
        Some("image") => AttachmentKind::Image,
        Some("audio") => AttachmentKind::Audio,
        Some("file") => AttachmentKind::File,
        Some(other) => AttachmentKind::Unknown(other.to_owned()),
        None => AttachmentKind::Unknown("unknown".to_owned()),
    }
}

pub fn alarm_from_object(row: &ObjectRow, ids: &EntityIds) -> Option<Alarm> {
    if !is_alarm_ent(row.ent, ids) {
        return None;
    }

    let kind = match row.ent {
        ent if ent == ids.alarm_date_trigger => AlarmKind::Absolute,
        ent if ent == ids.alarm_time_interval_trigger => AlarmKind::Relative,
        ent if ent == ids.alarm_location_trigger => AlarmKind::Location,
        ent if ent == ids.alarm => AlarmKind::Unknown,
        _ => AlarmKind::Unknown,
    };

    Some(Alarm {
        row_id: RowId::new(row.row_id),
        kind,
        title: row.title.clone(),
        latitude: row.latitude,
        longitude: row.longitude,
        radius: row.radius,
        time_interval: row.time_interval,
        decode_error: None,
    })
}

pub fn recurrence_from_object(row: &ObjectRow, recurrence_ent: i64) -> Option<RecurrenceRule> {
    if row.ent != recurrence_ent {
        return None;
    }

    Some(RecurrenceRule {
        frequency: row.frequency.unwrap_or(0),
        interval: row.recurrence_interval.unwrap_or(1),
        occurrence_count: row.occurrence_count,
        decode_error: None,
    })
}

pub fn build_section_map(membership_data: Option<&[u8]>) -> HashMap<String, String> {
    parse_section_memberships(membership_data)
}

pub fn smart_filter_from_data(data: Option<&[u8]>) -> SmartFilter {
    decode_smart_filter(data)
}
