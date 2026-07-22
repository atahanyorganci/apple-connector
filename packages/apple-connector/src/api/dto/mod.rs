//! API DTOs kept separate from domain models for the OpenAPI contract.

#![allow(dead_code)]

pub(crate) mod attachment;
pub(crate) mod chat;
pub(crate) mod common;
pub(crate) mod content;
pub(crate) mod convert;
pub(crate) mod message;
pub(crate) mod pagination;

pub use attachment::AttachmentDetailDto;
pub use chat::{ChatDetailDto, ChatPageDto};
pub use message::{MessageDetailDto, MessagePageDto};
pub use pagination::PageMetaDto;
