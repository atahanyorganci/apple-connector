import { clampLimit, truncate } from "../lib/ai";
import { request } from "../lib/client";

type Input = {
	/** Text to search event titles for. */
	query?: string;
	/** Only events starting at or after this instant, as Unix seconds (UTC). */
	start?: number;
	/** Only events ending at or before this instant, as Unix seconds (UTC). */
	end?: number;
	/** Restrict to a single calendar by id. */
	calendarId?: string;
	/** Maximum number of events to return. Defaults to 25, which is also the cap. */
	limit?: number;
};

/** List or search the user's calendar events across every calendar. */
export default async function listEvents(input: Input) {
	const page = await request("listEvents", {
		query: {
			q: truncate(input.query, 256),
			start: input.start,
			end: input.end,
			calendar_id: input.calendarId,
			limit: clampLimit(input.limit),
		},
	});

	return page.items.map(event => ({
		id: event.id,
		summary: event.summary,
		start: event.start,
		end: event.end,
		allDay: event.all_day,
		isRecurring: event.is_recurring,
		status: event.status,
	}));
}
