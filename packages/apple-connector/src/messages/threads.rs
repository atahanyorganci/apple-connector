use std::collections::{HashMap, HashSet};

use super::model::{Message, ReplyRef, ReplyThread};

/// Build reply threads from flat messages using `reply_to_guid` /
/// `thread_originator_guid`.
///
/// When `thread_originator_guid` is missing, the originator is found by walking
/// the `reply_to_guid` chain.
pub fn build_reply_threads(messages: &[Message]) -> Vec<ReplyThread> {
    let by_guid: HashMap<&str, &Message> = messages
        .iter()
        .map(|message| (message.envelope.guid.as_str(), message))
        .collect();

    let mut threads: HashMap<String, Vec<ReplyRef>> = HashMap::new();

    for message in messages {
        let Some(reply_to) = message.envelope.reply_to_guid.as_deref() else {
            continue;
        };
        if reply_to.is_empty() {
            continue;
        }

        let Some(originator) = resolve_originator(message, &by_guid) else {
            continue;
        };

        threads.entry(originator).or_default().push(ReplyRef {
            guid: message.envelope.guid.clone(),
            reply_to_guid: reply_to.to_owned(),
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

fn resolve_originator(message: &Message, by_guid: &HashMap<&str, &Message>) -> Option<String> {
    if let Some(originator) = message
        .envelope
        .thread_originator_guid
        .as_deref()
        .filter(|guid| !guid.is_empty())
    {
        return Some(originator.to_owned());
    }

    let mut current = message.envelope.reply_to_guid.as_deref()?;
    let mut seen = HashSet::new();
    seen.insert(message.envelope.guid.as_str());

    loop {
        if !seen.insert(current) {
            return None;
        }
        let Some(parent) = by_guid.get(current) else {
            return Some(current.to_owned());
        };
        if let Some(originator) = parent
            .envelope
            .thread_originator_guid
            .as_deref()
            .filter(|guid| !guid.is_empty())
        {
            return Some(originator.to_owned());
        }
        match parent.envelope.reply_to_guid.as_deref() {
            Some(next) if !next.is_empty() => current = next,
            _ => return Some(parent.envelope.guid.clone()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::build_reply_threads;
    use crate::messages::model::{
        Direction, Message, MessageContent, MessageEnvelope, ReplyRef, ReplyThread, TextMessage,
        Transport,
    };

    fn message(guid: &str, reply_to: Option<&str>, originator: Option<&str>) -> Message {
        Message {
            envelope: MessageEnvelope {
                row_id: 0,
                guid: guid.to_owned(),
                direction: Direction::Sent,
                transport: Transport::IMessage,
                sender: None,
                sent_at: None,
                read_at: None,
                edited_at: None,
                retracted_at: None,
                reply_to_guid: reply_to.map(str::to_owned),
                thread_originator_guid: originator.map(str::to_owned),
                chat_ids: Vec::new(),
            },
            content: MessageContent::Text(TextMessage {
                body: crate::messages::model::MessageBody {
                    text: Some(guid.to_owned()),
                    runs: Vec::new(),
                },
                is_forward: false,
                is_auto_reply: false,
                expressive_send_style_id: None,
            }),
        }
    }

    #[test]
    fn builds_thread_from_explicit_originator() {
        let messages = vec![
            message("root", None, None),
            message("r1", Some("root"), Some("root")),
            message("r2", Some("r1"), Some("root")),
        ];

        assert_eq!(
            build_reply_threads(&messages),
            vec![ReplyThread {
                originator_guid: "root".to_owned(),
                replies: vec![
                    ReplyRef {
                        guid: "r1".to_owned(),
                        reply_to_guid: "root".to_owned(),
                    },
                    ReplyRef {
                        guid: "r2".to_owned(),
                        reply_to_guid: "r1".to_owned(),
                    },
                ],
            }]
        );
    }

    #[test]
    fn walks_reply_chain_when_originator_missing() {
        // Matches the common chat.db shape: reply_to set, thread_originator unset.
        let messages = vec![
            message("root", None, None),
            message("r1", Some("root"), None),
            message("r2", Some("r1"), None),
        ];

        assert_eq!(
            build_reply_threads(&messages),
            vec![ReplyThread {
                originator_guid: "root".to_owned(),
                replies: vec![
                    ReplyRef {
                        guid: "r1".to_owned(),
                        reply_to_guid: "root".to_owned(),
                    },
                    ReplyRef {
                        guid: "r2".to_owned(),
                        reply_to_guid: "r1".to_owned(),
                    },
                ],
            }]
        );
    }

    #[test]
    fn ignores_messages_without_reply_to() {
        let messages = vec![message("alone", None, None)];
        assert!(build_reply_threads(&messages).is_empty());
    }
}
