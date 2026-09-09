import { Color, Icon } from "@raycast/api";
import { toDate } from "./time";
import type { DueDto, ReminderSummaryDto } from "./api.gen";

/**
 * Reminders priority is a 0–9 scale where 0 means "none", 1–4 is high, 5 is
 * medium and 6–9 is low. Expose the four values the app itself offers.
 */
export const PRIORITIES = [
	{ label: "None", value: 0 },
	{ label: "Low", value: 9 },
	{ label: "Medium", value: 5 },
	{ label: "High", value: 1 },
] as const;

export function priorityLabel(priority: number): string | undefined {
	if (priority === 0) {
		return undefined;
	}
	if (priority <= 4) {
		return "High";
	}
	return priority === 5 ? "Medium" : "Low";
}

/** Start of tomorrow, local time — the boundary for "due today". */
export function endOfToday(): Date {
	const date = new Date();
	date.setHours(23, 59, 59, 999);
	return date;
}

export function isOverdue(due: DueDto | null | undefined): boolean {
	const at = toDate(due?.at);
	return at !== undefined && at.getTime() < Date.now();
}

export function dueAccessory(due: DueDto | null | undefined) {
	const at = toDate(due?.at);
	if (!at) {
		return undefined;
	}
	return {
		date: at,
		tooltip: due?.all_day ? `Due ${at.toLocaleDateString()} (all day)` : `Due ${at.toLocaleString()}`,
	};
}

export function reminderIcon(reminder: ReminderSummaryDto) {
	if (reminder.completed) {
		return { source: Icon.CheckCircle, tintColor: Color.Green };
	}
	if (isOverdue(reminder.due)) {
		return { source: Icon.ExclamationMark, tintColor: Color.Red };
	}
	if (reminder.flagged) {
		return { source: Icon.Flag, tintColor: Color.Orange };
	}
	return { source: Icon.Circle };
}

/** Comma-separated tag input, normalized to the array the contract expects. */
export function parseTags(input: string): string[] {
	return input
		.split(",")
		.map(tag => tag.trim())
		.filter(tag => tag !== "");
}
