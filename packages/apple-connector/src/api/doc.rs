use utoipa::{Modify, OpenApi};

use super::{
    dto::{
        attachment::{AttachmentDetailDto, AttachmentKindDto, AttachmentSummaryDto},
        calendar::{
            AvailabilityDto, CalendarAccountDto, CalendarAccountPageDto, CalendarDetailDto,
            CalendarPageDto, CalendarSummaryDto, CreateEventRequest, DeleteEventParams,
            EventAlarmDto, EventAttachmentDetailDto, EventAttachmentSummaryDto, EventClassDto,
            EventDetailDto, EventLocationDto, EventPageDto, EventParticipantDto, EventSpanDto,
            EventStatusDto, EventStatusInputDto, EventSummaryDto, InvitationStatusDto,
            PrivacyLevelDto, RecurrenceRuleDto, StoreTypeDto, UpdateEventParams,
            UpdateEventRequest,
        },
        chat::{ChatDetailDto, ChatPageDto, ChatSummaryDto},
        common::{
            ContactsAuthStatusDto, DirectionDto, EventKitAuthStatusDto, HandleDto, HealthStatus,
            HealthStatusDto, TransportDto,
        },
        contacts::{
            ContactAddressDto, ContactDetailDto, ContactEmailDto, ContactPageDto, ContactPhoneDto,
            ContactSocialProfileDto, ContactSummaryDto, ContactUrlDto, ContainerDetailDto,
            ContainerPageDto, ContainerSummaryDto, CreateContactRequest, CreateGroupRequest,
            GroupDetailDto, GroupPageDto, GroupSummaryDto, LabeledStringDto, PostalAddressDto,
            UpdateContactRequest, UpdateGroupRequest,
        },
        content::{
            AppBalloonContentDto, AppBalloonKindDto, AttachmentContentDto, AttributedBodyErrorDto,
            AudioContentDto, GroupActionKindDto, GroupEventContentDto, MessageBodyDto,
            MessageContentDto, OpaquePayloadDto, PhotosBalloonDto, PollBalloonDto, PollOptionDto,
            ReactionActionDto, ReactionContentDto, ReactionKindDto, ShareMyLocationContentDto,
            ShareMyLocationStatusDto, SharePlayContentDto, SystemContentDto, TapbackDto,
            TextContentDto, UnknownContentDto, UrlBalloonDto,
        },
        message::{MessageDetailDto, MessagePageDto, MessageSummaryDto},
        note::{
            ChecklistItemDto, EmbeddedObjectDto, FolderKindDto, NoteAttachmentDetailDto,
            NoteAttachmentSummaryDto, NoteBodyDto, NoteContentsFolderDto, NoteContentsPreambleDto,
            NoteDetailDto, NoteFolderDetailDto, NoteFolderPageDto, NoteFolderSummaryDto,
            NotePageDto, NoteRunDto, NoteSummaryDto, ParagraphStyleDto, ParagraphStyleKindDto,
        },
        pagination::PageMetaDto,
        reminder::{
            AlarmDto, AlarmInputDto, AlarmKindDto, CreateReminderRequest, DueDto, DueInputDto,
            LocationInputDto, RecurrenceDto, RecurrenceFrequencyDto, RecurrenceInputDto,
            ReminderAttachmentDetailDto, ReminderAttachmentKindDto, ReminderAttachmentSummaryDto,
            ReminderDetailDto, ReminderListDetailDto, ReminderListKindDto, ReminderListPageDto,
            ReminderListSummaryDto, ReminderPageDto, ReminderSummaryDto, SectionSummaryDto,
            SmartFilterDto, UpdateReminderRequest,
        },
    },
    error::{ErrorBody, ErrorCode, ErrorResponse},
    hydrate::{
        SyncPendingContactDetailDto, SyncPendingEventDetailDto, SyncPendingReminderDetailDto,
    },
    params::{
        AttachmentGuidPath, CalendarIdPath, ChatIdPath, ConditionalRequestHeaders,
        ContactGroupPath, ContactIdPath, ContactListParams, ContainerIdPath, ContentTypeFilterDto,
        DirectionFilterDto, EventAttachmentIdPath, EventIdPath, EventListParams, GroupIdPath,
        MessageGuidPath, MessageListParams, NoteAttachmentIdPath, NoteFolderIdPath, NoteIdPath,
        NoteListParams, PageParams, RangeRequestHeader, ReminderAttachmentIdPath, ReminderIdPath,
        ReminderListIdPath, ReminderListParams, TransportFilterDto,
    },
};
use crate::apple_types::{
    AttachmentId, CalendarAccountId, CalendarAttachmentId, CalendarId, ChatId, ContactId,
    ContainerId, EventId, GroupId, MessageId, NoteAttachmentId, NoteFolderId, NoteId,
    ReminderAttachmentId, ReminderId, ReminderListId, SectionId, SourceId, UnixTimestamp,
};

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};

        let components = openapi.components.get_or_insert_with(Default::default);
        components.security_schemes.insert(
            "reverse_proxy".to_string(),
            SecurityScheme::Http(
                HttpBuilder::new()
                    .scheme(HttpAuthScheme::Bearer)
                    .bearer_format("JWT")
                    .description(Some(
                        "Authentication is expected to be enforced by an external reverse proxy or firewall. This API does not implement authentication or TLS.",
                    ))
                    .build(),
            ),
        );
    }
}

#[derive(OpenApi)]
#[openapi(
    modifiers(&SecurityAddon),
    info(
        title = "Apple Connector API",
        version = "1.0.0",
        description = "Hybrid HTTP API for Messages.app, Reminders.app, Notes.app, Calendar.app, and Contacts.\n\n\
            **Reads** use live SQLite connections (requires Full Disk Access).\n\n\
            **Writes** for Reminders, Calendar events, and Contacts use EventKit / Contacts framework \
            (requires Reminders, Calendars, and Contacts permissions in System Settings). Unsupported \
            fields return `422`; smart lists, read-only calendars/containers return `403`. Post-save \
            responses hydrate from SQLite with up to 5×100ms retries; `sync_pending: true` indicates \
            the SQLite read path has not caught up yet.\n\n\
            Pagination uses keyset cursors only (default limit 50, maximum 200, newest first). \
            Offsets are not supported.\n\n\
            Authentication, TLS, and network exposure controls are expected to be enforced by an \
            external reverse proxy or firewall. This service does not implement authentication or TLS."
    ),
    servers(
        (url = "/", description = "Same origin as this service (use the host you opened in the browser, e.g. http://127.0.0.1:3000 or http://localhost:3000)")
    ),
    security(
        ("reverse_proxy" = [])
    ),
    tags(
        (name = "health", description = "Health and readiness probes"),
        (name = "chats", description = "Chat listing and chat-scoped messages"),
        (name = "messages", description = "Global message listing and lookup"),
        (name = "attachments", description = "Attachment metadata and byte streaming"),
        (name = "reminder-lists", description = "Reminder list listing and list-scoped reminders"),
        (name = "reminders", description = "Global reminder listing and lookup"),
        (name = "reminder-attachments", description = "Reminder attachment metadata and byte streaming"),
        (name = "note-folders", description = "Note folder listing and folder-scoped notes"),
        (name = "notes", description = "Global note listing and lookup"),
        (name = "note-attachments", description = "Note attachment metadata and byte streaming"),
        (name = "calendars", description = "Calendar account and calendar listing"),
        (name = "events", description = "Global event listing, lookup, and interchange parsing"),
        (name = "event-attachments", description = "Event attachment byte streaming"),
        (name = "containers", description = "Contact container listing"),
        (name = "groups", description = "Contact group listing and membership"),
        (name = "contacts", description = "Global contact listing, lookup, vCard/CardDAV, and mutations"),
        (name = "meta", description = "API metadata and contract export")
    ),
    components(schemas(
        AttachmentId,
        CalendarAccountId,
        CalendarAttachmentId,
        CalendarId,
        ChatId,
        ContactId,
        ContainerId,
        EventId,
        GroupId,
        MessageId,
        NoteAttachmentId,
        NoteFolderId,
        NoteId,
        ReminderAttachmentId,
        ReminderId,
        ReminderListId,
        SectionId,
        SourceId,
        UnixTimestamp,
        ChecklistItemDto,
        EmbeddedObjectDto,
        FolderKindDto,
        NoteAttachmentDetailDto,
        NoteAttachmentIdPath,
        NoteAttachmentSummaryDto,
        NoteBodyDto,
        NoteContentsFolderDto,
        NoteContentsPreambleDto,
        NoteDetailDto,
        NoteFolderDetailDto,
        NoteFolderIdPath,
        NoteFolderPageDto,
        NoteFolderSummaryDto,
        NoteIdPath,
        NoteListParams,
        NotePageDto,
        NoteRunDto,
        NoteSummaryDto,
        ParagraphStyleDto,
        ParagraphStyleKindDto,
        AlarmDto,
        AlarmKindDto,
        AvailabilityDto,
        CalendarAccountDto,
        CalendarAccountPageDto,
        CalendarDetailDto,
        CalendarIdPath,
        CalendarPageDto,
        CalendarSummaryDto,
        DueDto,
        EventAlarmDto,
        EventAttachmentDetailDto,
        EventAttachmentIdPath,
        EventAttachmentSummaryDto,
        EventClassDto,
        EventDetailDto,
        EventIdPath,
        EventListParams,
        EventLocationDto,
        EventPageDto,
        EventParticipantDto,
        EventStatusDto,
        EventSummaryDto,
        InvitationStatusDto,
        PrivacyLevelDto,
        RecurrenceDto,
        RecurrenceRuleDto,
        StoreTypeDto,
        AttachmentDetailDto,
        AttachmentGuidPath,
        AttachmentKindDto,
        AttachmentSummaryDto,
        AttributedBodyErrorDto,
        AppBalloonContentDto,
        AppBalloonKindDto,
        AttachmentContentDto,
        AudioContentDto,
        ChatDetailDto,
        ChatIdPath,
        ChatPageDto,
        ChatSummaryDto,
        ConditionalRequestHeaders,
        ContentTypeFilterDto,
        DirectionFilterDto,
        DirectionDto,
        ErrorBody,
        ErrorCode,
        ErrorResponse,
        GroupActionKindDto,
        GroupEventContentDto,
        HandleDto,
        HealthStatus,
        HealthStatusDto,
        EventKitAuthStatusDto,
        ContactsAuthStatusDto,
        CreateReminderRequest,
        UpdateReminderRequest,
        DueInputDto,
        AlarmInputDto,
        RecurrenceInputDto,
        RecurrenceFrequencyDto,
        LocationInputDto,
        SyncPendingReminderDetailDto,
        CreateEventRequest,
        UpdateEventRequest,
        UpdateEventParams,
        DeleteEventParams,
        EventSpanDto,
        EventStatusInputDto,
        SyncPendingEventDetailDto,
        SyncPendingContactDetailDto,
        ContactAddressDto,
        ContactDetailDto,
        ContactEmailDto,
        ContactGroupPath,
        ContactIdPath,
        ContactListParams,
        ContactPageDto,
        ContactPhoneDto,
        ContactSocialProfileDto,
        ContactSummaryDto,
        ContactUrlDto,
        ContainerDetailDto,
        ContainerIdPath,
        ContainerPageDto,
        ContainerSummaryDto,
        CreateContactRequest,
        CreateGroupRequest,
        GroupDetailDto,
        GroupIdPath,
        GroupPageDto,
        GroupSummaryDto,
        LabeledStringDto,
        PostalAddressDto,
        UpdateContactRequest,
        UpdateGroupRequest,
        MessageBodyDto,
        MessageContentDto,
        MessageDetailDto,
        MessageGuidPath,
        MessageListParams,
        MessagePageDto,
        MessageSummaryDto,
        OpaquePayloadDto,
        PageMetaDto,
        PageParams,
        PhotosBalloonDto,
        PollBalloonDto,
        PollOptionDto,
        RangeRequestHeader,
        ReactionActionDto,
        ReactionContentDto,
        ReactionKindDto,
        ReminderAttachmentDetailDto,
        ReminderAttachmentIdPath,
        ReminderAttachmentKindDto,
        ReminderAttachmentSummaryDto,
        ReminderDetailDto,
        ReminderIdPath,
        ReminderListDetailDto,
        ReminderListIdPath,
        ReminderListKindDto,
        ReminderListPageDto,
        ReminderListParams,
        ReminderListSummaryDto,
        ReminderPageDto,
        ReminderSummaryDto,
        SectionSummaryDto,
        ShareMyLocationContentDto,
        ShareMyLocationStatusDto,
        SharePlayContentDto,
        SmartFilterDto,
        SystemContentDto,
        TapbackDto,
        TextContentDto,
        TransportFilterDto,
        TransportDto,
        UnknownContentDto,
        UrlBalloonDto,
    ))
)]
pub struct ApiDoc;

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use utoipa::{OpenApi, openapi::RefOr};

    use super::ApiDoc;
    use crate::api::router::openapi_contract::contract::{self, build_spec};

    const COMMITTED_OPENAPI: &str = include_str!("../../../../docs/openapi.json");

    fn assert_no_dangling_refs(spec: &utoipa::openapi::OpenApi) {
        let components = spec
            .components
            .as_ref()
            .expect("components section should exist");

        let mut refs = BTreeSet::new();
        collect_refs(&spec.paths, &mut refs);

        for reference in refs {
            let name = reference
                .strip_prefix("#/components/schemas/")
                .or_else(|| reference.strip_prefix("#/components/securitySchemes/"))
                .unwrap_or(reference.as_str());
            assert!(
                components.schemas.contains_key(name)
                    || components.security_schemes.contains_key(name),
                "dangling OpenAPI ref `{reference}`"
            );
        }
    }

    fn collect_refs(value: &impl serde::Serialize, refs: &mut BTreeSet<String>) {
        let json = serde_json::to_value(value).expect("serialize openapi fragment");
        collect_refs_value(&json, refs);
    }

    fn collect_refs_value(value: &serde_json::Value, refs: &mut BTreeSet<String>) {
        match value {
            serde_json::Value::Object(map) => {
                if let Some(serde_json::Value::String(reference)) = map.get("$ref") {
                    refs.insert(reference.clone());
                }
                for nested in map.values() {
                    collect_refs_value(nested, refs);
                }
            }
            serde_json::Value::Array(items) => {
                for nested in items {
                    collect_refs_value(nested, refs);
                }
            }
            _ => {}
        }
    }

    fn operation<'a>(
        spec: &'a utoipa::openapi::OpenApi,
        method: &str,
        path: &str,
    ) -> &'a utoipa::openapi::path::Operation {
        contract::operation(spec, method, path)
    }

    #[test]
    fn api_doc_openapi_is_version_3_1() {
        let spec = ApiDoc::openapi();
        assert!(matches!(
            spec.openapi,
            utoipa::openapi::OpenApiVersion::Version31
        ));
    }

    #[test]
    fn exported_openapi_matches_committed_contract() {
        let generated = build_spec()
            .to_pretty_json()
            .expect("serialize generated openapi");
        assert_eq!(
            generated.trim(),
            COMMITTED_OPENAPI.trim(),
            "docs/openapi.json is stale; rerun export-openapi"
        );
    }

    #[test]
    fn contract_covers_planned_operations() {
        let spec = build_spec();

        for (method, path, operation_id) in contract::operations(&spec) {
            let operation = contract::operation(&spec, &method, &path);
            assert_eq!(
                operation.operation_id.as_deref(),
                Some(operation_id.as_str()),
                "unexpected operationId for `{method} {path}`"
            );
            assert!(
                !operation.summary.as_deref().unwrap_or("").is_empty(),
                "missing summary for `{method} {path}`"
            );
            assert!(
                operation.tags.as_ref().is_some_and(|tags| !tags.is_empty()),
                "missing tags for `{method} {path}`"
            );
        }

        assert!(
            !contract::operations(&spec).is_empty(),
            "OpenAPI spec should expose production operations"
        );
    }

    #[test]
    fn contract_documents_pagination_defaults_and_bounds() {
        let spec = build_spec();
        let json = serde_json::to_value(&spec).expect("spec json");

        let list_chats_limit = json["paths"]["/v1/chats"]["get"]["parameters"]
            .as_array()
            .expect("listChats parameters")
            .iter()
            .find(|parameter| parameter["name"] == "limit")
            .expect("limit parameter")["schema"]
            .as_object()
            .expect("limit schema");
        assert_eq!(list_chats_limit.get("minimum"), Some(&serde_json::json!(1)));
        assert_eq!(
            list_chats_limit.get("maximum"),
            Some(&serde_json::json!(200))
        );
        assert_eq!(
            list_chats_limit.get("default"),
            Some(&serde_json::json!(50))
        );

        let page_meta_limit = json["components"]["schemas"]["PageMetaDto"]["properties"]["limit"]
            .as_object()
            .expect("PageMetaDto.limit");
        assert_eq!(page_meta_limit.get("minimum"), Some(&serde_json::json!(1)));
        assert_eq!(
            page_meta_limit.get("maximum"),
            Some(&serde_json::json!(200))
        );

        let serialized = serde_json::to_string(&json).expect("serialize spec");
        assert!(
            !json["paths"]
                .as_object()
                .expect("paths")
                .values()
                .flat_map(|path| path.as_object())
                .flat_map(|path| path.values())
                .flat_map(|operation| operation["parameters"].as_array())
                .flatten()
                .any(|parameter| parameter["name"] == "offset"),
            "offset pagination must not appear in the contract"
        );
        assert!(
            !serialized.contains("\"offset\""),
            "contract must not define an offset parameter"
        );
    }

    #[test]
    fn contract_documents_binary_attachment_responses() {
        let spec = build_spec();
        let operation = operation(&spec, "get", "/v1/attachments/{guid}/content");

        for status in ["200", "206", "304", "404", "416"] {
            assert!(
                operation.responses.responses.contains_key(status),
                "missing status {status} on getAttachmentContent"
            );
        }

        let partial = operation
            .responses
            .responses
            .get("206")
            .expect("206 response");
        let headers = match partial {
            RefOr::T(response) => &response.headers,
            RefOr::Ref(_) => panic!("unexpected 206 ref"),
        };
        assert!(headers.contains_key("Content-Range"));
        assert!(headers.contains_key("Accept-Ranges"));
    }

    #[test]
    fn contract_documents_list_messages_search_parameters() {
        let spec = build_spec();
        let json = serde_json::to_value(&spec).expect("spec json");
        let parameters = json["paths"]["/v1/messages"]["get"]["parameters"]
            .as_array()
            .expect("listMessages parameters");
        let names: Vec<_> = parameters
            .iter()
            .filter_map(|parameter| parameter["name"].as_str())
            .collect();
        for name in [
            "q",
            "chat_id",
            "sender",
            "before",
            "after",
            "direction",
            "transport",
            "content_type",
            "has_attachments",
        ] {
            assert!(
                names.contains(&name),
                "missing listMessages parameter `{name}`"
            );
        }
    }

    #[test]
    fn contract_documents_structured_errors() {
        let spec = build_spec();
        let components = spec.components.as_ref().expect("components");
        let error_response = components
            .schemas
            .get("ErrorResponse")
            .expect("ErrorResponse schema");
        assert!(matches!(error_response, RefOr::T(_)));

        let list_messages = operation(&spec, "get", "/v1/messages");
        for status in ["400", "503", "500"] {
            assert!(
                list_messages.responses.responses.contains_key(status),
                "listMessages missing {status}"
            );
        }
    }

    #[test]
    fn contract_has_no_dangling_refs() {
        assert_no_dangling_refs(&build_spec());
    }

    #[test]
    fn production_routes_expose_operation_ids() {
        let spec = build_spec();
        let operation_ids: BTreeSet<_> = contract::operations(&spec)
            .into_iter()
            .map(|(_, _, operation_id)| operation_id)
            .collect();
        assert_eq!(operation_ids.len(), contract::operations(&spec).len());
    }
}
