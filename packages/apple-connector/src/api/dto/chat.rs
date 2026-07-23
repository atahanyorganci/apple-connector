use serde::Serialize;
use utoipa::ToSchema;

use super::{
    common::{HandleDto, TransportDto},
    pagination::PageMetaDto,
};
use crate::apple_types::ChatId;

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ChatSummaryDto {
    pub id: ChatId,
    pub guid: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    pub is_group: bool,
    pub transport: TransportDto,
    #[schema(minimum = 0)]
    pub participant_count: u32,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ChatDetailDto {
    pub id: ChatId,
    pub guid: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identifier: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub room_name: Option<String>,
    pub is_group: bool,
    pub transport: TransportDto,
    pub participants: Vec<HandleDto>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ChatPageDto {
    pub items: Vec<ChatSummaryDto>,
    pub page: PageMetaDto,
}
