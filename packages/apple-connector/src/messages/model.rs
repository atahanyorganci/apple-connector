use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct Message {
    pub envelope: MessageEnvelope,
    pub content: MessageContent,
}

#[derive(Debug, Clone)]
pub struct MessageEnvelope {
    pub row_id: i64,
    pub guid: String,
    pub direction: Direction,
    pub transport: Transport,
    pub sender: Option<Handle>,
    pub sent_at: Option<DateTime<Utc>>,
    pub read_at: Option<DateTime<Utc>>,
    pub edited_at: Option<DateTime<Utc>>,
    pub retracted_at: Option<DateTime<Utc>>,
    pub reply_to_guid: Option<String>,
    pub thread_originator_guid: Option<String>,
    /// Chats this message belongs to (`chat_message_join`).
    pub chat_ids: Vec<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Sent,
    Received,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Transport {
    IMessage,
    Sms,
    Rcs,
    Unknown(String),
}

impl Transport {
    pub fn from_service(service: Option<&str>) -> Self {
        match service {
            Some("iMessage") => Self::IMessage,
            Some("SMS") => Self::Sms,
            Some("RCS") => Self::Rcs,
            Some(other) => Self::Unknown(other.to_owned()),
            None => Self::Unknown(String::new()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Handle {
    pub id: String,
    pub service: String,
}

#[derive(Debug, Clone)]
pub enum MessageContent {
    Text(TextMessage),
    Audio(AudioMessage),
    Attachment(AttachmentMessage),
    Reaction(Reaction),
    GroupEvent(GroupEvent),
    AppBalloon(AppBalloon),
    SharePlay(SharePlayMessage),
    ShareMyLocation(ShareMyLocationMessage),
    System(SystemMessage),
    Unknown(UnknownMessage),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageBody {
    pub text: Option<String>,
    pub runs: Vec<AttributedRun>,
    /// Set when `attributedBody` was present but could not be decoded.
    pub attributed_body_error: Option<AttributedBodyDecodeError>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttributedBodyDecodeError {
    InvalidTypedStream,
    NotAttributedString,
    MissingText,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttributedRun {
    /// UTF-8 byte offset into [`MessageBody::text`].
    pub start: usize,
    /// UTF-8 byte offset into [`MessageBody::text`] (exclusive).
    pub end: usize,
    /// `__kIMMessagePartAttributeName` when present.
    pub part: Option<i64>,
    pub attributes: Vec<BodyAttribute>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BodyAttribute {
    Link {
        url: String,
        is_rich: bool,
    },
    Mention(String),
    FileTransfer {
        guid: String,
        inline_sticker: bool,
    },
    PhoneNumber,
    DataDetected,
    CalendarEvent,
    Breadcrumb {
        marker: Option<String>,
        flags: Option<i64>,
    },
    Unknown(String),
}

#[derive(Debug, Clone)]
pub struct TextMessage {
    pub body: MessageBody,
    pub is_forward: bool,
    pub is_auto_reply: bool,
    pub expressive_send_style_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AudioMessage {
    pub body: MessageBody,
    pub attachments: Vec<Attachment>,
}

#[derive(Debug, Clone)]
pub struct AttachmentMessage {
    pub body: MessageBody,
    pub attachments: Vec<Attachment>,
}

#[derive(Debug, Clone)]
pub struct Attachment {
    pub guid: String,
    pub original_guid: String,
    /// Path as stored in `attachment.filename` (often `~/Library/Messages/...`).
    pub filename: Option<String>,
    /// `filename` with `~` expanded when possible.
    pub resolved_path: Option<String>,
    pub uti: Option<String>,
    pub mime_type: Option<String>,
    pub transfer_name: Option<String>,
    pub total_bytes: i64,
    pub kind: AttachmentKind,
    /// Raw `attachment.transfer_state` (Apple commonly uses `5` for finished).
    pub transfer_state: i64,
    pub transfer_complete: bool,
    /// True when `resolved_path` exists on disk.
    pub present_on_disk: bool,
    pub hide_attachment: bool,
    pub emoji_description: Option<String>,
    /// Linked `__kIMFileTransferGUIDAttributeName` run from attributedBody, if any.
    pub body_reference: Option<AttachmentBodyRef>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttachmentKind {
    Sticker { animated: bool },
    Image,
    Video,
    Audio,
    File,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttachmentBodyRef {
    pub part: Option<i64>,
    pub inline_sticker: bool,
}

#[derive(Debug, Clone)]
pub struct Reaction {
    pub target_guid: Option<String>,
    pub kind: ReactionKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReactionKind {
    Tapback(Tapback, ReactionAction),
    ApplePay,
    Unknown(i64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tapback {
    Love,
    Like,
    Dislike,
    Laugh,
    Emphasize,
    Question,
    Unknown(i64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReactionAction {
    Added,
    Removed,
}

#[derive(Debug, Clone)]
pub struct GroupEvent {
    pub action: GroupActionKind,
    pub title: Option<String>,
    pub actor: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GroupActionKind {
    ParticipantAdded,
    ParticipantRemoved,
    NameChange,
    ParticipantLeft,
    GroupIconChanged,
    GroupIconRemoved,
    ChatBackgroundChanged,
    ChatBackgroundRemoved,
    PhoneNumberChanged,
    Unknown {
        item_type: i64,
        group_action_type: i64,
    },
}

#[derive(Debug, Clone)]
pub struct ShareMyLocationMessage {
    pub status: ShareMyLocationStatus,
    pub other_handle: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShareMyLocationStatus {
    Started,
    Stopped,
}

#[derive(Debug, Clone)]
pub struct AppBalloon {
    pub bundle_id: String,
    pub text: Option<String>,
    pub kind: AppBalloonKind,
}

#[derive(Debug, Clone)]
pub enum AppBalloonKind {
    Url(UrlBalloon),
    Photos(PhotosBalloon),
    Poll(PollBalloon),
    DigitalTouch,
    Unknown { payload_data: Option<Vec<u8>> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UrlBalloon {
    pub url: Option<String>,
    pub original_url: Option<String>,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub site_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhotosBalloon {
    pub url: Option<String>,
    pub app_name: Option<String>,
    pub ldtext: Option<String>,
    pub caption: Option<String>,
    pub subcaption: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PollBalloon {
    pub title: Option<String>,
    pub creator_handle: Option<String>,
    pub options: Vec<PollOption>,
    pub ldtext: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PollOption {
    pub text: String,
    pub option_id: String,
    pub creator_handle: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SharePlayMessage {
    pub balloon_bundle_id: Option<String>,
    pub payload_data: Option<Vec<u8>>,
    pub text: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SystemMessage {
    pub is_system: bool,
    pub is_service: bool,
    pub text: Option<String>,
}

#[derive(Debug, Clone)]
pub struct UnknownMessage {
    pub item_type: i64,
    pub associated_message_type: i64,
    pub text: Option<String>,
    pub attachments: Vec<Attachment>,
}

#[derive(Debug, Clone)]
pub struct Chat {
    pub row_id: i64,
    pub guid: String,
    pub identifier: Option<String>,
    pub display_name: Option<String>,
    pub room_name: Option<String>,
    pub transport: Transport,
    /// Apple `chat.style == 43` for group chats.
    pub is_group: bool,
    pub participants: Vec<Handle>,
    pub messages: Vec<Message>,
    pub reply_threads: Vec<ReplyThread>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplyThread {
    /// Root message GUID for the thread.
    pub originator_guid: String,
    /// Replies in this thread (excluding the originator), parented by `reply_to_guid`.
    pub replies: Vec<ReplyRef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplyRef {
    pub guid: String,
    pub reply_to_guid: String,
}
