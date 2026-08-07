use chrono::{DateTime, Utc};

use crate::apple_types::{
    ReminderAttachmentId, ReminderId, ReminderListId, ReminderPriority, RowId, SectionId,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ListKind {
    Standard,
    Smart,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttachmentKind {
    File,
    Image,
    Audio,
    Unknown(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AlarmKind {
    Absolute,
    Relative,
    Location,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct Due {
    pub at: DateTime<Utc>,
    pub all_day: bool,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ReminderList {
    pub row_id: RowId,
    pub id: ReminderListId,
    pub name: Option<String>,
    pub kind: ListKind,
    pub smart_list_type: Option<String>,
    pub sharing_status: Option<i64>,
    pub shared_owner_name: Option<String>,
    pub shared_owner_address: Option<String>,
    pub filter_data: Option<Vec<u8>>,
    pub membership_data: Option<Vec<u8>>,
    pub sections: Vec<Section>,
    pub last_modified_at: Option<DateTime<Utc>>,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Section {
    pub row_id: RowId,
    pub id: SectionId,
    pub display_name: String,
    pub canonical_name: Option<String>,
    pub list_row_id: RowId,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Reminder {
    pub row_id: RowId,
    pub id: ReminderId,
    pub title: Option<String>,
    pub notes: Option<String>,
    pub completed: bool,
    pub flagged: bool,
    pub priority: ReminderPriority,
    pub list_row_id: RowId,
    pub list_id: ReminderListId,
    pub list_name: String,
    pub parent_row_id: Option<RowId>,
    pub parent_id: Option<ReminderId>,
    pub section_id: Option<SectionId>,
    pub display_order: i64,
    pub due: Option<Due>,
    pub completion_at: Option<DateTime<Utc>>,
    pub created_at: Option<DateTime<Utc>>,
    pub last_modified_at: Option<DateTime<Utc>>,
    pub subtasks: Vec<ReminderSummary>,
    pub tags: Vec<String>,
    pub alarms: Vec<Alarm>,
    pub recurrence: Option<RecurrenceRule>,
    pub attachments: Vec<ReminderAttachment>,
}

#[derive(Debug, Clone)]
pub struct ReminderSummary {
    pub row_id: RowId,
    pub id: ReminderId,
    pub title: Option<String>,
    pub completed: bool,
    pub flagged: bool,
    pub priority: ReminderPriority,
    pub list_row_id: RowId,
    pub list_id: ReminderListId,
    pub list_name: String,
    pub parent_id: Option<ReminderId>,
    pub section_id: Option<SectionId>,
    pub due: Option<Due>,
    pub last_modified_at: Option<DateTime<Utc>>,
    pub tags: Vec<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Alarm {
    pub row_id: RowId,
    pub kind: AlarmKind,
    pub title: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub radius: Option<f64>,
    pub time_interval: Option<f64>,
    pub decode_error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RecurrenceRule {
    pub frequency: i64,
    pub interval: i64,
    pub occurrence_count: Option<i64>,
    pub decode_error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ReminderAttachment {
    pub row_id: RowId,
    pub id: ReminderAttachmentId,
    pub filename: Option<String>,
    pub uti: Option<String>,
    pub sha512: Option<String>,
    pub kind: AttachmentKind,
    pub reminder_row_id: RowId,
    pub modified_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Default)]
pub struct SmartFilter {
    pub decoded: bool,
    pub raw: Option<serde_json::Value>,
}
