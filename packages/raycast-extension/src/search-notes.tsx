import { Action, ActionPanel, Color, Detail, Icon, List } from "@raycast/api";
import { useState } from "react";
import { ErrorState } from "./lib/ErrorState";
import { useApiItem, useApiList } from "./lib/hooks";
import { toDate } from "./lib/time";
import type { NoteSummaryDto } from "./lib/api.gen";

const ALL_FOLDERS = "";

/**
 * Locked notes never return decoded body text or ciphertext — only the snippet
 * may survive. Rendering an empty pane would read as "this note is blank", so
 * the locked state is stated explicitly.
 */
function NoteDetail({ note }: { note: NoteSummaryDto }) {
	const { data, isLoading, error } = useApiItem(
		"getNoteContents",
		{ path: { note_id: note.id } },
		{ execute: !note.is_locked },
	);

	const markdown = note.is_locked
		? `# 🔒 ${note.title}\n\nThis note is locked. The server never returns decoded text or ciphertext for locked notes.${
				note.snippet ? `\n\n> ${note.snippet}` : ""
			}`
		: error
			? `# ${note.title}\n\nCould not load this note.`
			: (data ?? "");

	const modified = toDate(note.modified_at);

	return (
		<Detail
			isLoading={isLoading}
			markdown={markdown}
			navigationTitle={note.title}
			metadata={
				<Detail.Metadata>
					<Detail.Metadata.Label title="Folder" text={note.folder_name ?? "Unknown"} />
					{modified ? <Detail.Metadata.Label title="Modified" text={modified.toLocaleString()} /> : null}
					<Detail.Metadata.TagList title="Flags">
						{note.is_pinned ? <Detail.Metadata.TagList.Item text="Pinned" color={Color.Yellow} /> : null}
						{note.is_locked ? <Detail.Metadata.TagList.Item text="Locked" color={Color.Red} /> : null}
						{note.has_checklist ? <Detail.Metadata.TagList.Item text="Checklist" color={Color.Blue} /> : null}
						{note.has_attachments ? <Detail.Metadata.TagList.Item text="Attachments" color={Color.Purple} /> : null}
					</Detail.Metadata.TagList>
				</Detail.Metadata>
			}
			actions={
				<ActionPanel>
					{!note.is_locked && data ? <Action.CopyToClipboard title="Copy Contents" content={data} /> : null}
					<Action.CopyToClipboard title="Copy Title" content={note.title} />
				</ActionPanel>
			}
		/>
	);
}

export default function SearchNotes() {
	const [search, setSearch] = useState("");
	const [folderId, setFolderId] = useState(ALL_FOLDERS);

	const folders = useApiList("listNoteFolders", { query: { limit: 200 } });
	// Recently Deleted is a folder kind, not a flag; excluding it here keeps
	// deleted notes out unless the user picks that folder explicitly.
	const selectableFolders = folders.data.filter(folder => folder.kind !== "smart");
	const showingDeleted = selectableFolders.find(folder => folder.id === folderId)?.kind === "deleted";

	// `q` is capped at 256 characters by the contract; sending more is a 400.
	const trimmed = search.trim().slice(0, 256);

	const { data, isLoading, error, pagination } = useApiList("listNotes", {
		query: {
			q: trimmed === "" ? undefined : trimmed,
			folder_id: folderId === ALL_FOLDERS ? undefined : folderId,
			include_deleted: showingDeleted ? true : undefined,
			limit: 50,
		},
	});

	return (
		<List
			isLoading={isLoading || folders.isLoading}
			searchText={search}
			onSearchTextChange={setSearch}
			searchBarPlaceholder="Search notes…"
			pagination={pagination}
			throttle
			isShowingDetail={false}
			searchBarAccessory={
				<List.Dropdown tooltip="Folder" value={folderId} onChange={setFolderId}>
					<List.Dropdown.Item title="All folders" value={ALL_FOLDERS} />
					{selectableFolders.map(folder => (
						<List.Dropdown.Item key={folder.id} title={folder.title} value={folder.id} />
					))}
				</List.Dropdown>
			}
		>
			{error ? (
				<ErrorState error={error} />
			) : (
				<>
					<List.EmptyView
						icon={Icon.Document}
						title={trimmed === "" ? "Search your notes" : "No matching notes"}
						description="Search covers note titles, snippets and decoded body text."
					/>
					{data.map(note => {
						const modified = toDate(note.modified_at);
						return (
							<List.Item
								key={note.id}
								icon={
									note.is_locked
										? { source: Icon.Lock, tintColor: Color.Red }
										: note.is_pinned
											? { source: Icon.Pin, tintColor: Color.Yellow }
											: Icon.Document
								}
								title={note.title}
								subtitle={note.snippet ?? undefined}
								accessories={[
									...(note.has_attachments ? [{ icon: Icon.Paperclip }] : []),
									...(note.has_checklist ? [{ icon: Icon.CheckList }] : []),
									{ text: note.folder_name ?? "Unknown" },
									...(modified ? [{ date: modified, tooltip: modified.toLocaleString() }] : []),
								]}
								actions={
									<ActionPanel>
										<Action.Push title="Open Note" icon={Icon.Eye} target={<NoteDetail note={note} />} />
										<Action.CopyToClipboard title="Copy Title" content={note.title} />
									</ActionPanel>
								}
							/>
						);
					})}
				</>
			)}
		</List>
	);
}
