import type { UnixTimestamp } from "./api.gen";

/**
 * The API speaks Unix **seconds** as JSON integers, in responses and query
 * bounds alike — RFC 3339 strings are rejected. Convert at this boundary so no
 * command has to re-derive it.
 */
export function toDate(seconds: UnixTimestamp | null | undefined): Date | undefined {
	if (seconds === null || seconds === undefined) {
		return undefined;
	}
	return new Date(seconds * 1000);
}

/** Inverse of {@link toDate}, for `before` / `after` style query bounds. */
export function toUnixSeconds(date: Date): number {
	return Math.floor(date.getTime() / 1000);
}
