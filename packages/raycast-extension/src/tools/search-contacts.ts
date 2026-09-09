import { clampLimit, truncate } from "../lib/ai";
import { request } from "../lib/client";
import { contactName } from "../lib/contacts";

type Input = {
	/** Name, organization or other text to search contacts for. */
	query?: string;
	/** Restrict to a single contact container by id. */
	containerId?: string;
	/** Restrict to a single contact group by id. */
	groupId?: string;
	/** Maximum number of contacts to return. Defaults to 25, which is also the cap. */
	limit?: number;
};

/** Search the user's contacts. */
export default async function searchContacts(input: Input) {
	const page = await request("searchContacts", {
		query: {
			q: truncate(input.query, 256) ?? "",
			container_id: input.containerId,
			group_id: input.groupId,
			limit: clampLimit(input.limit),
		},
	});

	return page.items.map(contact => ({
		id: contact.id,
		name: contactName(contact),
		organization: contact.organization,
	}));
}
