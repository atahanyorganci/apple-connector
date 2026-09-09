import { truncate } from "../lib/ai";
import { request } from "../lib/client";

type Input = {
	/** Identifier of the note, as returned by `search-notes`. */
	noteId: string;
	/** Maximum characters of body text to return. Defaults to 4000. */
	maxLength?: number;
};

/**
 * Read a note's full contents as Markdown.
 *
 * Locked notes never return decoded text or ciphertext; asking for one yields
 * an explicit locked result rather than an empty body.
 */
export default async function getNoteContents(input: Input) {
	const note = await request("getNote", { path: { note_id: input.noteId } });
	if (note.is_locked) {
		return { id: note.id, title: note.title, isLocked: true, contents: undefined };
	}

	const contents = await request("getNoteContents", { path: { note_id: input.noteId } });
	return {
		id: note.id,
		title: note.title,
		isLocked: false,
		contents: truncate(contents, input.maxLength ?? 4000),
	};
}
