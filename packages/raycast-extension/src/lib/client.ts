import { getPreferenceValues } from "@raycast/api";
import { routes } from "./api.gen";
import { ApiError, ServerUnreachableError } from "./errors";
import type { OperationId, Operations, ResponseKind } from "./api.gen";

const DEFAULT_BASE_URL = "http://127.0.0.1:3000";

/** Base URL of the apple-connector server, from extension preferences. */
export function baseUrl(): string {
	const preferences = getPreferenceValues<{ baseUrl?: string }>();
	return (preferences.baseUrl?.trim() || DEFAULT_BASE_URL).replace(/\/+$/, "");
}

/**
 * `never` marks an operation that takes no parameters of that kind, so the
 * corresponding option should be absent rather than required.
 */
type PathArg<K extends OperationId> = [Operations[K]["pathParams"]] extends [never]
	? { path?: never }
	: { path: Operations[K]["pathParams"] };
type QueryArg<K extends OperationId> = [Operations[K]["query"]] extends [never]
	? { query?: never }
	: { query?: Operations[K]["query"] };
type HeaderArg<K extends OperationId> = [Operations[K]["headers"]] extends [never]
	? { headers?: never }
	: { headers?: Operations[K]["headers"] };
type BodyArg<K extends OperationId> = [Operations[K]["body"]] extends [never]
	? { body?: never }
	: { body: Operations[K]["body"] };

export type RequestOptions<K extends OperationId> = PathArg<K> &
	QueryArg<K> &
	HeaderArg<K> &
	BodyArg<K> & { signal?: AbortSignal };

type ParamValue = string | number | boolean;

/**
 * Collect the scalar entries of a parameter object.
 *
 * Path, query and header values are always scalars in this contract, so
 * anything else is dropped rather than stringified into `[object Object]`.
 */
function scalarEntries(value: unknown): [string, ParamValue][] {
	if (typeof value !== "object" || value === null) {
		return [];
	}
	const entries: [string, ParamValue][] = [];
	for (const [key, item] of Object.entries(value)) {
		if (typeof item === "string" || typeof item === "number" || typeof item === "boolean") {
			entries.push([key, item]);
		}
	}
	return entries;
}

function asString(value: ParamValue): string {
	return typeof value === "string" ? value : String(value);
}

/** Substitute `{placeholder}` segments in a path template. */
function fillPath(template: string, params: unknown): string {
	const values = new Map(scalarEntries(params));
	return template.replace(/\{(\w+)\}/g, (_match, name: string) => {
		const value = values.get(name);
		if (value === undefined) {
			throw new Error(`missing path parameter "${name}" for ${template}`);
		}
		return encodeURIComponent(asString(value));
	});
}

/**
 * Serialize query params, dropping absent ones.
 *
 * Empty values are omitted rather than sent as `?q=`, which keeps a cleared
 * search box from binding the cursor to a different filter set.
 */
function fillQuery(query: unknown): string {
	const search = new URLSearchParams();
	for (const [key, value] of scalarEntries(query)) {
		const encoded = asString(value);
		if (encoded !== "") {
			search.set(key, encoded);
		}
	}
	const serialized = search.toString();
	return serialized ? `?${serialized}` : "";
}

/** Absolute URL for an operation. Exported so hooks can feed it to `useFetch`. */
export function urlFor<K extends OperationId>(id: K, options: RequestOptions<K>): string {
	const route = routes[id];
	return `${baseUrl()}${fillPath(route.path, options.path)}${fillQuery(options.query)}`;
}

/** Decode a successful response according to the contract's media type. */
async function decode(response: Response, kind: ResponseKind): Promise<unknown> {
	switch (kind) {
		case "json":
			return await response.json();
		case "text":
			return await response.text();
		case "binary":
			return await response.arrayBuffer();
		default:
			return undefined;
	}
}

/**
 * Perform a request against the API.
 *
 * Method, path template and response decoding all come from the generated route
 * table, so adding an endpoint to the contract makes it callable here with no
 * hand-written glue.
 */
export async function request<K extends OperationId>(
	id: K,
	options: RequestOptions<K>,
): Promise<Operations[K]["response"]> {
	const route = routes[id];
	const url = urlFor(id, options);

	const headers: Record<string, string> = { Accept: "application/json" };
	for (const [key, value] of scalarEntries(options.headers)) {
		headers[key] = asString(value);
	}
	if (options.body !== undefined) {
		headers["Content-Type"] = "application/json";
	}

	let response: Response;
	try {
		response = await fetch(url, {
			method: route.method,
			headers,
			body: options.body === undefined ? undefined : JSON.stringify(options.body),
			signal: options.signal,
		});
	} catch (cause) {
		if (cause instanceof Error && cause.name === "AbortError") {
			throw cause;
		}
		throw new ServerUnreachableError(baseUrl(), cause);
	}

	if (!response.ok) {
		throw await ApiError.fromResponse(response);
	}

	// `/openapi.json` is the one operation with no declared response schema, so the
	// generic return collapses to `unknown` and needs no assertion here. Concrete
	// call sites still resolve to their contract type.
	return await decode(response, route.kind);
}
