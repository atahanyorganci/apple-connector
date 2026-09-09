import { useFetch } from "@raycast/utils";
import { HealthStatusValues } from "./api.gen";
import { baseUrl } from "./client";
import type { HealthStatus } from "./api.gen";

/** Read-backed domains reported by `/healthz`. */
export type Domain = "messages" | "reminders" | "notes" | "calendar" | "contacts";

function isHealthStatus(value: unknown): value is HealthStatus {
	return typeof value === "string" && (HealthStatusValues as readonly string[]).includes(value);
}

/**
 * `/healthz` answers `503` with the *same* body when a pool is down, so a
 * non-OK status here is data rather than failure. Only a malformed body or an
 * unreachable server is treated as an error.
 */
function readStatus(body: unknown, domain: Domain): HealthStatus | undefined {
	if (typeof body !== "object" || body === null || !(domain in body)) {
		return undefined;
	}
	const value: unknown = Reflect.get(body, domain);
	return isHealthStatus(value) ? value : undefined;
}

export type HealthState = {
	isLoading: boolean;
	/** True once the server answered, whatever the per-domain results. */
	reachable: boolean;
	statusFor: (domain: Domain) => HealthStatus | undefined;
};

/**
 * Health of the server and each domain.
 *
 * Commands call this to render an actionable empty state instead of surfacing a
 * bare `service_unavailable`, which matters because most of this API depends on
 * Full Disk Access and per-domain TCC grants.
 */
export function useHealth(): HealthState {
	const { data, isLoading, error } = useFetch(`${baseUrl()}/healthz`, {
		async parseResponse(response) {
			return await response.json();
		},
		keepPreviousData: true,
	});

	return {
		isLoading,
		reachable: error === undefined && data !== undefined,
		statusFor: domain => readStatus(data, domain),
	};
}

/** Guidance shown when a domain's database could not be opened. */
export function unavailableHint(domain: Domain): string {
	return `The ${domain} database is unavailable. Grant Full Disk Access to the apple-connector server and restart it.`;
}
