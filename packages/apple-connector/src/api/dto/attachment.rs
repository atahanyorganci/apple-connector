use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AttachmentKindDto {
    Sticker { animated: bool },
    Image,
    Video,
    Audio,
    File,
    Unknown,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AttachmentSummaryDto {
    pub guid: String,
    pub original_guid: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uti: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transfer_name: Option<String>,
    #[schema(minimum = 0)]
    pub total_bytes: i64,
    pub kind: AttachmentKindDto,
    pub transfer_complete: bool,
    pub present_on_disk: bool,
    pub hide_attachment: bool,
    /// Safe relative link to attachment metadata.
    #[schema(example = "/v1/attachments/at_0_1234567890ABCDEF")]
    pub metadata_url: String,
    /// Safe relative link to attachment bytes.
    #[schema(example = "/v1/attachments/at_0_1234567890ABCDEF/content")]
    pub content_url: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AttachmentDetailDto {
    pub guid: String,
    pub original_guid: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uti: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transfer_name: Option<String>,
    #[schema(minimum = 0)]
    pub total_bytes: i64,
    pub kind: AttachmentKindDto,
    pub transfer_complete: bool,
    pub present_on_disk: bool,
    pub hide_attachment: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emoji_description: Option<String>,
    #[schema(example = "/v1/attachments/at_0_1234567890ABCDEF")]
    pub metadata_url: String,
    #[schema(example = "/v1/attachments/at_0_1234567890ABCDEF/content")]
    pub content_url: String,
}
