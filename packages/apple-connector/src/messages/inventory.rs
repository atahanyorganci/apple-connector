use std::fmt;

use super::model::{
    AppBalloonKind, AttachmentKind, AttributedBodyDecodeError, Message, MessageContent,
};

/// Aggregate counts for a loaded message dump — useful as a regression checklist
/// after classifier / body-parser changes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MessageInventory {
    pub total: usize,
    pub text: usize,
    pub audio: usize,
    pub attachment: usize,
    pub reaction: usize,
    pub group_event: usize,
    pub app_balloon: usize,
    pub share_play: usize,
    pub share_my_location: usize,
    pub system: usize,
    pub unknown: usize,
    pub balloon_url: usize,
    pub balloon_photos: usize,
    pub balloon_poll: usize,
    pub balloon_digital_touch: usize,
    pub balloon_unknown: usize,
    pub sticker_attachments: usize,
    pub missing_attachment_files: usize,
    pub empty_text: usize,
    pub replies: usize,
    pub thread_originators: usize,
    pub sentinel_sent_at: usize,
    pub attributed_body_errors: usize,
    pub attributed_body_invalid_typedstream: usize,
    pub attributed_body_not_attributed_string: usize,
    pub attributed_body_missing_text: usize,
}

impl MessageInventory {
    pub fn from_messages(messages: &[Message]) -> Self {
        let mut inventory = Self {
            total: messages.len(),
            ..Self::default()
        };

        for message in messages {
            inventory.count_content(&message.content);
            inventory.count_envelope(message);

            if message_text(&message.content).is_none_or(|text| text.trim().is_empty()) {
                inventory.empty_text += 1;
            }

            if let Some(error) = attributed_body_error(&message.content) {
                inventory.attributed_body_errors += 1;
                match error {
                    AttributedBodyDecodeError::InvalidTypedStream => {
                        inventory.attributed_body_invalid_typedstream += 1;
                    }
                    AttributedBodyDecodeError::NotAttributedString => {
                        inventory.attributed_body_not_attributed_string += 1;
                    }
                    AttributedBodyDecodeError::MissingText => {
                        inventory.attributed_body_missing_text += 1;
                    }
                    AttributedBodyDecodeError::PayloadTooLarge => {}
                }
            }
        }

        inventory
    }

    fn count_content(&mut self, content: &MessageContent) {
        match content {
            MessageContent::Text(_) => self.text += 1,
            MessageContent::Audio(audio) => {
                self.audio += 1;
                self.count_attachments(&audio.attachments);
            }
            MessageContent::Attachment(attachment) => {
                self.attachment += 1;
                self.count_attachments(&attachment.attachments);
            }
            MessageContent::Reaction(_) => self.reaction += 1,
            MessageContent::GroupEvent(_) => self.group_event += 1,
            MessageContent::AppBalloon(balloon) => {
                self.app_balloon += 1;
                match &balloon.kind {
                    AppBalloonKind::Url(_) => self.balloon_url += 1,
                    AppBalloonKind::Photos(_) => self.balloon_photos += 1,
                    AppBalloonKind::Poll(_) => self.balloon_poll += 1,
                    AppBalloonKind::DigitalTouch => self.balloon_digital_touch += 1,
                    AppBalloonKind::Unknown { .. } => self.balloon_unknown += 1,
                }
            }
            MessageContent::SharePlay(_) => self.share_play += 1,
            MessageContent::ShareMyLocation(_) => self.share_my_location += 1,
            MessageContent::System(_) => self.system += 1,
            MessageContent::Unknown(unknown) => {
                self.unknown += 1;
                self.count_attachments(&unknown.attachments);
            }
        }
    }

    fn count_attachments(&mut self, attachments: &[super::model::Attachment]) {
        for attachment in attachments {
            if matches!(attachment.kind, AttachmentKind::Sticker { .. }) {
                self.sticker_attachments += 1;
            }
            if !attachment.present_on_disk {
                self.missing_attachment_files += 1;
            }
        }
    }

    fn count_envelope(&mut self, message: &Message) {
        if message
            .envelope
            .reply_to_guid
            .as_ref()
            .is_some_and(|guid| !guid.as_str().is_empty())
        {
            self.replies += 1;
        }
        if message
            .envelope
            .thread_originator_guid
            .as_ref()
            .is_some_and(|guid| !guid.as_str().is_empty())
        {
            self.thread_originators += 1;
        }
        if message.envelope.sent_at.is_none() {
            self.sentinel_sent_at += 1;
        }
    }
}

impl fmt::Display for MessageInventory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "message inventory ({} total)", self.total)?;
        writeln!(f, "  content:")?;
        writeln!(f, "    text: {}", self.text)?;
        writeln!(f, "    audio: {}", self.audio)?;
        writeln!(f, "    attachment: {}", self.attachment)?;
        writeln!(f, "    reaction: {}", self.reaction)?;
        writeln!(f, "    group_event: {}", self.group_event)?;
        writeln!(f, "    app_balloon: {}", self.app_balloon)?;
        writeln!(f, "      url: {}", self.balloon_url)?;
        writeln!(f, "      photos: {}", self.balloon_photos)?;
        writeln!(f, "      poll: {}", self.balloon_poll)?;
        writeln!(f, "      digital_touch: {}", self.balloon_digital_touch)?;
        writeln!(f, "      unknown: {}", self.balloon_unknown)?;
        writeln!(f, "    share_play: {}", self.share_play)?;
        writeln!(f, "    share_my_location: {}", self.share_my_location)?;
        writeln!(f, "    system: {}", self.system)?;
        writeln!(f, "    unknown: {}", self.unknown)?;
        writeln!(f, "  hygiene:")?;
        writeln!(f, "    empty_text: {}", self.empty_text)?;
        writeln!(f, "    replies: {}", self.replies)?;
        writeln!(f, "    thread_originators: {}", self.thread_originators)?;
        writeln!(f, "    sentinel_sent_at: {}", self.sentinel_sent_at)?;
        writeln!(f, "    sticker_attachments: {}", self.sticker_attachments)?;
        writeln!(
            f,
            "    missing_attachment_files: {}",
            self.missing_attachment_files
        )?;
        writeln!(
            f,
            "    attributed_body_errors: {}",
            self.attributed_body_errors
        )?;
        writeln!(
            f,
            "      invalid_typedstream: {}",
            self.attributed_body_invalid_typedstream
        )?;
        writeln!(
            f,
            "      not_attributed_string: {}",
            self.attributed_body_not_attributed_string
        )?;
        writeln!(
            f,
            "      missing_text: {}",
            self.attributed_body_missing_text
        )
    }
}

fn message_text(content: &MessageContent) -> Option<&str> {
    match content {
        MessageContent::Text(message) => message.body.text.as_deref(),
        MessageContent::Audio(message) => message.body.text.as_deref(),
        MessageContent::Attachment(message) => message.body.text.as_deref(),
        MessageContent::AppBalloon(message) => message.text.as_deref(),
        MessageContent::SharePlay(message) => message.text.as_deref(),
        MessageContent::System(message) => message.text.as_deref(),
        MessageContent::Unknown(message) => message.text.as_deref(),
        MessageContent::Reaction(_)
        | MessageContent::GroupEvent(_)
        | MessageContent::ShareMyLocation(_) => None,
    }
}

fn attributed_body_error(content: &MessageContent) -> Option<AttributedBodyDecodeError> {
    match content {
        MessageContent::Text(message) => message.body.attributed_body_error,
        MessageContent::Audio(message) => message.body.attributed_body_error,
        MessageContent::Attachment(message) => message.body.attributed_body_error,
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::MessageInventory;
    use crate::{
        apple_types::{MessageId, RowId},
        messages::model::{
            AttributedBodyDecodeError, Direction, Message, MessageBody, MessageContent,
            MessageEnvelope, TextMessage, Transport, UnknownMessage,
        },
    };

    fn text_message(text: Option<&str>, error: Option<AttributedBodyDecodeError>) -> Message {
        Message {
            envelope: MessageEnvelope {
                row_id: RowId::new(1),
                guid: MessageId::new("g"),
                direction: Direction::Sent,
                transport: Transport::IMessage,
                sender: None,
                sent_at: None,
                read_at: None,
                edited_at: None,
                retracted_at: None,
                reply_to_guid: Some(MessageId::new("parent")),
                thread_originator_guid: None,
                chat_ids: Vec::new(),
            },
            content: MessageContent::Text(TextMessage {
                body: MessageBody {
                    text: text.map(str::to_owned),
                    runs: Vec::new(),
                    attributed_body_error: error,
                },
                is_forward: false,
                is_auto_reply: false,
                expressive_send_style_id: None,
            }),
        }
    }

    #[test]
    fn counts_unknown_empty_replies_sentinel_and_decode_errors() {
        let messages = vec![
            text_message(Some("hello"), None),
            text_message(None, Some(AttributedBodyDecodeError::InvalidTypedStream)),
            Message {
                envelope: MessageEnvelope {
                    row_id: RowId::new(2),
                    guid: MessageId::new("u"),
                    direction: Direction::Received,
                    transport: Transport::Sms,
                    sender: None,
                    sent_at: None,
                    read_at: None,
                    edited_at: None,
                    retracted_at: None,
                    reply_to_guid: None,
                    thread_originator_guid: None,
                    chat_ids: Vec::new(),
                },
                content: MessageContent::Unknown(UnknownMessage {
                    item_type: 99,
                    associated_message_type: 0,
                    text: None,
                    attachments: Vec::new(),
                }),
            },
        ];

        let inventory = MessageInventory::from_messages(&messages);
        assert_eq!(inventory.total, 3);
        assert_eq!(inventory.text, 2);
        assert_eq!(inventory.unknown, 1);
        assert_eq!(inventory.empty_text, 2);
        assert_eq!(inventory.replies, 2);
        assert_eq!(inventory.sentinel_sent_at, 3);
        assert_eq!(inventory.attributed_body_errors, 1);
        assert_eq!(inventory.attributed_body_invalid_typedstream, 1);
        assert!(inventory.to_string().contains("message inventory"));
    }
}
