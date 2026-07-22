use super::{
    attachments, attributed_body, balloon,
    model::{
        AttachmentMessage, AudioMessage, GroupActionKind, GroupEvent, MessageBody, MessageContent,
        Reaction, ReactionAction, ReactionKind, ShareMyLocationMessage, ShareMyLocationStatus,
        SharePlayMessage, SystemMessage, Tapback, TextMessage, UnknownMessage,
    },
    row::{AttachmentRow, MessageRow},
};

pub fn classify(row: &MessageRow, attachments: &[AttachmentRow]) -> MessageContent {
    if let Some(reaction) = parse_reaction(row) {
        return MessageContent::Reaction(reaction);
    }

    if row.is_system_message || row.is_service_message {
        return MessageContent::System(SystemMessage {
            is_system: row.is_system_message,
            is_service: row.is_service_message,
            text: message_body(row).text,
        });
    }

    match row.item_type {
        0 => classify_normal(row, attachments),
        1..=3 => MessageContent::GroupEvent(GroupEvent {
            action: map_group_action(row),
            title: row.group_title.clone(),
            actor: row.other_handle_id.clone(),
        }),
        4 => MessageContent::ShareMyLocation(ShareMyLocationMessage {
            status: if row.share_status {
                ShareMyLocationStatus::Stopped
            } else {
                ShareMyLocationStatus::Started
            },
            other_handle: row.other_handle_id.clone(),
        }),
        6 => MessageContent::SharePlay(SharePlayMessage {
            balloon_bundle_id: row.balloon_bundle_id.clone(),
            payload_data: row.payload_data.clone(),
            text: message_body(row).text,
        }),
        _ => unknown(row, attachments),
    }
}

fn map_group_action(row: &MessageRow) -> GroupActionKind {
    match (
        row.item_type,
        row.group_action_type,
        row.other_handle,
        row.group_title.as_deref(),
    ) {
        (1, 0, who, _) if who != 0 && row.handle_id == who => GroupActionKind::PhoneNumberChanged,
        (1, 0, who, _) if who != 0 => GroupActionKind::ParticipantAdded,
        (1, 1, who, _) if who != 0 => GroupActionKind::ParticipantRemoved,
        (2, _, _, Some(_)) => GroupActionKind::NameChange,
        (3, 0, _, _) => GroupActionKind::ParticipantLeft,
        (3, 1, _, _) => GroupActionKind::GroupIconChanged,
        (3, 2, _, _) => GroupActionKind::GroupIconRemoved,
        (3, 4, _, _) => GroupActionKind::ChatBackgroundChanged,
        (3, 6, _, _) => GroupActionKind::ChatBackgroundRemoved,
        (item_type, group_action_type, _, _) => GroupActionKind::Unknown {
            item_type,
            group_action_type,
        },
    }
}

fn classify_normal(row: &MessageRow, attachments: &[AttachmentRow]) -> MessageContent {
    if let Some(bundle_id) = row
        .balloon_bundle_id
        .clone()
        .filter(|bundle_id| !bundle_id.is_empty())
    {
        return MessageContent::AppBalloon(balloon::decode(
            bundle_id,
            row.payload_data.as_deref(),
            message_body(row).text,
        ));
    }

    let body = message_body(row);
    let attachments = attachments::assemble_attachments(attachments, &body);

    if row.is_audio_message {
        return MessageContent::Audio(AudioMessage { body, attachments });
    }

    if !attachments.is_empty() || row.cache_has_attachments {
        return MessageContent::Attachment(AttachmentMessage { body, attachments });
    }

    MessageContent::Text(TextMessage {
        body,
        is_forward: row.is_forward,
        is_auto_reply: row.is_auto_reply,
        expressive_send_style_id: row.expressive_send_style_id.clone(),
    })
}

fn unknown(row: &MessageRow, attachments: &[AttachmentRow]) -> MessageContent {
    let body = message_body(row);
    MessageContent::Unknown(UnknownMessage {
        item_type: row.item_type,
        associated_message_type: row.associated_message_type,
        text: body.text.clone(),
        attachments: attachments::assemble_attachments(attachments, &body),
    })
}

fn message_body(row: &MessageRow) -> MessageBody {
    if let Some(body) = row
        .attributed_body
        .as_deref()
        .and_then(attributed_body::decode)
    {
        return body;
    }

    let text = row
        .text
        .as_deref()
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_owned);

    MessageBody {
        text,
        runs: Vec::new(),
    }
}

fn parse_reaction(row: &MessageRow) -> Option<Reaction> {
    let reaction_type = row.associated_message_type;
    if reaction_type == 0 {
        return None;
    }

    let target_guid = row.associated_message_guid.clone();

    if reaction_type == 3 {
        return Some(Reaction {
            target_guid,
            kind: ReactionKind::ApplePay,
        });
    }

    if (2000..=2005).contains(&reaction_type) {
        return Some(Reaction {
            target_guid,
            kind: ReactionKind::Tapback(map_tapback(reaction_type), ReactionAction::Added),
        });
    }

    if (3000..=3005).contains(&reaction_type) {
        return Some(Reaction {
            target_guid,
            kind: ReactionKind::Tapback(map_tapback(reaction_type - 1000), ReactionAction::Removed),
        });
    }

    Some(Reaction {
        target_guid,
        kind: ReactionKind::Unknown(reaction_type),
    })
}

fn map_tapback(value: i64) -> Tapback {
    match value {
        2000 => Tapback::Love,
        2001 => Tapback::Like,
        2002 => Tapback::Dislike,
        2003 => Tapback::Laugh,
        2004 => Tapback::Emphasize,
        2005 => Tapback::Question,
        other => Tapback::Unknown(other),
    }
}

#[cfg(test)]
mod tests {
    use super::{classify, map_group_action, message_body};
    use crate::messages::{
        model::{GroupActionKind, MessageContent, ShareMyLocationStatus},
        row::MessageRow,
    };

    fn empty_row() -> MessageRow {
        MessageRow {
            row_id: 1,
            guid: "guid".to_owned(),
            text: None,
            attributed_body: None,
            service: None,
            sent_at: 0,
            read_at: 0,
            edited_at: 0,
            retracted_at: 0,
            is_from_me: false,
            sender_id: None,
            sender_service: None,
            item_type: 0,
            associated_message_guid: None,
            associated_message_type: 0,
            group_action_type: 0,
            group_title: None,
            handle_id: 0,
            other_handle: 0,
            other_handle_id: None,
            share_status: false,
            balloon_bundle_id: None,
            payload_data: None,
            is_audio_message: false,
            cache_has_attachments: false,
            is_forward: false,
            is_auto_reply: false,
            is_system_message: false,
            is_service_message: false,
            reply_to_guid: None,
            thread_originator_guid: None,
            expressive_send_style_id: None,
        }
    }

    #[test]
    fn prefers_plain_text_column_when_attributed_body_is_unusable() {
        let mut row = empty_row();
        row.text = Some("  hello  ".to_owned());
        row.attributed_body = Some(b"not a typedstream".to_vec());

        assert_eq!(message_body(&row).text.as_deref(), Some("hello"));
        assert!(message_body(&row).runs.is_empty());
    }

    #[test]
    fn falls_back_to_attributed_body() {
        let mut row = empty_row();
        row.attributed_body =
            Some(include_bytes!("../../fixtures/messages/attributed-body-hello.bin").to_vec());

        assert_eq!(message_body(&row).text.as_deref(), Some("Noter test"));
    }

    #[test]
    fn ignores_malformed_attributed_body() {
        let mut row = empty_row();
        row.attributed_body = Some(b"not a typedstream".to_vec());

        assert_eq!(message_body(&row).text, None);
    }

    #[test]
    fn classifies_share_my_location_started() {
        let mut row = empty_row();
        row.item_type = 4;
        row.share_status = false;
        row.other_handle_id = Some("+15551212".to_owned());

        match classify(&row, &[]) {
            MessageContent::ShareMyLocation(location) => {
                assert_eq!(location.status, ShareMyLocationStatus::Started);
                assert_eq!(location.other_handle.as_deref(), Some("+15551212"));
            }
            other => panic!("expected ShareMyLocation, got {other:?}"),
        }
    }

    #[test]
    fn classifies_share_my_location_stopped() {
        let mut row = empty_row();
        row.item_type = 4;
        row.share_status = true;

        match classify(&row, &[]) {
            MessageContent::ShareMyLocation(location) => {
                assert_eq!(location.status, ShareMyLocationStatus::Stopped);
            }
            other => panic!("expected ShareMyLocation, got {other:?}"),
        }
    }

    #[test]
    fn classifies_group_rename_from_bro_txt() {
        let mut row = empty_row();
        row.item_type = 2;
        row.group_action_type = 0;
        row.group_title = Some("batakhaklıyımikiüstübatar".to_owned());

        match classify(&row, &[]) {
            MessageContent::GroupEvent(event) => {
                assert_eq!(event.action, GroupActionKind::NameChange);
                assert_eq!(event.title.as_deref(), Some("batakhaklıyımikiüstübatar"));
            }
            other => panic!("expected GroupEvent, got {other:?}"),
        }
    }

    #[test]
    fn maps_group_icon_and_participant_actions() {
        let mut row = empty_row();
        row.item_type = 3;
        row.group_action_type = 1;
        assert_eq!(map_group_action(&row), GroupActionKind::GroupIconChanged);

        row.item_type = 1;
        row.group_action_type = 0;
        row.other_handle = 42;
        row.handle_id = 7;
        assert_eq!(map_group_action(&row), GroupActionKind::ParticipantAdded);

        row.handle_id = 42;
        assert_eq!(map_group_action(&row), GroupActionKind::PhoneNumberChanged);
    }

    #[test]
    fn keeps_caption_body_on_attachment_message() {
        use crate::messages::{
            model::{AttachmentKind, BodyAttribute},
            row::AttachmentRow,
        };

        let mut row = empty_row();
        row.cache_has_attachments = true;
        row.attributed_body = Some(
            include_bytes!(
                "../../../apple-typedstream/fixtures/attributed-body-18-photo-caption.bin"
            )
            .to_vec(),
        );

        let attachment = AttachmentRow {
            message_id: 1,
            guid: "714A7477-1CA9-4EA8-8D65-C3FB7DEB0C39".to_owned(),
            original_guid: "714A7477-1CA9-4EA8-8D65-C3FB7DEB0C39".to_owned(),
            filename: None,
            uti: Some("public.jpeg".to_owned()),
            mime_type: Some("image/jpeg".to_owned()),
            transfer_name: Some("photo.jpg".to_owned()),
            total_bytes: 12,
            is_sticker: false,
            transfer_state: 5,
            hide_attachment: false,
            emoji_description: None,
        };

        match classify(&row, &[attachment]) {
            MessageContent::Attachment(message) => {
                assert_eq!(
                    message.body.text.as_deref(),
                    Some("\u{fffc}fixture: photo caption")
                );
                assert_eq!(message.attachments.len(), 1);
                assert_eq!(message.attachments[0].kind, AttachmentKind::Image);
                assert!(message.attachments[0].transfer_complete);
                assert_eq!(
                    message.attachments[0]
                        .body_reference
                        .as_ref()
                        .map(|reference| reference.part),
                    Some(Some(0))
                );
                assert!(matches!(
                    message.body.runs[0].attributes[0],
                    BodyAttribute::FileTransfer { .. }
                ));
            }
            other => panic!("expected Attachment, got {other:?}"),
        }
    }
}
