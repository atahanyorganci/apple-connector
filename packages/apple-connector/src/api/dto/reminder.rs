use serde::Serialize;
use utoipa::ToSchema;

use super::{common::timestamp_to_unix, pagination::PageMetaDto};
use crate::apple_types::{
    ReminderAttachmentId, ReminderId, ReminderListId, SectionId, UnixTimestamp,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReminderListKindDto {
    Standard,
    Smart,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct DueDto {
    pub at: UnixTimestamp,
    pub all_day: bool,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SectionSummaryDto {
    pub id: SectionId,
    pub display_name: String,
    pub canonical_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SmartFilterDto {
    pub decoded: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ReminderListSummaryDto {
    pub id: ReminderListId,
    pub row_id: i64,
    pub name: String,
    pub kind: ReminderListKindDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub smart_list_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ReminderListDetailDto {
    pub id: ReminderListId,
    pub row_id: i64,
    pub name: String,
    pub kind: ReminderListKindDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub smart_list_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sharing_status: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shared_owner_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shared_owner_address: Option<String>,
    pub filter: SmartFilterDto,
    pub sections: Vec<SectionSummaryDto>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ReminderSummaryDto {
    pub id: ReminderId,
    pub row_id: i64,
    pub title: String,
    pub completed: bool,
    pub flagged: bool,
    pub priority: i64,
    pub list_id: ReminderListId,
    pub list_row_id: i64,
    pub list_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<ReminderId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub section_id: Option<SectionId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due: Option<DueDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_modified_at: Option<UnixTimestamp>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AlarmKindDto {
    Absolute,
    Relative,
    Location,
    Unknown,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AlarmDto {
    pub kind: AlarmKindDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latitude: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub longitude: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub radius: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_interval: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decode_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RecurrenceDto {
    pub frequency: i64,
    pub interval: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub occurrence_count: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decode_error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReminderAttachmentKindDto {
    File,
    Image,
    Audio,
    Unknown,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ReminderAttachmentSummaryDto {
    pub id: ReminderAttachmentId,
    pub row_id: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uti: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha512: Option<String>,
    pub kind: ReminderAttachmentKindDto,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ReminderDetailDto {
    pub id: ReminderId,
    pub row_id: i64,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    pub completed: bool,
    pub flagged: bool,
    pub priority: i64,
    pub list_id: ReminderListId,
    pub list_row_id: i64,
    pub list_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<ReminderId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub section_id: Option<SectionId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due: Option<DueDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completion_at: Option<UnixTimestamp>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<UnixTimestamp>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_modified_at: Option<UnixTimestamp>,
    pub subtasks: Vec<ReminderSummaryDto>,
    pub tags: Vec<String>,
    pub alarms: Vec<AlarmDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recurrence: Option<RecurrenceDto>,
    pub attachments: Vec<ReminderAttachmentSummaryDto>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ReminderListPageDto {
    pub items: Vec<ReminderListSummaryDto>,
    pub page: PageMetaDto,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ReminderPageDto {
    pub items: Vec<ReminderSummaryDto>,
    pub page: PageMetaDto,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ReminderAttachmentDetailDto {
    pub id: ReminderAttachmentId,
    pub row_id: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uti: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha512: Option<String>,
    pub kind: ReminderAttachmentKindDto,
    pub reminder_id: ReminderId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified_at: Option<UnixTimestamp>,
}

pub fn due_to_dto(due: &crate::reminders::Due) -> DueDto {
    DueDto {
        at: timestamp_to_unix(due.at),
        all_day: due.all_day,
    }
}
