use super::{
    attachment::{AttachmentDetailDto, AttachmentKindDto, AttachmentSummaryDto},
    chat::{ChatDetailDto, ChatSummaryDto},
    common::{direction_to_dto, handle_to_dto, timestamp_to_unix, transport_to_dto},
    content::{
        AppBalloonContentDto, AppBalloonKindDto, AttachmentContentDto, AttributedBodyErrorDto,
        AudioContentDto, GroupActionKindDto, GroupEventContentDto, MessageBodyDto,
        MessageContentDto, OpaquePayloadDto, PhotosBalloonDto, PollBalloonDto, PollOptionDto,
        ReactionActionDto, ReactionContentDto, ReactionKindDto, ShareMyLocationContentDto,
        ShareMyLocationStatusDto, SharePlayContentDto, SystemContentDto, TapbackDto,
        TextContentDto, UnknownContentDto, UrlBalloonDto,
    },
    message::{MessageDetailDto, MessageSummaryDto},
};
use crate::{
    apple_types::{AttachmentId, ChatId, MessageId},
    messages::{
        AppBalloon, AppBalloonKind, Attachment, AttachmentKind, AttributedBodyDecodeError, Chat,
        GroupActionKind, GroupEvent, Message, MessageBody, MessageContent, Reaction,
        ReactionAction, ReactionKind, ShareMyLocationMessage, ShareMyLocationStatus,
        SharePlayMessage, SystemMessage, Tapback, TextMessage, UnknownMessage,
    },
};

pub fn chat_summary_to_dto(chat: &Chat) -> ChatSummaryDto {
    ChatSummaryDto {
        id: ChatId::new(chat.row_id),
        guid: chat.guid.clone(),
        display_name: chat.display_name.clone(),
        is_group: chat.is_group,
        transport: transport_to_dto(&chat.transport),
        participant_count: chat.participants.len() as u32,
    }
}

pub fn chat_detail_to_dto(chat: &Chat) -> ChatDetailDto {
    ChatDetailDto {
        id: ChatId::new(chat.row_id),
        guid: chat.guid.clone(),
        identifier: chat.identifier.clone(),
        display_name: chat.display_name.clone(),
        room_name: chat.room_name.clone(),
        is_group: chat.is_group,
        transport: transport_to_dto(&chat.transport),
        participants: chat.participants.iter().map(handle_to_dto).collect(),
    }
}

pub fn message_summary_to_dto(message: &Message) -> MessageSummaryDto {
    MessageSummaryDto {
        guid: MessageId::new(message.envelope.guid.clone()),
        direction: direction_to_dto(message.envelope.direction),
        transport: transport_to_dto(&message.envelope.transport),
        sent_at: message.envelope.sent_at.map(timestamp_to_unix),
        sender: message.envelope.sender.as_ref().map(handle_to_dto),
        content: content_to_dto(&message.content),
    }
}

pub fn message_detail_to_dto(message: &Message) -> MessageDetailDto {
    MessageDetailDto {
        guid: MessageId::new(message.envelope.guid.clone()),
        direction: direction_to_dto(message.envelope.direction),
        transport: transport_to_dto(&message.envelope.transport),
        sent_at: message.envelope.sent_at.map(timestamp_to_unix),
        read_at: message.envelope.read_at.map(timestamp_to_unix),
        edited_at: message.envelope.edited_at.map(timestamp_to_unix),
        retracted_at: message.envelope.retracted_at.map(timestamp_to_unix),
        sender: message.envelope.sender.as_ref().map(handle_to_dto),
        reply_to_guid: message.envelope.reply_to_guid.clone().map(MessageId::new),
        thread_originator_guid: message
            .envelope
            .thread_originator_guid
            .clone()
            .map(MessageId::new),
        chat_ids: message
            .envelope
            .chat_ids
            .iter()
            .map(|id| ChatId::new(*id))
            .collect(),
        content: content_to_dto(&message.content),
    }
}

pub fn attachment_summary_to_dto(attachment: &Attachment) -> AttachmentSummaryDto {
    AttachmentSummaryDto {
        guid: AttachmentId::new(attachment.guid.clone()),
        original_guid: AttachmentId::new(attachment.original_guid.clone()),
        mime_type: attachment.mime_type.clone(),
        uti: attachment.uti.clone(),
        transfer_name: attachment.transfer_name.clone(),
        total_bytes: attachment.total_bytes,
        kind: attachment_kind_to_dto(&attachment.kind),
        transfer_complete: attachment.transfer_complete,
        present_on_disk: attachment.present_on_disk,
        hide_attachment: attachment.hide_attachment,
        metadata_url: attachment_metadata_url(&attachment.guid),
        content_url: attachment_content_url(&attachment.guid),
    }
}

pub fn attachment_detail_to_dto(attachment: &Attachment) -> AttachmentDetailDto {
    AttachmentDetailDto {
        guid: AttachmentId::new(attachment.guid.clone()),
        original_guid: AttachmentId::new(attachment.original_guid.clone()),
        mime_type: attachment.mime_type.clone(),
        uti: attachment.uti.clone(),
        transfer_name: attachment.transfer_name.clone(),
        total_bytes: attachment.total_bytes,
        kind: attachment_kind_to_dto(&attachment.kind),
        transfer_complete: attachment.transfer_complete,
        present_on_disk: attachment.present_on_disk,
        hide_attachment: attachment.hide_attachment,
        emoji_description: attachment.emoji_description.clone(),
        metadata_url: attachment_metadata_url(&attachment.guid),
        content_url: attachment_content_url(&attachment.guid),
    }
}

pub fn attachment_metadata_url(guid: &str) -> String {
    format!("/v1/attachments/{guid}")
}

pub fn attachment_content_url(guid: &str) -> String {
    format!("/v1/attachments/{guid}/content")
}

fn content_to_dto(content: &MessageContent) -> MessageContentDto {
    match content {
        MessageContent::Text(text) => MessageContentDto::Text(text_to_dto(text)),
        MessageContent::Audio(audio) => MessageContentDto::Audio(AudioContentDto {
            body: body_to_dto(&audio.body),
            attachments: audio
                .attachments
                .iter()
                .map(attachment_summary_to_dto)
                .collect(),
        }),
        MessageContent::Attachment(attachment) => {
            MessageContentDto::Attachment(AttachmentContentDto {
                body: body_to_dto(&attachment.body),
                attachments: attachment
                    .attachments
                    .iter()
                    .map(attachment_summary_to_dto)
                    .collect(),
            })
        }
        MessageContent::Reaction(reaction) => {
            MessageContentDto::Reaction(reaction_to_dto(reaction))
        }
        MessageContent::GroupEvent(event) => {
            MessageContentDto::GroupEvent(group_event_to_dto(event))
        }
        MessageContent::AppBalloon(balloon) => {
            MessageContentDto::AppBalloon(app_balloon_to_dto(balloon))
        }
        MessageContent::SharePlay(share_play) => {
            MessageContentDto::SharePlay(share_play_to_dto(share_play))
        }
        MessageContent::ShareMyLocation(location) => {
            MessageContentDto::ShareMyLocation(share_my_location_to_dto(location))
        }
        MessageContent::System(system) => MessageContentDto::System(system_to_dto(system)),
        MessageContent::Unknown(unknown) => MessageContentDto::Unknown(unknown_to_dto(unknown)),
    }
}

fn body_to_dto(body: &MessageBody) -> MessageBodyDto {
    MessageBodyDto {
        text: body.text.clone(),
        attributed_body_error: body.attributed_body_error.map(attributed_body_error_to_dto),
    }
}

fn text_to_dto(text: &TextMessage) -> TextContentDto {
    TextContentDto {
        body: body_to_dto(&text.body),
        is_forward: text.is_forward,
        is_auto_reply: text.is_auto_reply,
    }
}

fn reaction_to_dto(reaction: &Reaction) -> ReactionContentDto {
    ReactionContentDto {
        target_guid: reaction.target_guid.clone().map(MessageId::new),
        kind: match &reaction.kind {
            ReactionKind::Tapback(tapback, action) => ReactionKindDto::Tapback {
                tapback: tapback_to_dto(*tapback),
                action: reaction_action_to_dto(*action),
            },
            ReactionKind::ApplePay => ReactionKindDto::ApplePay,
            ReactionKind::Unknown(code) => ReactionKindDto::Unknown { code: *code },
        },
    }
}

fn group_event_to_dto(event: &GroupEvent) -> GroupEventContentDto {
    GroupEventContentDto {
        action: group_action_kind_to_dto(&event.action),
        title: event.title.clone(),
        actor: event.actor.clone(),
    }
}

fn app_balloon_to_dto(balloon: &AppBalloon) -> AppBalloonContentDto {
    AppBalloonContentDto {
        bundle_id: balloon.bundle_id.clone(),
        text: balloon.text.clone(),
        kind: match &balloon.kind {
            AppBalloonKind::Url(url) => AppBalloonKindDto::Url(UrlBalloonDto {
                url: url.url.clone(),
                title: url.title.clone(),
                summary: url.summary.clone(),
            }),
            AppBalloonKind::Photos(photos) => AppBalloonKindDto::Photos(PhotosBalloonDto {
                url: photos.url.clone(),
                caption: photos.caption.clone(),
            }),
            AppBalloonKind::Poll(poll) => AppBalloonKindDto::Poll(PollBalloonDto {
                title: poll.title.clone(),
                options: poll
                    .options
                    .iter()
                    .map(|option| PollOptionDto {
                        text: option.text.clone(),
                        option_id: option.option_id.clone(),
                    })
                    .collect(),
            }),
            AppBalloonKind::DigitalTouch => AppBalloonKindDto::DigitalTouch,
            AppBalloonKind::Unknown { payload_data } => AppBalloonKindDto::Unknown {
                payload: opaque_payload(payload_data.as_deref()),
            },
        },
    }
}

fn share_play_to_dto(share_play: &SharePlayMessage) -> SharePlayContentDto {
    SharePlayContentDto {
        text: share_play.text.clone(),
        payload: opaque_payload(share_play.payload_data.as_deref()),
    }
}

fn share_my_location_to_dto(location: &ShareMyLocationMessage) -> ShareMyLocationContentDto {
    ShareMyLocationContentDto {
        status: match location.status {
            ShareMyLocationStatus::Started => ShareMyLocationStatusDto::Started,
            ShareMyLocationStatus::Stopped => ShareMyLocationStatusDto::Stopped,
        },
        other_handle: location.other_handle.clone(),
    }
}

fn system_to_dto(system: &SystemMessage) -> SystemContentDto {
    SystemContentDto {
        is_system: system.is_system,
        is_service: system.is_service,
        text: system.text.clone(),
    }
}

fn unknown_to_dto(unknown: &UnknownMessage) -> UnknownContentDto {
    UnknownContentDto {
        item_type: unknown.item_type,
        associated_message_type: unknown.associated_message_type,
        text: unknown.text.clone(),
        attachments: unknown
            .attachments
            .iter()
            .map(attachment_summary_to_dto)
            .collect(),
    }
}

fn attachment_kind_to_dto(kind: &AttachmentKind) -> AttachmentKindDto {
    match kind {
        AttachmentKind::Sticker { animated } => AttachmentKindDto::Sticker {
            animated: *animated,
        },
        AttachmentKind::Image => AttachmentKindDto::Image,
        AttachmentKind::Video => AttachmentKindDto::Video,
        AttachmentKind::Audio => AttachmentKindDto::Audio,
        AttachmentKind::File => AttachmentKindDto::File,
        AttachmentKind::Unknown => AttachmentKindDto::Unknown,
    }
}

fn attributed_body_error_to_dto(error: AttributedBodyDecodeError) -> AttributedBodyErrorDto {
    match error {
        AttributedBodyDecodeError::InvalidTypedStream => AttributedBodyErrorDto::InvalidTypedStream,
        AttributedBodyDecodeError::NotAttributedString => {
            AttributedBodyErrorDto::NotAttributedString
        }
        AttributedBodyDecodeError::MissingText => AttributedBodyErrorDto::MissingText,
    }
}

fn tapback_to_dto(tapback: Tapback) -> TapbackDto {
    match tapback {
        Tapback::Love => TapbackDto::Love,
        Tapback::Like => TapbackDto::Like,
        Tapback::Dislike => TapbackDto::Dislike,
        Tapback::Laugh => TapbackDto::Laugh,
        Tapback::Emphasize => TapbackDto::Emphasize,
        Tapback::Question => TapbackDto::Question,
        Tapback::Unknown(_) => TapbackDto::Unknown,
    }
}

fn reaction_action_to_dto(action: ReactionAction) -> ReactionActionDto {
    match action {
        ReactionAction::Added => ReactionActionDto::Added,
        ReactionAction::Removed => ReactionActionDto::Removed,
    }
}

fn group_action_kind_to_dto(action: &GroupActionKind) -> GroupActionKindDto {
    match action {
        GroupActionKind::ParticipantAdded => GroupActionKindDto::ParticipantAdded,
        GroupActionKind::ParticipantRemoved => GroupActionKindDto::ParticipantRemoved,
        GroupActionKind::NameChange => GroupActionKindDto::NameChange,
        GroupActionKind::ParticipantLeft => GroupActionKindDto::ParticipantLeft,
        GroupActionKind::GroupIconChanged => GroupActionKindDto::GroupIconChanged,
        GroupActionKind::GroupIconRemoved => GroupActionKindDto::GroupIconRemoved,
        GroupActionKind::ChatBackgroundChanged => GroupActionKindDto::ChatBackgroundChanged,
        GroupActionKind::ChatBackgroundRemoved => GroupActionKindDto::ChatBackgroundRemoved,
        GroupActionKind::PhoneNumberChanged => GroupActionKindDto::PhoneNumberChanged,
        GroupActionKind::Unknown { .. } => GroupActionKindDto::Unknown,
    }
}

fn opaque_payload(payload: Option<&[u8]>) -> OpaquePayloadDto {
    match payload {
        Some(bytes) => OpaquePayloadDto {
            present: true,
            size_bytes: Some(bytes.len() as u64),
        },
        None => OpaquePayloadDto {
            present: false,
            size_bytes: None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        fixtures::FixtureDb,
        messages::{Attachment, AttachmentKind, load_chats},
    };

    const FORBIDDEN_SUBSTRINGS: &[&str] = &[
        "chat.db",
        "Library/Messages",
        "/Users/",
        "filename",
        "resolved_path",
        "payload_data",
    ];

    fn assert_safe_json(value: &serde_json::Value) {
        let serialized = serde_json::to_string(value).expect("serialize dto");
        for forbidden in FORBIDDEN_SUBSTRINGS {
            assert!(
                !serialized.contains(forbidden),
                "serialized dto leaked `{forbidden}`: {serialized}"
            );
        }
    }

    #[test]
    fn attachment_dto_omits_local_paths() {
        let attachment = Attachment {
            guid: "at-guid".to_owned(),
            original_guid: "at-guid".to_owned(),
            filename: Some("~/Library/Messages/Attachments/at/file.jpg".to_owned()),
            resolved_path: Some("/Users/test/Library/Messages/Attachments/at/file.jpg".to_owned()),
            uti: Some("public.jpeg".to_owned()),
            mime_type: Some("image/jpeg".to_owned()),
            transfer_name: Some("photo.jpg".to_owned()),
            total_bytes: 1234,
            kind: AttachmentKind::Image,
            transfer_state: 5,
            transfer_complete: true,
            present_on_disk: true,
            hide_attachment: false,
            emoji_description: None,
            body_reference: None,
        };

        let dto = attachment_detail_to_dto(&attachment);
        assert_safe_json(&serde_json::to_value(dto).expect("dto json"));
    }

    #[tokio::test]
    async fn seeded_fixture_dtos_do_not_leak_paths_or_payloads() {
        use sqlx::{Connection, sqlite::SqliteConnectOptions};

        let fixture = FixtureDb::seeded().await.expect("seeded fixture");
        let options = SqliteConnectOptions::new()
            .filename(fixture.path())
            .read_only(true);
        let mut connection = sqlx::SqliteConnection::connect_with(&options)
            .await
            .expect("connect fixture");
        let chats = load_chats(&mut connection).await.expect("load chats");

        for chat in &chats {
            assert_safe_json(&serde_json::to_value(chat_summary_to_dto(chat)).expect("chat json"));
            assert_safe_json(&serde_json::to_value(chat_detail_to_dto(chat)).expect("chat json"));

            for message in &chat.messages {
                assert_safe_json(
                    &serde_json::to_value(message_summary_to_dto(message)).expect("message json"),
                );
                assert_safe_json(
                    &serde_json::to_value(message_detail_to_dto(message)).expect("message json"),
                );
            }
        }
    }
}
