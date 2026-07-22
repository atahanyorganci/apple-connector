use super::{
    model::{
        AppBalloon, Attachment, AttachmentMessage, AudioMessage, GroupActionKind, GroupEvent,
        MessageBody, MessageContent, Reaction, ReactionAction, ReactionKind, SharePlayMessage,
        SystemMessage, Tapback, TextMessage, UnknownMessage,
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
            text: row.text.clone(),
            has_attributed_body: row.has_attributed_body(),
        });
    }

    match row.item_type {
        0 => classify_normal(row, attachments),
        2 => MessageContent::GroupEvent(GroupEvent {
            action: GroupActionKind::Unknown(row.group_action_type),
            title: row.group_title.clone(),
            actor: row.other_handle_id.clone(),
        }),
        6 => MessageContent::SharePlay(SharePlayMessage {
            balloon_bundle_id: row.balloon_bundle_id.clone(),
            payload_data: row.payload_data.clone(),
            text: row.text.clone(),
        }),
        _ => unknown(row, attachments),
    }
}

fn classify_normal(row: &MessageRow, attachments: &[AttachmentRow]) -> MessageContent {
    if let Some(bundle_id) = row
        .balloon_bundle_id
        .clone()
        .filter(|bundle_id| !bundle_id.is_empty())
    {
        return MessageContent::AppBalloon(AppBalloon {
            bundle_id,
            payload_data: row.payload_data.clone(),
            text: row.text.clone(),
        });
    }

    let attachments = to_attachments(attachments);

    if row.is_audio_message {
        return MessageContent::Audio(AudioMessage {
            body: message_body(row),
            attachments,
        });
    }

    if !attachments.is_empty() || row.cache_has_attachments {
        return MessageContent::Attachment(AttachmentMessage {
            body: message_body(row),
            attachments,
        });
    }

    MessageContent::Text(TextMessage {
        body: message_body(row),
        is_forward: row.is_forward,
        is_auto_reply: row.is_auto_reply,
        expressive_send_style_id: row.expressive_send_style_id.clone(),
    })
}

fn unknown(row: &MessageRow, attachments: &[AttachmentRow]) -> MessageContent {
    MessageContent::Unknown(UnknownMessage {
        item_type: row.item_type,
        associated_message_type: row.associated_message_type,
        text: row.text.clone(),
        has_attributed_body: row.has_attributed_body(),
        attachments: to_attachments(attachments),
    })
}

fn message_body(row: &MessageRow) -> MessageBody {
    MessageBody {
        text: row.text.clone(),
        has_attributed_body: row.has_attributed_body(),
    }
}

fn to_attachments(attachments: &[AttachmentRow]) -> Vec<Attachment> {
    attachments
        .iter()
        .map(|attachment| Attachment {
            guid: attachment.guid.clone(),
            filename: attachment.filename.clone(),
            uti: attachment.uti.clone(),
            mime_type: attachment.mime_type.clone(),
            transfer_name: attachment.transfer_name.clone(),
            total_bytes: attachment.total_bytes,
            is_sticker: attachment.is_sticker,
        })
        .collect()
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
