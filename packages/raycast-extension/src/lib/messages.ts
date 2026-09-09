import { Icon } from "@raycast/api";
import type { DirectionDto, HandleDto, MessageContentDto, TransportDto } from "./api.gen";

function trimmed(text: string | null | undefined): string | undefined {
	const value = text?.trim();
	return value ? value : undefined;
}

/**
 * One-line summary of a message.
 *
 * The content union is exhaustive in the contract, so every variant gets a
 * deliberate rendering rather than falling through to a generic placeholder.
 */
export function previewText(content: MessageContentDto): string {
	switch (content.type) {
		case "text":
			return trimmed(content.body.text) ?? "(no text)";
		case "audio":
			return trimmed(content.body.text) ?? "Audio message";
		case "attachment": {
			const count = content.attachments.length;
			return trimmed(content.body.text) ?? (count === 1 ? "1 attachment" : `${count} attachments`);
		}
		case "reaction":
			switch (content.kind.type) {
				case "tapback":
					return `${content.kind.action === "removed" ? "Removed" : "Added"} ${content.kind.tapback} reaction`;
				case "apple_pay":
					return "Apple Pay";
				default:
					return "Reaction";
			}
		case "group_event":
			return trimmed(content.title) ?? content.action.replaceAll("_", " ");
		case "app_balloon":
			return trimmed(content.text) ?? content.bundle_id;
		case "share_play":
			return trimmed(content.text) ?? "SharePlay";
		case "share_my_location":
			return `Location sharing: ${content.status}`;
		case "system":
			return trimmed(content.text) ?? "System message";
		default:
			return "Unsupported message";
	}
}

export function contentIcon(content: MessageContentDto): Icon {
	switch (content.type) {
		case "attachment":
			return Icon.Paperclip;
		case "audio":
			return Icon.Microphone;
		case "reaction":
			return Icon.Heart;
		case "group_event":
			return Icon.TwoPeople;
		case "share_my_location":
			return Icon.Pin;
		case "app_balloon":
		case "share_play":
			return Icon.AppWindow;
		case "system":
			return Icon.Info;
		case "text":
			return Icon.Message;
		default:
			return Icon.QuestionMark;
	}
}

/** Display label for a handle, falling back to the raw identifier. */
export function handleLabel(handle: HandleDto | null | undefined, direction: DirectionDto): string {
	if (direction === "sent") {
		return "You";
	}
	return handle?.id ?? "Unknown";
}

export const TRANSPORT_LABEL: Record<TransportDto, string> = {
	imessage: "iMessage",
	sms: "SMS",
	rcs: "RCS",
	unknown: "Unknown",
};

/**
 * Deep link that opens a conversation in Messages.app.
 *
 * The API is read-only for Messages — there is no send endpoint — so replying
 * always means handing off to the real app rather than pretending to send.
 */
export function messagesAppUrl(handleOrGuid: string | null | undefined): string | undefined {
	const value = trimmed(handleOrGuid);
	return value ? `imessage://${encodeURIComponent(value)}` : undefined;
}
