import { clampLimit, truncate } from "../lib/ai";
import { request } from "../lib/client";

type Input = {
	/** Text to search for across note titles, snippets and decoded body text. */
	query?: string;
	/** Restrict to a single folder by id. */
	folderId?: string;
	/** Only pinned notes when true. */
	pinned?: boolean;
	/** Maximum number of notes to return. Defaults to 25, which is also the cap. */
	limit?: number;
};

/**
 * Search the user's Apple Notes.
 *
 * Returns metadata and snippets only. Use `get-note-contents` for a note's full
 * Markdown body. Locked notes never expose body text.
 */
export default async function searchNotes(input: Input) {
	const page = await request("listNotes", {
		query: {
			q: truncate(input.query, 256),
			folder_id: input.folderId,
			is_pinned: input.pinned,
			limit: clampLimit(input.limit),
		},
	});

	return page.items.map(note => ({
		id: note.id,
		title: note.title,
		folder: note.folder_name,
		snippet: truncate(note.snippet, 200),
		isLocked: note.is_locked,
		isPinned: note.is_pinned,
		modifiedAt: note.modified_at,
	}));
}
