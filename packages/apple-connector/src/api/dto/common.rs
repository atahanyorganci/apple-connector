use serde::Serialize;
use utoipa::ToSchema;

use crate::{
    apple_types::UnixTimestamp,
    messages::{Direction, Handle, Transport},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum DirectionDto {
    Sent,
    Received,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum TransportDto {
    Imessage,
    Sms,
    Rcs,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
pub struct HandleDto {
    pub id: String,
    pub service: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatus {
    Ok,
    Unavailable,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct HealthStatusDto {
    pub messages: HealthStatus,
    pub reminders: HealthStatus,
    pub notes: HealthStatus,
    pub calendar: HealthStatus,
}

pub fn direction_to_dto(direction: Direction) -> DirectionDto {
    match direction {
        Direction::Sent => DirectionDto::Sent,
        Direction::Received => DirectionDto::Received,
    }
}

pub fn transport_to_dto(transport: &Transport) -> TransportDto {
    match transport {
        Transport::IMessage => TransportDto::Imessage,
        Transport::Sms => TransportDto::Sms,
        Transport::Rcs => TransportDto::Rcs,
        Transport::Unknown(_) => TransportDto::Unknown,
    }
}

pub fn handle_to_dto(handle: &Handle) -> HandleDto {
    HandleDto {
        id: handle.id.clone(),
        service: handle.service.clone(),
    }
}

pub fn timestamp_to_unix(timestamp: chrono::DateTime<chrono::Utc>) -> UnixTimestamp {
    UnixTimestamp::from(timestamp)
}
