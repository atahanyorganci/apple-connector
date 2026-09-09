import { useCachedPromise, useFetch } from "@raycast/utils";
import { request as apiRequest, urlFor } from "./client";
import { ApiError } from "./errors";
import type { OperationId, Operations, PageMetaDto } from "./api.gen";
import type { RequestOptions } from "./client";

/** Operations that return a keyset-paginated `{ items, page }` envelope. */
export type ListOperationId = {
	[K in OperationId]: Operations[K]["response"] extends { items: unknown[]; page: { has_more: boolean } } ? K : never;
}[OperationId];

type ItemOf<K extends ListOperationId> = Operations[K]["response"] extends { items: (infer T)[] } ? T : never;

/**
 * Paginated list backed by a contract operation.
 *
 * The API is keyset-only: pass `page.next_cursor` back as `cursor` while
 * `page.has_more` is true. Cursors are bound to the active filter set, so the
 * URL carries the filters and a changed URL restarts paging from the first
 * page — reusing a cursor across a filter change is rejected with
 * `validation_error`.
 */
export function useApiList<K extends ListOperationId>(
	id: K,
	request: RequestOptions<K>,
	options: { execute?: boolean } = {},
) {
	type Item = ItemOf<K>;
	type Envelope = { items: Item[]; page: PageMetaDto };

	return useFetch<Envelope, Item[], Item[]>(
		pagination => {
			const url = urlFor(id, request);
			if (typeof pagination.cursor !== "string" || pagination.cursor === "") {
				return url;
			}
			const separator = url.includes("?") ? "&" : "?";
			return `${url}${separator}cursor=${encodeURIComponent(pagination.cursor)}`;
		},
		{
			async parseResponse(response): Promise<Envelope> {
				if (!response.ok) {
					throw await ApiError.fromResponse(response);
				}
				const body: unknown = await response.json();
				if (typeof body !== "object" || body === null || !("items" in body) || !("page" in body)) {
					throw new Error("Malformed page response: expected `items` and `page`.");
				}
				// The one place untyped JSON becomes contract-typed. The envelope shape is
				// checked above; item types come from `docs/openapi.json`.
				// oxlint-disable-next-line typescript/no-unsafe-type-assertion
				return body as Envelope;
			},
			mapResult(result) {
				return {
					data: result.items,
					hasMore: result.page.has_more,
					cursor: result.page.next_cursor ?? undefined,
				};
			},
			// Avoids a flash of empty list while the next page loads.
			keepPreviousData: true,
			initialData: [],
			execute: options.execute,
		},
	);
}

/**
 * Single (non-paginated) operation.
 *
 * Args are deep-compared by `useCachedPromise`, so passing a fresh options
 * object each render does not re-fire the request.
 */
export function useApiItem<K extends OperationId>(
	id: K,
	options: RequestOptions<K>,
	config: { execute?: boolean } = {},
) {
	return useCachedPromise(
		async (operationId: K, requestOptions: RequestOptions<K>) => await apiRequest(operationId, requestOptions),
		[id, options] as const,
		{ execute: config.execute },
	);
}
