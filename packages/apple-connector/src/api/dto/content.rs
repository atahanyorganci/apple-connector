use serde::Serialize;
use utoipa::ToSchema;

use super::attachment::AttachmentSummaryDto;

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct OpaquePayloadDto {
    pub present: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(minimum = 0, example = 1024)]
    pub size_bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MessageBodyDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attributed_body_error: Option<AttributedBodyErrorDto>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AttributedBodyErrorDto {
    InvalidTypedStream,
    NotAttributedString,
    MissingText,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MessageContentDto {
    Text(TextContentDto),
    Audio(AudioContentDto),
    Attachment(AttachmentContentDto),
    Reaction(ReactionContentDto),
    GroupEvent(GroupEventContentDto),
    AppBalloon(AppBalloonContentDto),
    SharePlay(SharePlayContentDto),
    ShareMyLocation(ShareMyLocationContentDto),
    System(SystemContentDto),
    Unknown(UnknownContentDto),
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct TextContentDto {
    pub body: MessageBodyDto,
    pub is_forward: bool,
    pub is_auto_reply: bool,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AudioContentDto {
    pub body: MessageBodyDto,
    pub attachments: Vec<AttachmentSummaryDto>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AttachmentContentDto {
    pub body: MessageBodyDto,
    pub attachments: Vec<AttachmentSummaryDto>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ReactionContentDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_guid: Option<String>,
    pub kind: ReactionKindDto,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ReactionKindDto {
    Tapback {
        tapback: TapbackDto,
        action: ReactionActionDto,
    },
    ApplePay,
    Unknown {
        code: i64,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum TapbackDto {
    Love,
    Like,
    Dislike,
    Laugh,
    Emphasize,
    Question,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReactionActionDto {
    Added,
    Removed,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct GroupEventContentDto {
    pub action: GroupActionKindDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actor: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum GroupActionKindDto {
    ParticipantAdded,
    ParticipantRemoved,
    NameChange,
    ParticipantLeft,
    GroupIconChanged,
    GroupIconRemoved,
    ChatBackgroundChanged,
    ChatBackgroundRemoved,
    PhoneNumberChanged,
    Unknown,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AppBalloonContentDto {
    pub bundle_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    pub kind: AppBalloonKindDto,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AppBalloonKindDto {
    Url(UrlBalloonDto),
    Photos(PhotosBalloonDto),
    Poll(PollBalloonDto),
    DigitalTouch,
    Unknown { payload: OpaquePayloadDto },
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct UrlBalloonDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PhotosBalloonDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caption: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PollBalloonDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub options: Vec<PollOptionDto>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PollOptionDto {
    pub text: String,
    pub option_id: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SharePlayContentDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    pub payload: OpaquePayloadDto,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ShareMyLocationContentDto {
    pub status: ShareMyLocationStatusDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub other_handle: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ShareMyLocationStatusDto {
    Started,
    Stopped,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SystemContentDto {
    pub is_system: bool,
    pub is_service: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct UnknownContentDto {
    pub item_type: i64,
    pub associated_message_type: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    pub attachments: Vec<AttachmentSummaryDto>,
}
