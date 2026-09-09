import { request } from "../lib/client";
import type { Tool } from "@raycast/api";

type Input = {
	/** Title of the event. Required. */
	summary: string;
	/** When the event starts, as Unix seconds (UTC). Required. */
	start: number;
	/** When the event ends, as Unix seconds (UTC). Must not precede `start`. */
	end: number;
	/**
	 * Identifier of the calendar to create the event in. Omit to use the first
	 * writable calendar.
	 */
	calendarId?: string;
	/** Treat `start` and `end` as whole calendar days rather than exact instants. */
	allDay?: boolean;
	/** Optional description or notes. */
	description?: string;
	/** Optional location name. */
	location?: string;
	/** Optional URL to attach. */
	url?: string;
};

/**
 * The contract exposes no writability flag on a calendar — a read-only target is
 * only reported at write time as `calendar_read_only`. So this picks the first
 * calendar and lets the server reject it, rather than guessing.
 */
async function resolveCalendarId(calendarId: string | undefined): Promise<string> {
	if (calendarId) {
		return calendarId;
	}
	const calendars = await request("listCalendars", { query: { limit: 200 } });
	const first = calendars.items[0];
	if (!first) {
		throw new Error("No calendars found.");
	}
	return first.id;
}

/** Create an event in the user's Calendar app. */
export default async function createEvent(input: Input) {
	const calendarId = await resolveCalendarId(input.calendarId);
	const event = await request("createEvent", {
		path: { calendar_id: calendarId },
		body: {
			summary: input.summary,
			start: input.start,
			end: input.end,
			all_day: input.allDay ?? false,
			description: input.description ?? null,
			location: input.location === undefined ? null : { title: input.location },
			url: input.url ?? null,
		},
	});

	// As with reminders, `detail` is absent until the SQLite read path catches up.
	return {
		id: event.id,
		summary: event.detail?.summary ?? input.summary,
		start: event.detail?.start ?? input.start,
		end: event.detail?.end ?? input.end,
		syncPending: event.sync_pending,
	};
}

/** Writes to the user's real calendar, so it always confirms first. */
export const confirmation: Tool.Confirmation<Input> = async input => ({
	message: `Create "${input.summary}" in your calendar?`,
	info: [
		{ name: "Starts", value: new Date(input.start * 1000).toLocaleString() },
		{ name: "Ends", value: new Date(input.end * 1000).toLocaleString() },
		...(input.location ? [{ name: "Location", value: input.location }] : []),
	],
});
