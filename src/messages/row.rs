#[derive(Debug)]
pub struct MessageRow {
    pub row_id: i64,
    pub guid: String,
    pub text: Option<String>,
    pub attributed_body: Option<Vec<u8>>,
    pub service: Option<String>,
    pub sent_at: Option<String>,
    pub read_at: Option<String>,
    pub edited_at: Option<String>,
    pub retracted_at: Option<String>,
    pub is_from_me: bool,
    pub sender_id: Option<String>,
    pub sender_service: Option<String>,
    pub item_type: i64,
    pub associated_message_guid: Option<String>,
    pub associated_message_type: i64,
    pub group_action_type: i64,
    pub group_title: Option<String>,
    pub other_handle_id: Option<String>,
    pub balloon_bundle_id: Option<String>,
    pub payload_data: Option<Vec<u8>>,
    pub is_audio_message: bool,
    pub cache_has_attachments: bool,
    pub is_forward: bool,
    pub is_auto_reply: bool,
    pub is_system_message: bool,
    pub is_service_message: bool,
    pub reply_to_guid: Option<String>,
    pub thread_originator_guid: Option<String>,
    pub expressive_send_style_id: Option<String>,
}

impl MessageRow {
    pub fn has_attributed_body(&self) -> bool {
        self.attributed_body
            .as_ref()
            .is_some_and(|body| !body.is_empty())
    }
}

#[derive(Debug)]
pub struct AttachmentRow {
    pub message_id: i64,
    pub guid: String,
    pub filename: Option<String>,
    pub uti: Option<String>,
    pub mime_type: Option<String>,
    pub transfer_name: Option<String>,
    pub total_bytes: i64,
    pub is_sticker: bool,
}
