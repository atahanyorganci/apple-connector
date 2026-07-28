use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::pagination::PageMetaDto;
use crate::apple_types::{ContactId, ContainerId, GroupId, SourceId, UnixTimestamp};

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ContainerSummaryDto {
    pub id: ContainerId,
    pub source_id: SourceId,
    pub name: Option<String>,
    pub container_type: i64,
    pub read_only: bool,
}

pub type ContainerDetailDto = ContainerSummaryDto;

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ContainerPageDto {
    pub items: Vec<ContainerSummaryDto>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct GroupSummaryDto {
    pub id: GroupId,
    pub source_id: SourceId,
    pub container_id: ContainerId,
    pub name: Option<String>,
    pub is_smart_group: bool,
    pub is_subscribed: bool,
}

pub type GroupDetailDto = GroupSummaryDto;

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct GroupPageDto {
    pub items: Vec<GroupSummaryDto>,
    pub page: PageMetaDto,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ContactSummaryDto {
    pub id: ContactId,
    pub source_id: SourceId,
    pub container_id: ContainerId,
    pub display_name: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub organization: Option<String>,
    pub modification_date: Option<UnixTimestamp>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ContactPhoneDto {
    pub id: String,
    pub label: Option<String>,
    pub number: String,
    pub is_primary: bool,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ContactEmailDto {
    pub id: String,
    pub label: Option<String>,
    pub address: String,
    pub is_primary: bool,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ContactAddressDto {
    pub id: String,
    pub label: Option<String>,
    pub street: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub postal_code: Option<String>,
    pub country: Option<String>,
    pub is_primary: bool,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ContactUrlDto {
    pub id: String,
    pub label: Option<String>,
    pub url: String,
    pub is_primary: bool,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ContactSocialProfileDto {
    pub id: String,
    pub label: Option<String>,
    pub service: Option<String>,
    pub username: Option<String>,
    pub url: Option<String>,
    pub is_primary: bool,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ContactDetailDto {
    #[serde(flatten)]
    pub summary: ContactSummaryDto,
    pub middle_name: Option<String>,
    pub nickname: Option<String>,
    pub job_title: Option<String>,
    pub department: Option<String>,
    pub note: Option<String>,
    pub birthday: Option<UnixTimestamp>,
    pub creation_date: Option<UnixTimestamp>,
    pub phones: Vec<ContactPhoneDto>,
    pub emails: Vec<ContactEmailDto>,
    pub addresses: Vec<ContactAddressDto>,
    pub urls: Vec<ContactUrlDto>,
    pub social_profiles: Vec<ContactSocialProfileDto>,
    pub group_ids: Vec<GroupId>,
    pub has_photo: bool,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ContactPageDto {
    pub items: Vec<ContactSummaryDto>,
    pub page: PageMetaDto,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct LabeledStringDto {
    pub label: Option<String>,
    pub value: String,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct PostalAddressDto {
    pub label: Option<String>,
    pub street: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub postal_code: Option<String>,
    pub country: Option<String>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateContactRequest {
    pub given_name: Option<String>,
    pub family_name: Option<String>,
    #[serde(default)]
    pub middle_name: Option<String>,
    #[serde(default)]
    pub nickname: Option<String>,
    #[serde(default)]
    pub organization_name: Option<String>,
    #[serde(default)]
    pub job_title: Option<String>,
    #[serde(default)]
    pub department_name: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub phone_numbers: Vec<LabeledStringDto>,
    #[serde(default)]
    pub email_addresses: Vec<LabeledStringDto>,
    #[serde(default)]
    pub postal_addresses: Vec<PostalAddressDto>,
    #[serde(default)]
    pub url_addresses: Vec<LabeledStringDto>,
}

#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
pub struct UpdateContactRequest {
    #[serde(default)]
    pub given_name: Option<String>,
    #[serde(default)]
    pub family_name: Option<String>,
    #[serde(default)]
    pub middle_name: Option<String>,
    #[serde(default)]
    pub nickname: Option<String>,
    #[serde(default)]
    pub organization_name: Option<String>,
    #[serde(default)]
    pub job_title: Option<String>,
    #[serde(default)]
    pub department_name: Option<String>,
    #[serde(default)]
    pub note: Option<Option<String>>,
    #[serde(default)]
    pub phone_numbers: Option<Vec<LabeledStringDto>>,
    #[serde(default)]
    pub email_addresses: Option<Vec<LabeledStringDto>>,
    #[serde(default)]
    pub postal_addresses: Option<Vec<PostalAddressDto>>,
    #[serde(default)]
    pub url_addresses: Option<Vec<LabeledStringDto>>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateGroupRequest {
    pub name: String,
}

#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
pub struct UpdateGroupRequest {
    #[serde(default)]
    pub name: Option<String>,
}
