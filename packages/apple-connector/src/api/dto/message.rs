use serde::Serialize;
use utoipa::ToSchema;

use super::{
    common::{DirectionDto, HandleDto, TransportDto},
    content::MessageContentDto,
    pagination::PageMetaDto,
};
use crate::apple_types::{ChatId, MessageId, UnixTimestamp};

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MessageSummaryDto {
    pub guid: MessageId,
    pub direction: DirectionDto,
    pub transport: TransportDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sent_at: Option<UnixTimestamp>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sender: Option<HandleDto>,
    pub content: MessageContentDto,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MessageDetailDto {
    pub guid: MessageId,
    pub direction: DirectionDto,
    pub transport: TransportDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sent_at: Option<UnixTimestamp>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub read_at: Option<UnixTimestamp>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edited_at: Option<UnixTimestamp>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retracted_at: Option<UnixTimestamp>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sender: Option<HandleDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply_to_guid: Option<MessageId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thread_originator_guid: Option<MessageId>,
    pub chat_ids: Vec<ChatId>,
    pub content: MessageContentDto,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MessagePageDto {
    pub items: Vec<MessageSummaryDto>,
    pub page: PageMetaDto,
}
