//! API DTOs kept separate from domain models for the OpenAPI contract.

#![allow(dead_code)]

pub(crate) mod attachment;
pub(crate) mod calendar;
pub(crate) mod calendar_convert;
pub(crate) mod chat;
pub(crate) mod common;
pub(crate) mod contacts;
pub(crate) mod contacts_convert;
pub(crate) mod content;
pub(crate) mod convert;
pub(crate) mod message;
pub(crate) mod note;
pub(crate) mod note_convert;
pub(crate) mod pagination;
pub(crate) mod reminder;
pub(crate) mod reminder_convert;

pub use attachment::AttachmentDetailDto;
pub use calendar::{
    CalendarAccountPageDto, CalendarDetailDto, CalendarPageDto, EventDetailDto, EventPageDto,
};
pub use chat::{ChatDetailDto, ChatPageDto};
pub use contacts::{
    ContactDetailDto, ContactPageDto, ContainerDetailDto, ContainerPageDto, GroupDetailDto,
    GroupPageDto,
};
pub use message::{MessageDetailDto, MessagePageDto};
pub use note::{
    NoteAttachmentDetailDto, NoteDetailDto, NoteFolderDetailDto, NoteFolderPageDto, NotePageDto,
};
pub use pagination::PageMetaDto;
pub use reminder::{
    ReminderAttachmentDetailDto, ReminderDetailDto, ReminderListDetailDto, ReminderListPageDto,
    ReminderPageDto,
};
