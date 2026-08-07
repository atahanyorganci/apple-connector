use std::collections::{HashMap, HashSet};

use super::model::{Message, ReplyRef, ReplyThread};
use crate::apple_types::MessageId;

/// Build reply threads from flat messages using `reply_to_guid` /
/// `thread_originator_guid`.
///
/// When `thread_originator_guid` is missing, the originator is found by walking
/// the `reply_to_guid` chain.
pub fn build_reply_threads(messages: &[&Message]) -> Vec<ReplyThread> {
    let by_guid: HashMap<&str, &Message> = messages
        .iter()
        .map(|message| (message.envelope.guid.as_str(), *message))
        .collect();

    let mut threads: HashMap<MessageId, Vec<ReplyRef>> = HashMap::new();

    for message in messages {
        let Some(reply_to) = message.envelope.reply_to_guid.as_ref() else {
            continue;
        };
        if reply_to.as_str().is_empty() {
            continue;
        }

        let Some(originator) = resolve_originator(message, &by_guid) else {
            continue;
        };

        threads.entry(originator).or_default().push(ReplyRef {
            guid: message.envelope.guid.clone(),
            reply_to_guid: reply_to.clone(),
        });
    }

    let mut threads: Vec<ReplyThread> = threads
        .into_iter()
        .map(|(originator_guid, mut replies)| {
            replies.sort_by(|left, right| left.guid.cmp(&right.guid));
            ReplyThread {
                originator_guid,
                replies,
            }
        })
        .collect();
    threads.sort_by(|left, right| left.originator_guid.cmp(&right.originator_guid));
    threads
}

fn resolve_originator(message: &Message, by_guid: &HashMap<&str, &Message>) -> Option<MessageId> {
    if let Some(originator) = message
        .envelope
        .thread_originator_guid
        .as_ref()
        .filter(|guid| !guid.as_str().is_empty())
    {
        return Some(originator.clone());
    }

    let mut current = message.envelope.reply_to_guid.as_ref()?;
    let mut seen = HashSet::new();
    seen.insert(message.envelope.guid.as_str());

    loop {
        if !seen.insert(current.as_str()) {
            return None;
        }
        let Some(parent) = by_guid.get(current.as_str()) else {
            return Some(current.clone());
        };
        if let Some(originator) = parent
            .envelope
            .thread_originator_guid
            .as_ref()
            .filter(|guid| !guid.as_str().is_empty())
        {
            return Some(originator.clone());
        }
        match parent.envelope.reply_to_guid.as_ref() {
            Some(next) if !next.as_str().is_empty() => current = next,
            _ => return Some(parent.envelope.guid.clone()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::build_reply_threads;
    use crate::{
        apple_types::{MessageId, RowId},
        messages::model::{
            Direction, Message, MessageContent, MessageEnvelope, ReplyRef, ReplyThread,
            TextMessage, Transport,
        },
    };

    fn message(guid: &str, reply_to: Option<&str>, originator: Option<&str>) -> Message {
        Message {
            envelope: MessageEnvelope {
                row_id: RowId::new(0),
                guid: MessageId::new(guid),
                direction: Direction::Sent,
                transport: Transport::IMessage,
                sender: None,
                sent_at: None,
                read_at: None,
                edited_at: None,
                retracted_at: None,
                reply_to_guid: reply_to.map(MessageId::new),
                thread_originator_guid: originator.map(MessageId::new),
                chat_ids: Vec::new(),
            },
            content: MessageContent::Text(TextMessage {
                body: crate::messages::model::MessageBody {
                    text: Some(guid.to_owned()),
                    runs: Vec::new(),
                    attributed_body_error: None,
                },
                is_forward: false,
                is_auto_reply: false,
                expressive_send_style_id: None,
            }),
        }
    }

    #[test]
    fn builds_thread_from_explicit_originator() {
        let messages = [
            message("root", None, None),
            message("r1", Some("root"), Some("root")),
            message("r2", Some("r1"), Some("root")),
        ];

        let message_refs: Vec<&Message> = messages.iter().collect();
        assert_eq!(
            build_reply_threads(&message_refs),
            vec![ReplyThread {
                originator_guid: MessageId::new("root"),
                replies: vec![
                    ReplyRef {
                        guid: MessageId::new("r1"),
                        reply_to_guid: MessageId::new("root"),
                    },
                    ReplyRef {
                        guid: MessageId::new("r2"),
                        reply_to_guid: MessageId::new("r1"),
                    },
                ],
            }]
        );
    }

    #[test]
    fn walks_reply_chain_when_originator_missing() {
        // Matches the common chat.db shape: reply_to set, thread_originator unset.
        let messages = [
            message("root", None, None),
            message("r1", Some("root"), None),
            message("r2", Some("r1"), None),
        ];

        let message_refs: Vec<&Message> = messages.iter().collect();
        assert_eq!(
            build_reply_threads(&message_refs),
            vec![ReplyThread {
                originator_guid: MessageId::new("root"),
                replies: vec![
                    ReplyRef {
                        guid: MessageId::new("r1"),
                        reply_to_guid: MessageId::new("root"),
                    },
                    ReplyRef {
                        guid: MessageId::new("r2"),
                        reply_to_guid: MessageId::new("r1"),
                    },
                ],
            }]
        );
    }

    #[test]
    fn ignores_messages_without_reply_to() {
        let messages = [message("alone", None, None)];
        let message_refs: Vec<&Message> = messages.iter().collect();
        assert!(build_reply_threads(&message_refs).is_empty());
    }
}
