import { clampLimit, truncate } from "../lib/ai";
import { request } from "../lib/client";

type Input = {
	/** Text to search for across reminder titles and notes. */
	query?: string;
	/** Filter by completion. Omit to include both completed and open reminders. */
	completed?: boolean;
	/** Only flagged reminders when true. */
	flagged?: boolean;
	/** Only reminders due strictly before this instant, as Unix seconds (UTC). */
	dueBefore?: number;
	/** Only reminders due strictly after this instant, as Unix seconds (UTC). */
	dueAfter?: number;
	/** Maximum number of reminders to return. Defaults to 25, which is also the cap. */
	limit?: number;
};

/** Search the user's reminders across every list. */
export default async function searchReminders(input: Input) {
	const page = await request("listReminders", {
		query: {
			q: truncate(input.query, 256),
			completed: input.completed,
			flagged: input.flagged,
			due_before: input.dueBefore,
			due_after: input.dueAfter,
			include_tags: true,
			limit: clampLimit(input.limit),
		},
	});

	return page.items.map(reminder => ({
		id: reminder.id,
		title: reminder.title,
		list: reminder.list_name,
		completed: reminder.completed,
		flagged: reminder.flagged,
		priority: reminder.priority,
		dueAt: reminder.due?.at,
		allDay: reminder.due?.all_day,
		tags: reminder.tags,
	}));
}
