import { request } from "../lib/client";
import type { Tool } from "@raycast/api";

type Input = {
	/** Title of the reminder. Required. */
	title: string;
	/**
	 * Identifier of the list to create the reminder in. Omit to use the first
	 * writable list. Smart lists cannot be written to.
	 */
	listId?: string;
	/** Optional free-text notes. */
	notes?: string;
	/** When the reminder is due, as Unix seconds (UTC). */
	dueAt?: number;
	/** Treat `dueAt` as a whole calendar day rather than an exact instant. */
	allDay?: boolean;
	/** Flag the reminder. */
	flagged?: boolean;
	/** Reminders priority: 0 none, 1 high, 5 medium, 9 low. */
	priority?: number;
	/** Optional URL to attach. */
	url?: string;
	/** Tags to apply, without a leading hash. */
	tags?: string[];
};

async function resolveListId(listId: string | undefined): Promise<string> {
	if (listId) {
		return listId;
	}
	const lists = await request("listReminderLists", { query: { limit: 200 } });
	const writable = lists.items.find(list => list.kind !== "smart");
	if (!writable) {
		throw new Error("No writable reminder list found.");
	}
	return writable.id;
}

/**
 * Create a reminder in the user's Reminders app.
 *
 * Supports fields the built-in Raycast extension cannot set, including flags,
 * URLs and tags.
 */
export default async function createReminder(input: Input) {
	const listId = await resolveListId(input.listId);
	const reminder = await request("createReminder", {
		path: { list_id: listId },
		body: {
			title: input.title,
			notes: input.notes ?? null,
			flagged: input.flagged ?? null,
			priority: input.priority ?? null,
			url: input.url ?? null,
			tags: input.tags ?? [],
			due: input.dueAt === undefined ? null : { at: input.dueAt, all_day: input.allDay ?? false },
		},
	});

	// The write always answers with the sync-pending envelope. `detail` is only
	// populated once the SQLite read path has caught up, so fall back to the
	// requested title rather than reporting an empty result.
	return {
		id: reminder.id,
		title: reminder.detail?.title ?? input.title,
		list: reminder.detail?.list_name,
		syncPending: reminder.sync_pending,
	};
}

/** Writes to the user's real Reminders database, so it always confirms first. */
export const confirmation: Tool.Confirmation<Input> = async input => ({
	message: `Create a reminder titled "${input.title}"?`,
	info: [
		{ name: "Title", value: input.title },
		...(input.dueAt === undefined ? [] : [{ name: "Due", value: new Date(input.dueAt * 1000).toLocaleString() }]),
		...(input.tags?.length ? [{ name: "Tags", value: input.tags.join(", ") }] : []),
	],
});
