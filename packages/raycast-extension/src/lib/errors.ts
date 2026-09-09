import { ErrorCodeValues } from "./api.gen";
import type { ErrorCode } from "./api.gen";

/** Runtime membership test, so an unknown payload is validated, never asserted. */
function isErrorCode(value: unknown): value is ErrorCode {
	return typeof value === "string" && (ErrorCodeValues as readonly string[]).includes(value);
}

/**
 * A typed error returned by the API.
 *
 * The server always answers failures with `{ error: { code, message, details } }`
 * where `code` is a stable snake_case identifier (see `docs/errors.md`). Callers
 * branch on `code`; `message` is human-readable and must never be parsed.
 */
export class ApiError extends Error {
	readonly code: ErrorCode | "unknown";
	readonly status: number;
	readonly details: unknown;

	constructor(status: number, code: ErrorCode | "unknown", message: string, details: unknown) {
		super(message);
		this.name = "ApiError";
		this.status = status;
		this.code = code;
		this.details = details;
	}

	/**
	 * Build an error from a failed response.
	 *
	 * The body is validated structurally rather than cast: a proxy or a crash can
	 * put arbitrary content on the wire, and a bad payload should still surface as
	 * a usable error instead of throwing inside the error path.
	 */
	static async fromResponse(response: Response): Promise<ApiError> {
		const fallback = `Request failed with status ${response.status}`;
		let body: unknown;
		try {
			body = await response.json();
		} catch {
			return new ApiError(response.status, "unknown", fallback, undefined);
		}
		if (typeof body !== "object" || body === null || !("error" in body)) {
			return new ApiError(response.status, "unknown", fallback, undefined);
		}
		const error = body.error;
		if (typeof error !== "object" || error === null) {
			return new ApiError(response.status, "unknown", fallback, undefined);
		}
		const code = "code" in error && isErrorCode(error.code) ? error.code : "unknown";
		const message = "message" in error && typeof error.message === "string" ? error.message : fallback;
		const details = "details" in error ? error.details : undefined;
		return new ApiError(response.status, code, message, details);
	}
}

/** The server could not be reached at all — usually it simply is not running. */
export class ServerUnreachableError extends Error {
	readonly baseUrl: string;

	constructor(baseUrl: string, cause: unknown) {
		super(`Could not reach apple-connector at ${baseUrl}`);
		this.name = "ServerUnreachableError";
		this.baseUrl = baseUrl;
		this.cause = cause;
	}
}

/**
 * Guidance for the error codes a user can actually act on.
 *
 * Anything absent from this map falls back to the server's `message`, which is
 * already human-readable. Only codes with a concrete remediation belong here.
 */
const REMEDIATION: Partial<Record<ErrorCode, string>> = {
	eventkit_access_denied: "Grant Reminders and Calendars access in System Settings › Privacy & Security.",
	contacts_access_denied: "Grant Contacts access in System Settings › Privacy & Security.",
	eventkit_unavailable: "EventKit could not start. Restart the apple-connector server.",
	contacts_unavailable: "The Contacts framework could not start. Restart the apple-connector server.",
	messages_database_unavailable: "Grant Full Disk Access to the apple-connector server, then restart it.",
	reminders_database_unavailable: "Grant Full Disk Access to the apple-connector server, then restart it.",
	notes_database_unavailable: "Grant Full Disk Access to the apple-connector server, then restart it.",
	calendar_database_unavailable: "Grant Full Disk Access to the apple-connector server, then restart it.",
	contacts_database_unavailable: "Grant Full Disk Access to the apple-connector server, then restart it.",
	invalid_cursor: "The result cursor expired. Run the search again.",
	smart_list_read_only: "Smart lists cannot be written to. Pick a regular list.",
	calendar_read_only: "That calendar or list is read-only.",
	read_only_container: "That container is read-only.",
};

/** Actionable remediation for an error, when one exists. */
export function remediationFor(error: unknown): string | undefined {
	if (error instanceof ServerUnreachableError) {
		return `Start the server with \`cargo run -p apple-connector\`, or update the server URL in this extension's preferences.`;
	}
	if (error instanceof ApiError && error.code !== "unknown") {
		return REMEDIATION[error.code];
	}
	return undefined;
}

/** Short, user-facing title for an error. */
export function titleFor(error: unknown): string {
	if (error instanceof ServerUnreachableError) {
		return "Server unreachable";
	}
	if (error instanceof ApiError) {
		return error.message;
	}
	return error instanceof Error ? error.message : "Something went wrong";
}
