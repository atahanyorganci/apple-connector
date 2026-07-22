use serde::Serialize;
use utoipa::ToSchema;

use super::{
    common::{DirectionDto, HandleDto, TransportDto},
    content::MessageContentDto,
    pagination::PageMetaDto,
};

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MessageSummaryDto {
    pub guid: String,
    pub direction: DirectionDto,
    pub transport: TransportDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(format = "date-time", example = "2024-01-15T12:00:00Z")]
    pub sent_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sender: Option<HandleDto>,
    pub content: MessageContentDto,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MessageDetailDto {
    pub guid: String,
    pub direction: DirectionDto,
    pub transport: TransportDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(format = "date-time", example = "2024-01-15T12:00:00Z")]
    pub sent_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(format = "date-time", example = "2024-01-15T12:05:00Z")]
    pub read_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(format = "date-time", example = "2024-01-15T12:10:00Z")]
    pub edited_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(format = "date-time")]
    pub retracted_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sender: Option<HandleDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply_to_guid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thread_originator_guid: Option<String>,
    pub chat_ids: Vec<i64>,
    pub content: MessageContentDto,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MessagePageDto {
    pub items: Vec<MessageSummaryDto>,
    pub page: PageMetaDto,
}
