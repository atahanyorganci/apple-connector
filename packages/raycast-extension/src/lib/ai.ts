/**
 * Tool results share a context window with the conversation, so every tool caps
 * how much it returns rather than dumping a full page.
 */
export const MAX_RESULTS = 25;

/** Clamp a caller-supplied limit into something a context window can hold. */
export function clampLimit(limit: number | undefined): number {
	if (limit === undefined || !Number.isFinite(limit)) {
		return MAX_RESULTS;
	}
	return Math.max(1, Math.min(Math.floor(limit), MAX_RESULTS));
}

/** Trim long free text so one field cannot crowd out the rest of the result. */
export function truncate(text: string | null | undefined, max = 500): string | undefined {
	const value = text?.trim();
	if (!value) {
		return undefined;
	}
	return value.length <= max ? value : `${value.slice(0, max)}…`;
}
