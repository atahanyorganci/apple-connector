/**
 * Generated from `docs/openapi.json` — do not edit by hand.
 *
 * Regenerate with `pnpm generate` after updating the contract via
 * `cargo run -p apple-connector --bin export-openapi docs/openapi.json`.
 *
 * API version: 1.0.0
 */

export type AlarmDto = {
	decode_error?: string | null;
	kind: AlarmKindDto;
	latitude?: number | null;
	longitude?: number | null;
	radius?: number | null;
	time_interval?: number | null;
	title?: string | null;
};

export type AlarmInputDto = {
	at?: null | UnixTimestamp;
	kind: AlarmKindDto;
	offset_seconds?: number | null;
};

export type AlarmKindDto = "absolute" | "relative" | "location" | "unknown";

export type AppBalloonContentDto = {
	bundle_id: string;
	kind: AppBalloonKindDto;
	text?: string | null;
};

export type AppBalloonKindDto = UrlBalloonDto & {
	type: "url";
} | PhotosBalloonDto & {
	type: "photos";
} | PollBalloonDto & {
	type: "poll";
} | {
	type: "digital_touch";
} | {
	payload: OpaquePayloadDto;
	type: "unknown";
};

export type AttachmentContentDto = {
	attachments: AttachmentSummaryDto[];
	body: MessageBodyDto;
};

export type AttachmentDetailDto = {
	content_url: string;
	emoji_description?: string | null;
	guid: AttachmentId;
	hide_attachment: boolean;
	kind: AttachmentKindDto;
	metadata_url: string;
	mime_type?: string | null;
	original_guid: AttachmentId;
	present_on_disk: boolean;
	total_bytes: number;
	transfer_complete: boolean;
	transfer_name?: string | null;
	uti?: string | null;
};

export type AttachmentGuidPath = {
	/** Attachment GUID. */
	guid: AttachmentId;
};

/** Stable identifier for an attachment (its GUID). */
export type AttachmentId = string;

export type AttachmentKindDto = {
	sticker: {
		animated: boolean;
	};
} | "image" | "video" | "audio" | "file" | "unknown";

export type AttachmentSummaryDto = {
	/** Safe relative link to attachment bytes. */
	content_url: string;
	guid: AttachmentId;
	hide_attachment: boolean;
	kind: AttachmentKindDto;
	/** Safe relative link to attachment metadata. */
	metadata_url: string;
	mime_type?: string | null;
	original_guid: AttachmentId;
	present_on_disk: boolean;
	total_bytes: number;
	transfer_complete: boolean;
	transfer_name?: string | null;
	uti?: string | null;
};

export type AttributedBodyErrorDto = "invalid_typed_stream" | "not_attributed_string" | "missing_text" | "payload_too_large";

export type AudioContentDto = {
	attachments: AttachmentSummaryDto[];
	body: MessageBodyDto;
};

export type AvailabilityDto = "busy" | "free" | "tentative" | "unavailable" | {
	unknown: {
		code: number;
	};
};

export type CalendarAccountDto = {
	disabled: boolean;
	id?: null | CalendarAccountId;
	name?: string | null;
	row_id: number;
	store_type: StoreTypeDto;
};

/** Stable identifier for a calendar account (store external id). */
export type CalendarAccountId = string;

export type CalendarAccountPageDto = {
	items: CalendarAccountDto[];
};

/** Stable identifier for a calendar event attachment (its UUID). */
export type CalendarAttachmentId = string;

export type CalendarDetailDto = CalendarSummaryDto & ({
	notes?: string | null;
	sharing_status?: number | null;
});

/** Stable identifier for a calendar (its UUID). */
export type CalendarId = string;

export type CalendarIdPath = {
	calendar_id: CalendarId;
};

export type CalendarPageDto = {
	items: CalendarSummaryDto[];
	page: PageMetaDto;
};

export type CalendarSummaryDto = {
	account_id?: null | CalendarAccountId;
	account_row_id: number;
	color?: string | null;
	id: CalendarId;
	row_id: number;
	title?: string | null;
};

export type ChatDetailDto = {
	display_name?: string | null;
	guid: string;
	id: ChatId;
	identifier?: string | null;
	is_group: boolean;
	participants: HandleDto[];
	room_name?: string | null;
	transport: TransportDto;
};

/**
 * Internal chat row identifier.
 *
 * Serialized as a JSON integer, matching the `chat.ROWID` primary key exposed
 * by the Messages routes.
 */
export type ChatId = number;

export type ChatIdPath = {
	/** Internal chat row identifier. */
	chat_id: ChatId;
};

export type ChatPageDto = {
	items: ChatSummaryDto[];
	page: PageMetaDto;
};

export type ChatSummaryDto = {
	display_name?: string | null;
	guid: string;
	id: ChatId;
	is_group: boolean;
	participant_count: number;
	transport: TransportDto;
};

export type ChecklistItemDto = {
	done: boolean;
	id: string;
	text: string;
};

export type ConditionalRequestHeaders = {
	/** Timestamp validator for conditional GET/HEAD requests. */
	if_modified_since?: string | null;
	/** Validator for conditional GET/HEAD requests. */
	if_none_match?: string | null;
};

export type ContactAddressDto = {
	city?: string | null;
	country?: string | null;
	id: string;
	is_primary: boolean;
	label?: string | null;
	postal_code?: string | null;
	state?: string | null;
	street?: string | null;
};

export type ContactDetailDto = ContactSummaryDto & ({
	addresses: ContactAddressDto[];
	birthday?: null | UnixTimestamp;
	creation_date?: null | UnixTimestamp;
	department?: string | null;
	emails: ContactEmailDto[];
	group_ids: GroupId[];
	has_photo: boolean;
	job_title?: string | null;
	middle_name?: string | null;
	nickname?: string | null;
	note?: string | null;
	phones: ContactPhoneDto[];
	social_profiles: ContactSocialProfileDto[];
	urls: ContactUrlDto[];
});

export type ContactEmailDto = {
	address: string;
	id: string;
	is_primary: boolean;
	label?: string | null;
};

export type ContactGroupPath = {
	contact_id: ContactId;
	group_id: GroupId;
};

/** Stable identifier for a contact (its UUID). */
export type ContactId = string;

export type ContactIdPath = {
	contact_id: ContactId;
};

export type ContactListParams = {
	container_id?: string | null;
	cursor?: string | null;
	group_id?: string | null;
	limit?: number | null;
	q?: string | null;
};

export type ContactPageDto = {
	items: ContactSummaryDto[];
	page: PageMetaDto;
};

export type ContactPhoneDto = {
	id: string;
	is_primary: boolean;
	label?: string | null;
	number: string;
};

export type ContactSocialProfileDto = {
	id: string;
	is_primary: boolean;
	label?: string | null;
	service?: string | null;
	url?: string | null;
	username?: string | null;
};

export type ContactSummaryDto = {
	container_id?: null | ContainerId;
	display_name?: string | null;
	first_name?: string | null;
	id: ContactId;
	last_name?: string | null;
	modification_date?: null | UnixTimestamp;
	organization?: string | null;
	source_id: SourceId;
};

export type ContactUrlDto = {
	id: string;
	is_primary: boolean;
	label?: string | null;
	url: string;
};

export type ContactsAuthStatusDto = "not_determined" | "restricted" | "denied" | "authorized" | "limited" | "unavailable";

/** Stable identifier for a contact container (its UUID). */
export type ContainerId = string;

export type ContainerIdPath = {
	container_id: ContainerId;
};

export type ContainerPageDto = {
	items: ContainerSummaryDto[];
};

/**
 * An AddressBook container.
 *
 * There is no writability flag: the SQLite read path cannot know whether the Contacts framework
 * will accept a write, and the field this DTO used to carry was always `false`. A write to a
 * container the framework refuses answers `403 read_only_container`.
 */
export type ContainerSummaryDto = {
	container_type: number;
	id: ContainerId;
	name?: string | null;
	source_id: SourceId;
};

export type ContentTypeFilterDto = "text" | "audio" | "attachment" | "reaction" | "group_event" | "app_balloon" | "share_play" | "share_my_location" | "system" | "unknown";

export type CreateContactRequest = {
	department_name?: string | null;
	email_addresses?: LabeledStringDto[];
	family_name?: string | null;
	given_name?: string | null;
	job_title?: string | null;
	middle_name?: string | null;
	nickname?: string | null;
	note?: string | null;
	organization_name?: string | null;
	phone_numbers?: LabeledStringDto[];
	postal_addresses?: PostalAddressDto[];
	url_addresses?: LabeledStringDto[];
};

export type CreateEventRequest = {
	alarms?: AlarmInputDto[];
	/**
	 * When true, `start` and `end` are read as the local calendar days containing them, and both
	 * are stored snapped to local midnight. When false, both are stored as exact instants.
	 */
	all_day?: boolean;
	description?: string | null;
	end: UnixTimestamp;
	location?: null | LocationInputDto;
	recurrence?: null | RecurrenceInputDto;
	start: UnixTimestamp;
	status?: null | EventStatusInputDto;
	summary: string;
	url?: string | null;
};

export type CreateGroupRequest = {
	name: string;
};

export type CreateReminderRequest = {
	alarms?: AlarmInputDto[];
	attachments?: ReminderAttachmentId[];
	completed?: boolean | null;
	due?: null | DueInputDto;
	flagged?: boolean | null;
	location?: null | LocationInputDto;
	notes?: string | null;
	parent_id?: null | ReminderId;
	priority?: number | null;
	recurrence?: null | RecurrenceInputDto;
	section_id?: null | SectionId;
	tags?: string[];
	title: string;
	url?: string | null;
};

export type DeleteEventParams = {
	occurrence_start?: null | UnixTimestamp;
	span?: null | EventSpanDto;
};

export type DirectionDto = "sent" | "received";

export type DirectionFilterDto = "sent" | "received";

export type DueDto = {
	all_day: boolean;
	at: UnixTimestamp;
};

export type DueInputDto = {
	/**
	 * When true, the due date is the local calendar day containing `at`, and the time of day is
	 * discarded. When false, `at` is stored as an exact instant.
	 */
	all_day: boolean;
	/** Instant the reminder is due, as UTC Unix seconds. */
	at: UnixTimestamp;
};

export type EmbeddedObjectDto = {
	attachment_identifier?: string | null;
	type_uti?: string | null;
};

export type ErrorBody = {
	code: ErrorCode;
	details?: unknown;
	message: string;
};

/** Stable, unique snake_case error identifiers returned in `error.code`. */
export type ErrorCode = "route_not_found" | "method_not_allowed" | "invalid_cursor" | "invalid_limit" | "invalid_timestamp" | "invalid_parameter" | "unsupported_query_parameter" | "request_timeout" | "query_timeout" | "gateway_timeout" | "internal_error" | "byte_range_not_satisfiable" | "messages_database_unavailable" | "reminders_database_unavailable" | "notes_database_unavailable" | "calendar_database_unavailable" | "contacts_database_unavailable" | "message_not_found" | "chat_not_found" | "message_attachment_not_found" | "message_attachment_unavailable" | "note_not_found" | "note_folder_not_found" | "note_attachment_not_found" | "note_attachment_unavailable" | "reminder_not_found" | "reminder_list_not_found" | "reminder_attachment_not_found" | "reminder_attachment_unavailable" | "smart_list_read_only" | "unsupported_reminder_field" | "calendar_not_found" | "calendar_account_not_found" | "event_not_found" | "event_attachment_not_found" | "event_attachment_unavailable" | "event_end_before_start" | "immutable_event_field" | "unsupported_alarm_kind" | "ambiguous_event_kit_match" | "contact_not_found" | "group_not_found" | "container_not_found" | "contact_photo_not_found" | "read_only_container" | "ambiguous_contacts_match" | "eventkit_access_denied" | "contacts_access_denied" | "eventkit_unavailable" | "contacts_unavailable" | "calendar_read_only" | "sqlite_sync_pending" | "validation_error" | "resource_not_found" | "service_unavailable" | "forbidden" | "conflict" | "unprocessable_entity";

export type ErrorResponse = {
	error: ErrorBody;
};

export type EventAlarmDto = {
	alarm_type: number;
	disabled: boolean;
	id: string;
	trigger_date?: null | UnixTimestamp;
	trigger_interval_seconds?: number | null;
};

export type EventAttachmentDetailDto = EventAttachmentSummaryDto & ({
	local_path?: string | null;
});

export type EventAttachmentIdPath = {
	attachment_id: CalendarAttachmentId;
	event_id: EventId;
};

export type EventAttachmentSummaryDto = {
	file_size?: number | null;
	filename?: string | null;
	format?: string | null;
	id: CalendarAttachmentId;
	row_id: number;
};

export type EventClassDto = "standard" | "birthday" | "special_day" | {
	unknown: {
		code: number;
	};
};

export type EventDetailDto = EventSummaryDto & ({
	alarms: EventAlarmDto[];
	attachments: EventAttachmentSummaryDto[];
	attendees: EventParticipantDto[];
	availability: AvailabilityDto;
	conference_url?: string | null;
	creation_date?: null | UnixTimestamp;
	description?: string | null;
	exception_dates: UnixTimestamp[];
	has_app_link: boolean;
	has_structured_data: boolean;
	invitation_status: InvitationStatusDto;
	last_modified?: null | UnixTimestamp;
	location?: null | EventLocationDto;
	organizer?: null | EventParticipantDto;
	original_start?: null | UnixTimestamp;
	privacy_level: PrivacyLevelDto;
	recurrence?: null | RecurrenceRuleDto;
	series_id?: null | EventId;
	series_row_id?: number | null;
	travel_time_seconds?: number | null;
	url?: string | null;
});

/** Stable identifier for a calendar event (its UUID). */
export type EventId = string;

export type EventIdPath = {
	event_id: EventId;
};

export type EventKitAuthStatusDto = "not_determined" | "restricted" | "denied" | "authorized" | "write_only" | "unavailable";

export type EventListParams = {
	account_id?: string | null;
	calendar_id?: string | null;
	cursor?: string | null;
	end?: number | null;
	include_cancelled?: boolean | null;
	include_hidden?: boolean | null;
	limit?: number | null;
	q?: string | null;
	start?: number | null;
};

export type EventLocationDto = {
	address?: string | null;
	latitude?: number | null;
	longitude?: number | null;
	title?: string | null;
};

export type EventPageDto = {
	items: EventSummaryDto[];
	page: PageMetaDto;
};

export type EventParticipantDto = {
	comment?: string | null;
	email?: string | null;
	id: string;
	is_self: boolean;
	name?: string | null;
	phone_number?: string | null;
	role?: number | null;
	status: InvitationStatusDto;
};

/**
 * How far a change to a recurring event reaches.
 *
 * EventKit implements exactly these two scopes, so the API offers no more than it can deliver.
 */
export type EventSpanDto = "this" | "future";

export type EventStatusDto = "confirmed" | "tentative" | "cancelled" | {
	unknown: {
		code: number;
	};
};

export type EventStatusInputDto = "confirmed" | "tentative" | "cancelled";

export type EventSummaryDto = {
	all_day: boolean;
	calendar_id: CalendarId;
	calendar_row_id: number;
	end?: null | UnixTimestamp;
	event_class: EventClassDto;
	hidden: boolean;
	id: EventId;
	is_recurring: boolean;
	occurrence_end?: null | UnixTimestamp;
	occurrence_start?: null | UnixTimestamp;
	row_id: number;
	start?: null | UnixTimestamp;
	status: EventStatusDto;
	summary?: string | null;
};

export type FolderKindDto = "standard" | "smart" | "deleted";

export type GroupActionKindDto = "participant_added" | "participant_removed" | "name_change" | "participant_left" | "group_icon_changed" | "group_icon_removed" | "chat_background_changed" | "chat_background_removed" | "phone_number_changed" | "unknown";

export type GroupEventContentDto = {
	action: GroupActionKindDto;
	actor?: string | null;
	title?: string | null;
};

/** Stable identifier for a contact group (its UUID). */
export type GroupId = string;

export type GroupIdPath = {
	group_id: GroupId;
};

export type GroupPageDto = {
	items: GroupSummaryDto[];
	page: PageMetaDto;
};

export type GroupSummaryDto = {
	container_id?: null | ContainerId;
	id: GroupId;
	is_smart_group: boolean;
	is_subscribed: boolean;
	name?: string | null;
	source_id: SourceId;
};

export type HandleDto = {
	id: string;
	service: string;
};

export type HealthStatus = "ok" | "unavailable";

export type HealthStatusDto = {
	calendar: HealthStatus;
	contacts: HealthStatus;
	contacts_auth: ContactsAuthStatusDto;
	eventkit_events: EventKitAuthStatusDto;
	eventkit_reminders: EventKitAuthStatusDto;
	messages: HealthStatus;
	notes: HealthStatus;
	reminders: HealthStatus;
};

export type InvitationStatusDto = "unknown" | "accepted" | "declined" | "tentative" | "needs_action" | {
	raw: {
		code: number;
	};
};

export type LabeledStringDto = {
	label?: string | null;
	value: string;
};

export type LocationInputDto = {
	/**
	 * Events only. EventKit cannot store a coordinate on a reminder, so sending this on a
	 * reminder returns 422 rather than dropping it.
	 */
	latitude?: number | null;
	/**
	 * Events only. EventKit cannot store a coordinate on a reminder, so sending this on a
	 * reminder returns 422 rather than dropping it.
	 */
	longitude?: number | null;
	/** Free-text location. Stored for both events and reminders. */
	title?: string | null;
};

export type MessageBodyDto = {
	attributed_body_error?: null | AttributedBodyErrorDto;
	text?: string | null;
};

export type MessageContentDto = TextContentDto & {
	type: "text";
} | AudioContentDto & {
	type: "audio";
} | AttachmentContentDto & {
	type: "attachment";
} | ReactionContentDto & {
	type: "reaction";
} | GroupEventContentDto & {
	type: "group_event";
} | AppBalloonContentDto & {
	type: "app_balloon";
} | SharePlayContentDto & {
	type: "share_play";
} | ShareMyLocationContentDto & {
	type: "share_my_location";
} | SystemContentDto & {
	type: "system";
} | UnknownContentDto & {
	type: "unknown";
};

export type MessageDetailDto = {
	chat_ids: ChatId[];
	content: MessageContentDto;
	direction: DirectionDto;
	edited_at?: null | UnixTimestamp;
	guid: MessageId;
	read_at?: null | UnixTimestamp;
	reply_to_guid?: null | MessageId;
	retracted_at?: null | UnixTimestamp;
	sender?: null | HandleDto;
	sent_at?: null | UnixTimestamp;
	thread_originator_guid?: null | MessageId;
	transport: TransportDto;
};

export type MessageGuidPath = {
	/** Message GUID. */
	guid: MessageId;
};

/** Stable identifier for a message (its GUID). */
export type MessageId = string;

export type MessageListParams = {
	/** Return messages sent strictly after this Unix timestamp (UTC seconds). */
	after?: number | null;
	/** Return messages sent strictly before this Unix timestamp (UTC seconds). */
	before?: number | null;
	chat_id?: null | ChatId;
	content_type?: null | ContentTypeFilterDto;
	/** URL-safe versioned cursor for the next page. Results are ordered newest first. */
	cursor?: string | null;
	direction?: null | DirectionFilterDto;
	/** Restrict results to messages with or without attachments. */
	has_attachments?: boolean | null;
	/** Maximum number of items to return. Defaults to 50. Keyset pagination only; no offsets. */
	limit?: number | null;
	/** Case-insensitive search over plain text and decoded attributed-body text. */
	q?: string | null;
	/** Restrict results to messages from this handle identifier. */
	sender?: string | null;
	transport?: null | TransportFilterDto;
};

export type MessagePageDto = {
	items: MessageSummaryDto[];
	page: PageMetaDto;
};

export type MessageSummaryDto = {
	content: MessageContentDto;
	direction: DirectionDto;
	guid: MessageId;
	sender?: null | HandleDto;
	sent_at?: null | UnixTimestamp;
	transport: TransportDto;
};

export type NoteAttachmentDetailDto = {
	file_size?: number | null;
	filename?: string | null;
	id: NoteAttachmentId;
	modified_at?: null | UnixTimestamp;
	note_id: NoteId;
	row_id: number;
	uti?: string | null;
};

/** Stable identifier for a note attachment (its UUID). */
export type NoteAttachmentId = string;

export type NoteAttachmentIdPath = {
	id: NoteAttachmentId;
};

export type NoteAttachmentSummaryDto = {
	file_size?: number | null;
	filename?: string | null;
	id: NoteAttachmentId;
	row_id: number;
	uti?: string | null;
};

export type NoteBodyDto = {
	checklist_items?: ChecklistItemDto[];
	decode_error?: string | null;
	embedded?: EmbeddedObjectDto[];
	runs?: NoteRunDto[];
	text?: string | null;
};

export type NoteContentsFolderDto = {
	id: NoteFolderId;
	kind: FolderKindDto;
	name?: string | null;
};

/**
 * YAML front matter schema for `GET /v1/notes/{note_id}/contents`.
 *
 * The HTTP response is a single `text/markdown` document; this schema documents
 * the fields serialized between the opening and closing `---` delimiters.
 */
export type NoteContentsPreambleDto = {
	created_at?: null | UnixTimestamp;
	folder?: null | NoteContentsFolderDto;
	has_attachments: boolean;
	has_checklist: boolean;
	id: NoteId;
	is_locked: boolean;
	is_pinned: boolean;
	marked_for_deletion: boolean;
	modified_at?: null | UnixTimestamp;
	schema_version: number;
	tags: string[];
	title: string;
};

export type NoteDetailDto = {
	attachments: NoteAttachmentSummaryDto[];
	body: NoteBodyDto;
	created_at?: null | UnixTimestamp;
	folder_id?: null | NoteFolderId;
	folder_kind: FolderKindDto;
	folder_name?: string | null;
	folder_row_id?: number | null;
	has_attachments: boolean;
	has_checklist: boolean;
	id: NoteId;
	is_locked: boolean;
	is_pinned: boolean;
	marked_for_deletion: boolean;
	modified_at?: null | UnixTimestamp;
	row_id: number;
	snippet?: string | null;
	title: string;
};

export type NoteFolderDetailDto = {
	account_id?: string | null;
	id: NoteFolderId;
	kind: FolderKindDto;
	modified_at?: null | UnixTimestamp;
	parent_id?: null | NoteFolderId;
	parent_row_id?: number | null;
	row_id: number;
	title: string;
};

/** Stable identifier for a note folder (its UUID). */
export type NoteFolderId = string;

export type NoteFolderIdPath = {
	folder_id: NoteFolderId;
};

export type NoteFolderPageDto = {
	items: NoteFolderSummaryDto[];
	page: PageMetaDto;
};

export type NoteFolderSummaryDto = {
	id: NoteFolderId;
	kind: FolderKindDto;
	modified_at?: null | UnixTimestamp;
	parent_id?: null | NoteFolderId;
	row_id: number;
	title: string;
};

/** Stable identifier for a note (its UUID). */
export type NoteId = string;

export type NoteIdPath = {
	note_id: NoteId;
};

export type NoteListParams = {
	cursor?: string | null;
	folder_id?: string | null;
	has_attachments?: boolean | null;
	has_checklist?: boolean | null;
	include_deleted?: boolean | null;
	is_locked?: boolean | null;
	is_pinned?: boolean | null;
	limit?: number | null;
	modified_after?: number | null;
	modified_before?: number | null;
	q?: string | null;
};

export type NotePageDto = {
	items: NoteSummaryDto[];
	page: PageMetaDto;
};

export type NoteRunDto = {
	font_hints?: number | null;
	length: number;
	link?: string | null;
	paragraph_style?: null | ParagraphStyleDto;
	start: number;
};

export type NoteSummaryDto = {
	created_at?: null | UnixTimestamp;
	folder_id?: null | NoteFolderId;
	folder_kind: FolderKindDto;
	folder_name?: string | null;
	folder_row_id?: number | null;
	has_attachments: boolean;
	has_checklist: boolean;
	id: NoteId;
	is_locked: boolean;
	is_pinned: boolean;
	marked_for_deletion: boolean;
	modified_at?: null | UnixTimestamp;
	row_id: number;
	snippet?: string | null;
	title: string;
};

export type OpaquePayloadDto = {
	present: boolean;
	size_bytes?: number | null;
};

export type PageMetaDto = {
	/** Whether additional items exist beyond this page. */
	has_more: boolean;
	/** Applied page size for this response. */
	limit: number;
	/** URL-safe versioned cursor for the next page when `has_more` is true. */
	next_cursor?: string | null;
};

export type PageParams = {
	/** URL-safe versioned cursor for the next page. Results are ordered newest first. */
	cursor?: string | null;
	/** Maximum number of items to return. Defaults to 50. Keyset pagination only; no offsets. */
	limit?: number | null;
};

export type ParagraphStyleDto = {
	done?: boolean | null;
	style: ParagraphStyleKindDto;
	todo_uuid?: string | null;
};

export type ParagraphStyleKindDto = "title" | "heading" | "monospace" | "bullet_list" | "dash_list" | "numbered_list" | "checklist" | "unknown";

export type PhotosBalloonDto = {
	caption?: string | null;
	url?: string | null;
};

export type PollBalloonDto = {
	options: PollOptionDto[];
	title?: string | null;
};

export type PollOptionDto = {
	option_id: string;
	text: string;
};

export type PostalAddressDto = {
	city?: string | null;
	country?: string | null;
	label?: string | null;
	postal_code?: string | null;
	state?: string | null;
	street?: string | null;
};

export type PrivacyLevelDto = "default" | "public" | "private" | {
	unknown: {
		code: number;
	};
};

export type RangeRequestHeader = {
	/** Byte range for partial content requests. */
	range?: string | null;
};

export type ReactionActionDto = "added" | "removed";

export type ReactionContentDto = {
	kind: ReactionKindDto;
	target_guid?: null | MessageId;
};

export type ReactionKindDto = {
	action: ReactionActionDto;
	tapback: TapbackDto;
	type: "tapback";
} | {
	type: "apple_pay";
} | {
	code: number;
	type: "unknown";
};

export type RecurrenceDto = {
	decode_error?: string | null;
	frequency: number;
	interval: number;
	occurrence_count?: number | null;
};

export type RecurrenceFrequencyDto = "daily" | "weekly" | "monthly" | "yearly";

export type RecurrenceInputDto = {
	count?: number | null;
	end_date?: null | UnixTimestamp;
	frequency: RecurrenceFrequencyDto;
	interval: number;
};

export type RecurrenceRuleDto = {
	count?: number | null;
	end_date?: null | UnixTimestamp;
	frequency: number;
	interval: number;
	raw_specifier: string;
	specifier?: string | null;
};

export type ReminderAttachmentDetailDto = {
	filename?: string | null;
	id: ReminderAttachmentId;
	kind: ReminderAttachmentKindDto;
	modified_at?: null | UnixTimestamp;
	reminder_id: ReminderId;
	row_id: number;
	sha512?: string | null;
	uti?: string | null;
};

/** Stable identifier for a reminder attachment (its UUID). */
export type ReminderAttachmentId = string;

export type ReminderAttachmentIdPath = {
	id: ReminderAttachmentId;
};

export type ReminderAttachmentKindDto = "file" | "image" | "audio" | "unknown";

export type ReminderAttachmentSummaryDto = {
	filename?: string | null;
	id: ReminderAttachmentId;
	kind: ReminderAttachmentKindDto;
	row_id: number;
	sha512?: string | null;
	uti?: string | null;
};

export type ReminderDetailDto = {
	alarms: AlarmDto[];
	attachments: ReminderAttachmentSummaryDto[];
	completed: boolean;
	completion_at?: null | UnixTimestamp;
	created_at?: null | UnixTimestamp;
	due?: null | DueDto;
	flagged: boolean;
	id: ReminderId;
	last_modified_at?: null | UnixTimestamp;
	list_id: ReminderListId;
	list_name: string;
	list_row_id: number;
	notes?: string | null;
	parent_id?: null | ReminderId;
	priority: number;
	recurrence?: null | RecurrenceDto;
	row_id: number;
	section_id?: null | SectionId;
	subtasks: ReminderSummaryDto[];
	tags: string[];
	title: string;
};

/** Stable identifier for a reminder (its UUID). */
export type ReminderId = string;

export type ReminderIdPath = {
	reminder_id: ReminderId;
};

export type ReminderListDetailDto = {
	filter: SmartFilterDto;
	id: ReminderListId;
	kind: ReminderListKindDto;
	name: string;
	row_id: number;
	sections: SectionSummaryDto[];
	shared_owner_address?: string | null;
	shared_owner_name?: string | null;
	sharing_status?: number | null;
	smart_list_type?: string | null;
};

/** Stable identifier for a reminder list (its UUID). */
export type ReminderListId = string;

export type ReminderListIdPath = {
	list_id: ReminderListId;
};

export type ReminderListKindDto = "standard" | "smart";

export type ReminderListPageDto = {
	items: ReminderListSummaryDto[];
	page: PageMetaDto;
};

export type ReminderListParams = {
	completed?: boolean | null;
	cursor?: string | null;
	due_after?: number | null;
	due_before?: number | null;
	flagged?: boolean | null;
	has_due_date?: boolean | null;
	has_notes?: boolean | null;
	include_subtasks?: boolean | null;
	include_tags?: boolean | null;
	limit?: number | null;
	list_id?: string | null;
	priority_min?: number | null;
	q?: string | null;
	top_level_only?: boolean | null;
};

export type ReminderListSummaryDto = {
	id: ReminderListId;
	kind: ReminderListKindDto;
	name: string;
	row_id: number;
	smart_list_type?: string | null;
};

export type ReminderPageDto = {
	items: ReminderSummaryDto[];
	page: PageMetaDto;
};

export type ReminderSummaryDto = {
	completed: boolean;
	due?: null | DueDto;
	flagged: boolean;
	id: ReminderId;
	last_modified_at?: null | UnixTimestamp;
	list_id: ReminderListId;
	list_name: string;
	list_row_id: number;
	parent_id?: null | ReminderId;
	priority: number;
	row_id: number;
	section_id?: null | SectionId;
	tags?: string[];
	title: string;
};

/** Stable identifier for a reminder section (its UUID). */
export type SectionId = string;

export type SectionSummaryDto = {
	canonical_name?: string | null;
	display_name: string;
	id: SectionId;
};

export type ShareMyLocationContentDto = {
	other_handle?: string | null;
	status: ShareMyLocationStatusDto;
};

export type ShareMyLocationStatusDto = "started" | "stopped";

export type SharePlayContentDto = {
	payload: OpaquePayloadDto;
	text?: string | null;
};

export type SmartFilterDto = {
	decoded: boolean;
	raw?: unknown;
};

/** AddressBook source UUID (directory name under Sources/). */
export type SourceId = string;

export type StoreTypeDto = "local" | "cal_dav" | "exchange" | "subscription" | "birthday" | {
	unknown: {
		code: number;
	};
};

export type SyncPendingContactDetailDto = {
	detail?: null | ContactDetailDto;
	id: ContactId;
	sync_pending: boolean;
};

export type SyncPendingEventDetailDto = {
	detail?: null | EventDetailDto;
	id: EventId;
	sync_pending: boolean;
};

export type SyncPendingGroupDetailDto = {
	detail?: null | GroupSummaryDto;
	id: GroupId;
	sync_pending: boolean;
};

export type SyncPendingReminderDetailDto = {
	detail?: null | ReminderDetailDto;
	id: ReminderId;
	sync_pending: boolean;
};

export type SystemContentDto = {
	is_service: boolean;
	is_system: boolean;
	text?: string | null;
};

export type TapbackDto = "love" | "like" | "dislike" | "laugh" | "emphasize" | "question" | "unknown";

export type TextContentDto = {
	body: MessageBodyDto;
	is_auto_reply: boolean;
	is_forward: boolean;
};

export type TransportDto = "imessage" | "sms" | "rcs" | "unknown";

export type TransportFilterDto = "imessage" | "sms" | "rcs" | "unknown";

/**
 * Whole seconds since the Unix epoch (`1970-01-01T00:00:00Z`), in UTC.
 *
 * Serialized as a JSON integer. This replaces the RFC 3339 timestamp strings
 * emitted by earlier revisions of the API (see the `apple_types` breaking
 * change in the README).
 */
export type UnixTimestamp = number;

export type UnknownContentDto = {
	associated_message_type: number;
	attachments: AttachmentSummaryDto[];
	item_type: number;
	text?: string | null;
};

export type UpdateContactRequest = {
	department_name?: string | null;
	email_addresses?: LabeledStringDto[] | null;
	family_name?: string | null;
	given_name?: string | null;
	job_title?: string | null;
	middle_name?: string | null;
	nickname?: string | null;
	note?: string | null;
	organization_name?: string | null;
	phone_numbers?: LabeledStringDto[] | null;
	postal_addresses?: PostalAddressDto[] | null;
	url_addresses?: LabeledStringDto[] | null;
};

export type UpdateEventParams = {
	occurrence_start?: null | UnixTimestamp;
};

export type UpdateEventRequest = {
	alarms?: AlarmInputDto[] | null;
	/**
	 * When true, `start` and `end` are read as the local calendar days containing them, and both
	 * are stored snapped to local midnight. When false, both are stored as exact instants.
	 */
	all_day?: boolean | null;
	calendar_id?: null | CalendarId;
	description?: string | null;
	end?: null | UnixTimestamp;
	location?: null | LocationInputDto;
	recurrence?: null | RecurrenceInputDto;
	span?: null | EventSpanDto;
	start?: null | UnixTimestamp;
	status?: null | EventStatusInputDto;
	summary?: string | null;
	url?: string | null;
};

export type UpdateGroupRequest = {
	name?: string | null;
};

export type UpdateReminderRequest = {
	alarms?: AlarmInputDto[] | null;
	attachments?: ReminderAttachmentId[];
	completed?: boolean | null;
	due?: null | DueInputDto;
	flagged?: boolean | null;
	list_id?: null | ReminderListId;
	location?: null | LocationInputDto;
	notes?: string | null;
	parent_id?: null | ReminderId;
	priority?: number | null;
	recurrence?: null | RecurrenceInputDto;
	section_id?: null | SectionId;
	tags?: string[];
	title?: string | null;
	url?: string | null;
};

export type UrlBalloonDto = {
	summary?: string | null;
	title?: string | null;
	url?: string | null;
};

/** Every operation in the contract, keyed by `operationId`. */
export interface Operations {
	/** Add a contact to a group */
	addContactToGroup: {
		method: "POST";
		path: "/v1/groups/{group_id}/contacts/{contact_id}";
		pathParams: {
			group_id: GroupId;
			contact_id: ContactId;
		};
		query: never;
		headers: never;
		body: never;
		status: 204;
		response: void;
	};
	/** Create a contact in a container */
	createContact: {
		method: "POST";
		path: "/v1/containers/{container_id}/contacts";
		pathParams: {
			container_id: ContainerId;
		};
		query: never;
		headers: never;
		body: CreateContactRequest;
		status: 201;
		response: SyncPendingContactDetailDto;
	};
	/** Create a calendar event */
	createEvent: {
		method: "POST";
		path: "/v1/calendars/{calendar_id}/events";
		pathParams: {
			calendar_id: CalendarId;
		};
		query: never;
		headers: never;
		body: CreateEventRequest;
		status: 201;
		response: SyncPendingEventDetailDto;
	};
	/** Create a group in a container */
	createGroup: {
		method: "POST";
		path: "/v1/containers/{container_id}/groups";
		pathParams: {
			container_id: ContainerId;
		};
		query: never;
		headers: never;
		body: CreateGroupRequest;
		status: 201;
		response: SyncPendingGroupDetailDto;
	};
	/** Create a reminder in a list */
	createReminder: {
		method: "POST";
		path: "/v1/reminder-lists/{list_id}/reminders";
		pathParams: {
			list_id: ReminderListId;
		};
		query: never;
		headers: never;
		body: CreateReminderRequest;
		status: 201;
		response: SyncPendingReminderDetailDto;
	};
	/** Delete a contact */
	deleteContact: {
		method: "DELETE";
		path: "/v1/contacts/{contact_id}";
		pathParams: {
			contact_id: ContactId;
		};
		query: never;
		headers: never;
		body: never;
		status: 204;
		response: void;
	};
	/** Delete a calendar event */
	deleteEvent: {
		method: "DELETE";
		path: "/v1/events/{event_id}";
		pathParams: {
			event_id: EventId;
			span: null | EventSpanDto;
			occurrence_start: null | UnixTimestamp;
		};
		query: never;
		headers: never;
		body: never;
		status: 204;
		response: void;
	};
	/** Delete a group */
	deleteGroup: {
		method: "DELETE";
		path: "/v1/groups/{group_id}";
		pathParams: {
			group_id: GroupId;
		};
		query: never;
		headers: never;
		body: never;
		status: 204;
		response: void;
	};
	/** Delete a reminder */
	deleteReminder: {
		method: "DELETE";
		path: "/v1/reminders/{reminder_id}";
		pathParams: {
			reminder_id: ReminderId;
		};
		query: never;
		headers: never;
		body: never;
		status: 204;
		response: void;
	};
	/** Get attachment metadata */
	getAttachment: {
		method: "GET";
		path: "/v1/attachments/{guid}";
		pathParams: {
			/** Attachment GUID. */
			guid: AttachmentId;
		};
		query: never;
		headers: never;
		body: never;
		status: 200;
		response: AttachmentDetailDto;
	};
	/** Download attachment content */
	getAttachmentContent: {
		method: "GET";
		path: "/v1/attachments/{guid}/content";
		pathParams: {
			/** Attachment GUID. */
			guid: AttachmentId;
		};
		query: never;
		headers: {
			/** Byte range for partial content requests. */
			Range?: string | null;
			/** Validator for conditional GET/HEAD requests. */
			"If-None-Match"?: string | null;
			/** Timestamp validator for conditional GET/HEAD requests. */
			"If-Modified-Since"?: string | null;
		};
		body: never;
		status: 200;
		response: ArrayBuffer;
	};
	/** Get a calendar */
	getCalendar: {
		method: "GET";
		path: "/v1/calendars/{calendar_id}";
		pathParams: {
			calendar_id: CalendarId;
		};
		query: never;
		headers: never;
		body: never;
		status: 200;
		response: CalendarDetailDto;
	};
	/** Get chat */
	getChat: {
		method: "GET";
		path: "/v1/chats/{chat_id}";
		pathParams: {
			/** Internal chat row identifier. */
			chat_id: ChatId;
		};
		query: never;
		headers: never;
		body: never;
		status: 200;
		response: ChatDetailDto;
	};
	/** Get a contact as JSON */
	getContact: {
		method: "GET";
		path: "/v1/contacts/{contact_id}";
		pathParams: {
			contact_id: ContactId;
		};
		query: never;
		headers: never;
		body: never;
		status: 200;
		response: ContactDetailDto;
	};
	/** Get a contact as CardDAV XML */
	getContactCarddav: {
		method: "GET";
		path: "/v1/contacts/{contact_id}/carddav";
		pathParams: {
			contact_id: ContactId;
		};
		query: never;
		headers: never;
		body: never;
		status: 200;
		response: string;
	};
	/** Get a contact photo */
	getContactPhoto: {
		method: "GET";
		path: "/v1/contacts/{contact_id}/photo";
		pathParams: {
			contact_id: ContactId;
		};
		query: never;
		headers: never;
		body: never;
		status: 200;
		response: void;
	};
	/** Get a contact as vCard */
	getContactVcard: {
		method: "GET";
		path: "/v1/contacts/{contact_id}/vcard";
		pathParams: {
			contact_id: ContactId;
		};
		query: never;
		headers: never;
		body: never;
		status: 200;
		response: string;
	};
	/** Get a contact container */
	getContainer: {
		method: "GET";
		path: "/v1/containers/{container_id}";
		pathParams: {
			container_id: ContainerId;
		};
		query: never;
		headers: never;
		body: never;
		status: 200;
		response: ContainerSummaryDto;
	};
	/** Get an event as JSON */
	getEvent: {
		method: "GET";
		path: "/v1/events/{event_id}";
		pathParams: {
			event_id: EventId;
		};
		query: never;
		headers: never;
		body: never;
		status: 200;
		response: EventDetailDto;
	};
	/** Stream event attachment bytes */
	getEventAttachmentContent: {
		method: "GET";
		path: "/v1/events/{event_id}/attachments/{attachment_id}";
		pathParams: {
			event_id: EventId;
			attachment_id: CalendarAttachmentId;
		};
		query: never;
		headers: {
			/** Validator for conditional GET/HEAD requests. */
			"If-None-Match"?: string | null;
			/** Timestamp validator for conditional GET/HEAD requests. */
			"If-Modified-Since"?: string | null;
			/** Byte range for partial content requests. */
			Range?: string | null;
		};
		body: never;
		status: 200;
		response: ArrayBuffer;
	};
	/** Get an event as CalDAV XML */
	getEventCaldav: {
		method: "GET";
		path: "/v1/events/{event_id}/caldav";
		pathParams: {
			event_id: EventId;
		};
		query: never;
		headers: never;
		body: never;
		status: 200;
		response: string;
	};
	/** Get an event as iCalendar */
	getEventIcal: {
		method: "GET";
		path: "/v1/events/{event_id}/iCal";
		pathParams: {
			event_id: EventId;
		};
		query: never;
		headers: never;
		body: never;
		status: 200;
		response: string;
	};
	/** Get a contact group */
	getGroup: {
		method: "GET";
		path: "/v1/groups/{group_id}";
		pathParams: {
			group_id: GroupId;
		};
		query: never;
		headers: never;
		body: never;
		status: 200;
		response: GroupSummaryDto;
	};
	/** Health check */
	getHealth: {
		method: "GET";
		path: "/healthz";
		pathParams: never;
		query: never;
		headers: never;
		body: never;
		status: 200;
		response: HealthStatusDto;
	};
	/** Get message */
	getMessage: {
		method: "GET";
		path: "/v1/messages/{guid}";
		pathParams: {
			/** Message GUID. */
			guid: MessageId;
		};
		query: never;
		headers: never;
		body: never;
		status: 200;
		response: MessageDetailDto;
	};
	/** Get a note */
	getNote: {
		method: "GET";
		path: "/v1/notes/{note_id}";
		pathParams: {
			note_id: NoteId;
		};
		query: never;
		headers: never;
		body: never;
		status: 200;
		response: NoteDetailDto;
	};
	/** Get note attachment metadata */
	getNoteAttachment: {
		method: "GET";
		path: "/v1/note-attachments/{id}";
		pathParams: {
			id: NoteAttachmentId;
		};
		query: never;
		headers: never;
		body: never;
		status: 200;
		response: NoteAttachmentDetailDto;
	};
	/** Download note attachment content */
	getNoteAttachmentContent: {
		method: "GET";
		path: "/v1/note-attachments/{id}/content";
		pathParams: {
			id: NoteAttachmentId;
		};
		query: never;
		headers: {
			/** Byte range for partial content requests. */
			Range?: string | null;
			/** Validator for conditional GET/HEAD requests. */
			"If-None-Match"?: string | null;
			/** Timestamp validator for conditional GET/HEAD requests. */
			"If-Modified-Since"?: string | null;
		};
		body: never;
		status: 200;
		response: ArrayBuffer;
	};
	/** Get note contents as Markdown */
	getNoteContents: {
		method: "GET";
		path: "/v1/notes/{note_id}/contents";
		pathParams: {
			note_id: NoteId;
		};
		query: never;
		headers: never;
		body: never;
		status: 200;
		response: string;
	};
	/** Get a note folder */
	getNoteFolder: {
		method: "GET";
		path: "/v1/note-folders/{folder_id}";
		pathParams: {
			folder_id: NoteFolderId;
		};
		query: never;
		headers: never;
		body: never;
		status: 200;
		response: NoteFolderDetailDto;
	};
	/** Get OpenAPI specification */
	getOpenApiSpec: {
		method: "GET";
		path: "/openapi.json";
		pathParams: never;
		query: never;
		headers: never;
		body: never;
		status: 200;
		response: unknown;
	};
	/** Get a reminder */
	getReminder: {
		method: "GET";
		path: "/v1/reminders/{reminder_id}";
		pathParams: {
			reminder_id: ReminderId;
		};
		query: never;
		headers: never;
		body: never;
		status: 200;
		response: ReminderDetailDto;
	};
	/** Get reminder attachment metadata */
	getReminderAttachment: {
		method: "GET";
		path: "/v1/reminder-attachments/{id}";
		pathParams: {
			id: ReminderAttachmentId;
		};
		query: never;
		headers: never;
		body: never;
		status: 200;
		response: ReminderAttachmentDetailDto;
	};
	/** Download reminder attachment content */
	getReminderAttachmentContent: {
		method: "GET";
		path: "/v1/reminder-attachments/{id}/content";
		pathParams: {
			id: ReminderAttachmentId;
		};
		query: never;
		headers: {
			/** Byte range for partial content requests. */
			Range?: string | null;
			/** Validator for conditional GET/HEAD requests. */
			"If-None-Match"?: string | null;
			/** Timestamp validator for conditional GET/HEAD requests. */
			"If-Modified-Since"?: string | null;
		};
		body: never;
		status: 200;
		response: ArrayBuffer;
	};
	/** Get a reminder list */
	getReminderList: {
		method: "GET";
		path: "/v1/reminder-lists/{list_id}";
		pathParams: {
			list_id: ReminderListId;
		};
		query: never;
		headers: never;
		body: never;
		status: 200;
		response: ReminderListDetailDto;
	};
	/** Check attachment content metadata */
	headAttachmentContent: {
		method: "HEAD";
		path: "/v1/attachments/{guid}/content";
		pathParams: {
			/** Attachment GUID. */
			guid: AttachmentId;
		};
		query: never;
		headers: {
			/** Validator for conditional GET/HEAD requests. */
			"If-None-Match"?: string | null;
			/** Timestamp validator for conditional GET/HEAD requests. */
			"If-Modified-Since"?: string | null;
		};
		body: never;
		status: 200;
		response: void;
	};
	/** Check event attachment content metadata */
	headEventAttachmentContent: {
		method: "HEAD";
		path: "/v1/events/{event_id}/attachments/{attachment_id}";
		pathParams: {
			event_id: EventId;
			attachment_id: CalendarAttachmentId;
		};
		query: never;
		headers: {
			/** Validator for conditional GET/HEAD requests. */
			"If-None-Match"?: string | null;
			/** Timestamp validator for conditional GET/HEAD requests. */
			"If-Modified-Since"?: string | null;
		};
		body: never;
		status: 200;
		response: void;
	};
	/** Head note attachment content */
	headNoteAttachmentContent: {
		method: "HEAD";
		path: "/v1/note-attachments/{id}/content";
		pathParams: {
			id: NoteAttachmentId;
		};
		query: never;
		headers: {
			/** Validator for conditional GET/HEAD requests. */
			"If-None-Match"?: string | null;
			/** Timestamp validator for conditional GET/HEAD requests. */
			"If-Modified-Since"?: string | null;
		};
		body: never;
		status: 200;
		response: void;
	};
	/** Head reminder attachment content */
	headReminderAttachmentContent: {
		method: "HEAD";
		path: "/v1/reminder-attachments/{id}/content";
		pathParams: {
			id: ReminderAttachmentId;
		};
		query: never;
		headers: {
			/** Validator for conditional GET/HEAD requests. */
			"If-None-Match"?: string | null;
			/** Timestamp validator for conditional GET/HEAD requests. */
			"If-Modified-Since"?: string | null;
		};
		body: never;
		status: 200;
		response: void;
	};
	/** List calendar accounts */
	listCalendarAccounts: {
		method: "GET";
		path: "/v1/calendar-accounts";
		pathParams: never;
		query: never;
		headers: never;
		body: never;
		status: 200;
		response: CalendarAccountPageDto;
	};
	/** List events for a calendar as JSON */
	listCalendarEvents: {
		method: "GET";
		path: "/v1/calendars/{calendar_id}/events";
		pathParams: {
			calendar_id: CalendarId;
		};
		query: {
			limit?: number;
			cursor?: string;
			q?: string;
			calendar_id?: string;
			account_id?: string;
			start?: number;
			end?: number;
			include_hidden?: boolean;
			include_cancelled?: boolean;
		};
		headers: never;
		body: never;
		status: 200;
		response: EventPageDto;
	};
	/** List events for a calendar as CalDAV XML */
	listCalendarEventsCaldav: {
		method: "GET";
		path: "/v1/calendars/{calendar_id}/events/caldav";
		pathParams: {
			calendar_id: CalendarId;
		};
		query: {
			limit?: number;
			cursor?: string;
			q?: string;
			calendar_id?: string;
			account_id?: string;
			start?: number;
			end?: number;
			include_hidden?: boolean;
			include_cancelled?: boolean;
		};
		headers: never;
		body: never;
		status: 200;
		response: string;
	};
	/** List events for a calendar as iCalendar */
	listCalendarEventsIcal: {
		method: "GET";
		path: "/v1/calendars/{calendar_id}/events/iCal";
		pathParams: {
			calendar_id: CalendarId;
		};
		query: {
			limit?: number;
			cursor?: string;
			q?: string;
			calendar_id?: string;
			account_id?: string;
			start?: number;
			end?: number;
			include_hidden?: boolean;
			include_cancelled?: boolean;
		};
		headers: never;
		body: never;
		status: 200;
		response: string;
	};
	/** List calendars */
	listCalendars: {
		method: "GET";
		path: "/v1/calendars";
		pathParams: never;
		query: {
			/** Maximum number of items to return. Defaults to 50. Keyset pagination only; no offsets. */
			limit?: number;
			/** URL-safe versioned cursor for the next page. Results are ordered newest first. */
			cursor?: string;
		};
		headers: never;
		body: never;
		status: 200;
		response: CalendarPageDto;
	};
	/** List chat messages */
	listChatMessages: {
		method: "GET";
		path: "/v1/chats/{chat_id}/messages";
		pathParams: {
			/** Internal chat row identifier. */
			chat_id: ChatId;
		};
		query: {
			/** Maximum number of items to return. Defaults to 50. Keyset pagination only; no offsets. */
			limit?: number;
			/** URL-safe versioned cursor for the next page. Results are ordered newest first. */
			cursor?: string;
		};
		headers: never;
		body: never;
		status: 200;
		response: MessagePageDto;
	};
	/** List chats */
	listChats: {
		method: "GET";
		path: "/v1/chats";
		pathParams: never;
		query: {
			/** Maximum number of items to return. Defaults to 50. Keyset pagination only; no offsets. */
			limit?: number;
			/** URL-safe versioned cursor for the next page. Results are ordered newest first. */
			cursor?: string;
		};
		headers: never;
		body: never;
		status: 200;
		response: ChatPageDto;
	};
	/** List contacts globally as JSON */
	listContacts: {
		method: "GET";
		path: "/v1/contacts";
		pathParams: never;
		query: {
			limit?: number;
			cursor?: string;
			q?: string;
			container_id?: string;
			group_id?: string;
		};
		headers: never;
		body: never;
		status: 200;
		response: ContactPageDto;
	};
	/** List contacts globally as CardDAV XML */
	listContactsCarddav: {
		method: "GET";
		path: "/v1/contacts/carddav";
		pathParams: never;
		query: {
			limit?: number;
			cursor?: string;
			q?: string;
			container_id?: string;
			group_id?: string;
		};
		headers: never;
		body: never;
		status: 200;
		response: string;
	};
	/** List contacts globally as vCard */
	listContactsVcard: {
		method: "GET";
		path: "/v1/contacts/vcard";
		pathParams: never;
		query: {
			limit?: number;
			cursor?: string;
			q?: string;
			container_id?: string;
			group_id?: string;
		};
		headers: never;
		body: never;
		status: 200;
		response: string;
	};
	/** List contact containers */
	listContainers: {
		method: "GET";
		path: "/v1/containers";
		pathParams: never;
		query: never;
		headers: never;
		body: never;
		status: 200;
		response: ContainerPageDto;
	};
	/** List events globally as JSON */
	listEvents: {
		method: "GET";
		path: "/v1/events";
		pathParams: never;
		query: {
			limit?: number;
			cursor?: string;
			q?: string;
			calendar_id?: string;
			account_id?: string;
			start?: number;
			end?: number;
			include_hidden?: boolean;
			include_cancelled?: boolean;
		};
		headers: never;
		body: never;
		status: 200;
		response: EventPageDto;
	};
	/** List events globally as CalDAV XML */
	listEventsCaldav: {
		method: "GET";
		path: "/v1/events/caldav";
		pathParams: never;
		query: {
			limit?: number;
			cursor?: string;
			q?: string;
			calendar_id?: string;
			account_id?: string;
			start?: number;
			end?: number;
			include_hidden?: boolean;
			include_cancelled?: boolean;
		};
		headers: never;
		body: never;
		status: 200;
		response: string;
	};
	/** List events globally as iCalendar */
	listEventsIcal: {
		method: "GET";
		path: "/v1/events/iCal";
		pathParams: never;
		query: {
			limit?: number;
			cursor?: string;
			q?: string;
			calendar_id?: string;
			account_id?: string;
			start?: number;
			end?: number;
			include_hidden?: boolean;
			include_cancelled?: boolean;
		};
		headers: never;
		body: never;
		status: 200;
		response: string;
	};
	/** List notes in a note folder */
	listFolderNotes: {
		method: "GET";
		path: "/v1/note-folders/{folder_id}/notes";
		pathParams: {
			folder_id: NoteFolderId;
		};
		query: {
			limit?: number;
			cursor?: string;
			q?: string;
			folder_id?: string;
			is_pinned?: boolean;
			is_locked?: boolean;
			has_checklist?: boolean;
			has_attachments?: boolean;
			include_deleted?: boolean;
			modified_before?: number;
			modified_after?: number;
		};
		headers: never;
		body: never;
		status: 200;
		response: NotePageDto;
	};
	/** List contacts in a group as JSON */
	listGroupContacts: {
		method: "GET";
		path: "/v1/groups/{group_id}/contacts";
		pathParams: {
			group_id: GroupId;
		};
		query: {
			/** Maximum number of items to return. Defaults to 50. Keyset pagination only; no offsets. */
			limit?: number;
			/** URL-safe versioned cursor for the next page. Results are ordered newest first. */
			cursor?: string;
		};
		headers: never;
		body: never;
		status: 200;
		response: ContactPageDto;
	};
	/** List contacts in a group as CardDAV XML */
	listGroupContactsCarddav: {
		method: "GET";
		path: "/v1/groups/{group_id}/contacts/carddav";
		pathParams: {
			group_id: GroupId;
		};
		query: {
			/** Maximum number of items to return. Defaults to 50. Keyset pagination only; no offsets. */
			limit?: number;
			/** URL-safe versioned cursor for the next page. Results are ordered newest first. */
			cursor?: string;
		};
		headers: never;
		body: never;
		status: 200;
		response: string;
	};
	/** List contacts in a group as vCard */
	listGroupContactsVcard: {
		method: "GET";
		path: "/v1/groups/{group_id}/contacts/vcard";
		pathParams: {
			group_id: GroupId;
		};
		query: {
			/** Maximum number of items to return. Defaults to 50. Keyset pagination only; no offsets. */
			limit?: number;
			/** URL-safe versioned cursor for the next page. Results are ordered newest first. */
			cursor?: string;
		};
		headers: never;
		body: never;
		status: 200;
		response: string;
	};
	/** List contact groups */
	listGroups: {
		method: "GET";
		path: "/v1/groups";
		pathParams: never;
		query: {
			/** Maximum number of items to return. Defaults to 50. Keyset pagination only; no offsets. */
			limit?: number;
			/** URL-safe versioned cursor for the next page. Results are ordered newest first. */
			cursor?: string;
		};
		headers: never;
		body: never;
		status: 200;
		response: GroupPageDto;
	};
	/** List messages */
	listMessages: {
		method: "GET";
		path: "/v1/messages";
		pathParams: never;
		query: {
			/** Maximum number of items to return. Defaults to 50. Keyset pagination only; no offsets. */
			limit?: number;
			/** URL-safe versioned cursor for the next page. Results are ordered newest first. */
			cursor?: string;
			/** Case-insensitive search over plain text and decoded attributed-body text. */
			q?: string;
			/** Restrict results to messages in this chat. */
			chat_id?: ChatId;
			/** Restrict results to messages from this handle identifier. */
			sender?: string;
			/** Return messages sent strictly before this Unix timestamp (UTC seconds). */
			before?: number;
			/** Return messages sent strictly after this Unix timestamp (UTC seconds). */
			after?: number;
			/** Restrict results to sent or received messages. */
			direction?: DirectionFilterDto;
			/** Restrict results to a transport/service. */
			transport?: TransportFilterDto;
			/** Restrict results to a coarse message content category. */
			content_type?: ContentTypeFilterDto;
			/** Restrict results to messages with or without attachments. */
			has_attachments?: boolean;
		};
		headers: never;
		body: never;
		status: 200;
		response: MessagePageDto;
	};
	/** List note folders */
	listNoteFolders: {
		method: "GET";
		path: "/v1/note-folders";
		pathParams: never;
		query: {
			/** Maximum number of items to return. Defaults to 50. Keyset pagination only; no offsets. */
			limit?: number;
			/** URL-safe versioned cursor for the next page. Results are ordered newest first. */
			cursor?: string;
		};
		headers: never;
		body: never;
		status: 200;
		response: NoteFolderPageDto;
	};
	/** List notes globally */
	listNotes: {
		method: "GET";
		path: "/v1/notes";
		pathParams: never;
		query: {
			limit?: number;
			cursor?: string;
			q?: string;
			folder_id?: string;
			is_pinned?: boolean;
			is_locked?: boolean;
			has_checklist?: boolean;
			has_attachments?: boolean;
			include_deleted?: boolean;
			modified_before?: number;
			modified_after?: number;
		};
		headers: never;
		body: never;
		status: 200;
		response: NotePageDto;
	};
	/** List reminders in a reminder list */
	listReminderListReminders: {
		method: "GET";
		path: "/v1/reminder-lists/{list_id}/reminders";
		pathParams: {
			list_id: ReminderListId;
		};
		query: {
			limit?: number;
			cursor?: string;
			completed?: boolean;
			flagged?: boolean;
			has_due_date?: boolean;
			due_before?: number;
			due_after?: number;
			priority_min?: number;
			has_notes?: boolean;
			top_level_only?: boolean;
			include_subtasks?: boolean;
			include_tags?: boolean;
			q?: string;
			list_id?: string;
		};
		headers: never;
		body: never;
		status: 200;
		response: ReminderPageDto;
	};
	/** List reminder lists */
	listReminderLists: {
		method: "GET";
		path: "/v1/reminder-lists";
		pathParams: never;
		query: {
			/** Maximum number of items to return. Defaults to 50. Keyset pagination only; no offsets. */
			limit?: number;
			/** URL-safe versioned cursor for the next page. Results are ordered newest first. */
			cursor?: string;
		};
		headers: never;
		body: never;
		status: 200;
		response: ReminderListPageDto;
	};
	/** List reminders globally */
	listReminders: {
		method: "GET";
		path: "/v1/reminders";
		pathParams: never;
		query: {
			limit?: number;
			cursor?: string;
			completed?: boolean;
			flagged?: boolean;
			has_due_date?: boolean;
			due_before?: number;
			due_after?: number;
			priority_min?: number;
			has_notes?: boolean;
			top_level_only?: boolean;
			include_subtasks?: boolean;
			include_tags?: boolean;
			q?: string;
			list_id?: string;
		};
		headers: never;
		body: never;
		status: 200;
		response: ReminderPageDto;
	};
	/** Remove a contact from a group */
	removeContactFromGroup: {
		method: "DELETE";
		path: "/v1/groups/{group_id}/contacts/{contact_id}";
		pathParams: {
			group_id: GroupId;
			contact_id: ContactId;
		};
		query: never;
		headers: never;
		body: never;
		status: 204;
		response: void;
	};
	/** Search contacts */
	searchContacts: {
		method: "GET";
		path: "/v1/contacts/search";
		pathParams: never;
		query: {
			limit?: number;
			cursor?: string;
			q?: string;
			container_id?: string;
			group_id?: string;
		};
		headers: never;
		body: never;
		status: 200;
		response: ContactPageDto;
	};
	/** Update a contact */
	updateContact: {
		method: "PATCH";
		path: "/v1/contacts/{contact_id}";
		pathParams: {
			contact_id: ContactId;
		};
		query: never;
		headers: never;
		body: UpdateContactRequest;
		status: 200;
		response: SyncPendingContactDetailDto;
	};
	/** Update a calendar event */
	updateEvent: {
		method: "PATCH";
		path: "/v1/events/{event_id}";
		pathParams: {
			event_id: EventId;
			occurrence_start: null | UnixTimestamp;
		};
		query: never;
		headers: never;
		body: UpdateEventRequest;
		status: 200;
		response: SyncPendingEventDetailDto;
	};
	/** Update a group */
	updateGroup: {
		method: "PATCH";
		path: "/v1/groups/{group_id}";
		pathParams: {
			group_id: GroupId;
		};
		query: never;
		headers: never;
		body: UpdateGroupRequest;
		status: 200;
		response: SyncPendingGroupDetailDto;
	};
	/** Update a reminder */
	updateReminder: {
		method: "PATCH";
		path: "/v1/reminders/{reminder_id}";
		pathParams: {
			reminder_id: ReminderId;
		};
		query: never;
		headers: never;
		body: UpdateReminderRequest;
		status: 200;
		response: SyncPendingReminderDetailDto;
	};
}

/** Union of every `operationId` in the contract. */
export type OperationId = keyof Operations;

/** How the client should decode a successful response body. */
export type ResponseKind = "json" | "text" | "binary" | "void";

/** Runtime method, path template and decoding strategy for each operation. */
export const routes = {
	addContactToGroup: { method: "POST", path: "/v1/groups/{group_id}/contacts/{contact_id}", kind: "void" },
	createContact: { method: "POST", path: "/v1/containers/{container_id}/contacts", kind: "json" },
	createEvent: { method: "POST", path: "/v1/calendars/{calendar_id}/events", kind: "json" },
	createGroup: { method: "POST", path: "/v1/containers/{container_id}/groups", kind: "json" },
	createReminder: { method: "POST", path: "/v1/reminder-lists/{list_id}/reminders", kind: "json" },
	deleteContact: { method: "DELETE", path: "/v1/contacts/{contact_id}", kind: "void" },
	deleteEvent: { method: "DELETE", path: "/v1/events/{event_id}", kind: "void" },
	deleteGroup: { method: "DELETE", path: "/v1/groups/{group_id}", kind: "void" },
	deleteReminder: { method: "DELETE", path: "/v1/reminders/{reminder_id}", kind: "void" },
	getAttachment: { method: "GET", path: "/v1/attachments/{guid}", kind: "json" },
	getAttachmentContent: { method: "GET", path: "/v1/attachments/{guid}/content", kind: "binary" },
	getCalendar: { method: "GET", path: "/v1/calendars/{calendar_id}", kind: "json" },
	getChat: { method: "GET", path: "/v1/chats/{chat_id}", kind: "json" },
	getContact: { method: "GET", path: "/v1/contacts/{contact_id}", kind: "json" },
	getContactCarddav: { method: "GET", path: "/v1/contacts/{contact_id}/carddav", kind: "text" },
	getContactPhoto: { method: "GET", path: "/v1/contacts/{contact_id}/photo", kind: "void" },
	getContactVcard: { method: "GET", path: "/v1/contacts/{contact_id}/vcard", kind: "text" },
	getContainer: { method: "GET", path: "/v1/containers/{container_id}", kind: "json" },
	getEvent: { method: "GET", path: "/v1/events/{event_id}", kind: "json" },
	getEventAttachmentContent: { method: "GET", path: "/v1/events/{event_id}/attachments/{attachment_id}", kind: "binary" },
	getEventCaldav: { method: "GET", path: "/v1/events/{event_id}/caldav", kind: "text" },
	getEventIcal: { method: "GET", path: "/v1/events/{event_id}/iCal", kind: "text" },
	getGroup: { method: "GET", path: "/v1/groups/{group_id}", kind: "json" },
	getHealth: { method: "GET", path: "/healthz", kind: "json" },
	getMessage: { method: "GET", path: "/v1/messages/{guid}", kind: "json" },
	getNote: { method: "GET", path: "/v1/notes/{note_id}", kind: "json" },
	getNoteAttachment: { method: "GET", path: "/v1/note-attachments/{id}", kind: "json" },
	getNoteAttachmentContent: { method: "GET", path: "/v1/note-attachments/{id}/content", kind: "binary" },
	getNoteContents: { method: "GET", path: "/v1/notes/{note_id}/contents", kind: "text" },
	getNoteFolder: { method: "GET", path: "/v1/note-folders/{folder_id}", kind: "json" },
	getOpenApiSpec: { method: "GET", path: "/openapi.json", kind: "json" },
	getReminder: { method: "GET", path: "/v1/reminders/{reminder_id}", kind: "json" },
	getReminderAttachment: { method: "GET", path: "/v1/reminder-attachments/{id}", kind: "json" },
	getReminderAttachmentContent: { method: "GET", path: "/v1/reminder-attachments/{id}/content", kind: "binary" },
	getReminderList: { method: "GET", path: "/v1/reminder-lists/{list_id}", kind: "json" },
	headAttachmentContent: { method: "HEAD", path: "/v1/attachments/{guid}/content", kind: "void" },
	headEventAttachmentContent: { method: "HEAD", path: "/v1/events/{event_id}/attachments/{attachment_id}", kind: "void" },
	headNoteAttachmentContent: { method: "HEAD", path: "/v1/note-attachments/{id}/content", kind: "void" },
	headReminderAttachmentContent: { method: "HEAD", path: "/v1/reminder-attachments/{id}/content", kind: "void" },
	listCalendarAccounts: { method: "GET", path: "/v1/calendar-accounts", kind: "json" },
	listCalendarEvents: { method: "GET", path: "/v1/calendars/{calendar_id}/events", kind: "json" },
	listCalendarEventsCaldav: { method: "GET", path: "/v1/calendars/{calendar_id}/events/caldav", kind: "text" },
	listCalendarEventsIcal: { method: "GET", path: "/v1/calendars/{calendar_id}/events/iCal", kind: "text" },
	listCalendars: { method: "GET", path: "/v1/calendars", kind: "json" },
	listChatMessages: { method: "GET", path: "/v1/chats/{chat_id}/messages", kind: "json" },
	listChats: { method: "GET", path: "/v1/chats", kind: "json" },
	listContacts: { method: "GET", path: "/v1/contacts", kind: "json" },
	listContactsCarddav: { method: "GET", path: "/v1/contacts/carddav", kind: "text" },
	listContactsVcard: { method: "GET", path: "/v1/contacts/vcard", kind: "text" },
	listContainers: { method: "GET", path: "/v1/containers", kind: "json" },
	listEvents: { method: "GET", path: "/v1/events", kind: "json" },
	listEventsCaldav: { method: "GET", path: "/v1/events/caldav", kind: "text" },
	listEventsIcal: { method: "GET", path: "/v1/events/iCal", kind: "text" },
	listFolderNotes: { method: "GET", path: "/v1/note-folders/{folder_id}/notes", kind: "json" },
	listGroupContacts: { method: "GET", path: "/v1/groups/{group_id}/contacts", kind: "json" },
	listGroupContactsCarddav: { method: "GET", path: "/v1/groups/{group_id}/contacts/carddav", kind: "text" },
	listGroupContactsVcard: { method: "GET", path: "/v1/groups/{group_id}/contacts/vcard", kind: "text" },
	listGroups: { method: "GET", path: "/v1/groups", kind: "json" },
	listMessages: { method: "GET", path: "/v1/messages", kind: "json" },
	listNoteFolders: { method: "GET", path: "/v1/note-folders", kind: "json" },
	listNotes: { method: "GET", path: "/v1/notes", kind: "json" },
	listReminderListReminders: { method: "GET", path: "/v1/reminder-lists/{list_id}/reminders", kind: "json" },
	listReminderLists: { method: "GET", path: "/v1/reminder-lists", kind: "json" },
	listReminders: { method: "GET", path: "/v1/reminders", kind: "json" },
	removeContactFromGroup: { method: "DELETE", path: "/v1/groups/{group_id}/contacts/{contact_id}", kind: "void" },
	searchContacts: { method: "GET", path: "/v1/contacts/search", kind: "json" },
	updateContact: { method: "PATCH", path: "/v1/contacts/{contact_id}", kind: "json" },
	updateEvent: { method: "PATCH", path: "/v1/events/{event_id}", kind: "json" },
	updateGroup: { method: "PATCH", path: "/v1/groups/{group_id}", kind: "json" },
	updateReminder: { method: "PATCH", path: "/v1/reminders/{reminder_id}", kind: "json" },
} as const satisfies Record<OperationId, { method: string; path: string; kind: ResponseKind }>;
