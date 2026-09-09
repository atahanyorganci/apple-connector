import { clampLimit, truncate } from "../lib/ai";
import { request } from "../lib/client";
import { previewText } from "../lib/messages";
import type { DirectionFilterDto, TransportFilterDto } from "../lib/api.gen";

type Input = {
	/** Text to search for. Matches plain message text and decoded attributed bodies. */
	query?: string;
	/** Restrict to messages the user sent or received. */
	direction?: DirectionFilterDto;
	/** Restrict to a transport: imessage, sms, rcs or unknown. */
	transport?: TransportFilterDto;
	/** Only messages sent strictly after this instant, as Unix seconds (UTC). */
	after?: number;
	/** Only messages sent strictly before this instant, as Unix seconds (UTC). */
	before?: number;
	/** Maximum number of messages to return. Defaults to 25, which is also the cap. */
	limit?: number;
};

/**
 * Search the user's Messages history.
 *
 * Covers both plain text and decoded attributed bodies. This is read-only —
 * there is no way to send a message through this extension.
 */
export default async function searchMessages(input: Input) {
	const page = await request("listMessages", {
		query: {
			q: truncate(input.query, 256),
			direction: input.direction,
			transport: input.transport,
			after: input.after,
			before: input.before,
			limit: clampLimit(input.limit),
		},
	});

	return page.items.map(message => ({
		id: message.guid,
		text: previewText(message.content),
		sender: message.direction === "sent" ? "me" : message.sender?.id,
		direction: message.direction,
		transport: message.transport,
		sentAt: message.sent_at,
	}));
}
