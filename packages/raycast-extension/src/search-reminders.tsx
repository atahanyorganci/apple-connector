import { Action, ActionPanel, Color, confirmAlert, Icon, List, showToast, Toast } from "@raycast/api";
import { showFailureToast } from "@raycast/utils";
import { useState } from "react";
import { request } from "./lib/client";
import { remediationFor } from "./lib/errors";
import { ErrorState } from "./lib/ErrorState";
import { useApiList } from "./lib/hooks";
import { dueAccessory, priorityLabel, reminderIcon } from "./lib/reminders";
import type { ReminderSummaryDto } from "./lib/api.gen";

type Scope = "open" | "completed" | "flagged" | "overdue";

const SCOPES: { value: Scope; title: string }[] = [
	{ value: "open", title: "Open" },
	{ value: "completed", title: "Completed" },
	{ value: "flagged", title: "Flagged" },
	{ value: "overdue", title: "Overdue" },
];

function queryFor(scope: Scope, search: string) {
	const base = { q: search === "" ? undefined : search, limit: 50, include_tags: true };
	switch (scope) {
		case "completed":
			return { ...base, completed: true };
		case "flagged":
			return { ...base, completed: false, flagged: true };
		case "overdue":
			return { ...base, completed: false, has_due_date: true, due_before: Math.floor(Date.now() / 1000) };
		default:
			return { ...base, completed: false };
	}
}

export default function SearchReminders() {
	const [search, setSearch] = useState("");
	const [scope, setScope] = useState<Scope>("open");

	// `q` is capped at 256 characters by the contract; sending more is a 400.
	const trimmed = search.trim().slice(0, 256);
	const { data, isLoading, error, pagination, revalidate } = useApiList("listReminders", {
		query: queryFor(scope, trimmed),
	});

	async function toggleCompleted(reminder: ReminderSummaryDto) {
		const toast = await showToast({ style: Toast.Style.Animated, title: "Updating…" });
		try {
			await request("updateReminder", {
				path: { reminder_id: reminder.id },
				body: { completed: !reminder.completed },
			});
			toast.style = Toast.Style.Success;
			toast.title = reminder.completed ? "Marked incomplete" : "Completed";
			revalidate();
		} catch (failure) {
			await toast.hide();
			await showFailureToast(failure, { title: "Could not update reminder", message: remediationFor(failure) });
		}
	}

	async function deleteReminder(reminder: ReminderSummaryDto) {
		const confirmed = await confirmAlert({
			title: "Delete reminder?",
			message: reminder.title,
			icon: Icon.Trash,
			primaryAction: { title: "Delete" },
		});
		if (!confirmed) {
			return;
		}
		const toast = await showToast({ style: Toast.Style.Animated, title: "Deleting…" });
		try {
			await request("deleteReminder", { path: { reminder_id: reminder.id } });
			toast.style = Toast.Style.Success;
			toast.title = "Reminder deleted";
			revalidate();
		} catch (failure) {
			await toast.hide();
			await showFailureToast(failure, { title: "Could not delete reminder", message: remediationFor(failure) });
		}
	}

	return (
		<List
			isLoading={isLoading}
			searchText={search}
			onSearchTextChange={setSearch}
			searchBarPlaceholder="Search reminders…"
			pagination={pagination}
			throttle
			searchBarAccessory={
				<List.Dropdown
					tooltip="Scope"
					value={scope}
					onChange={value => setScope(SCOPES.find(item => item.value === value)?.value ?? "open")}
				>
					{SCOPES.map(item => (
						<List.Dropdown.Item key={item.value} value={item.value} title={item.title} />
					))}
				</List.Dropdown>
			}
		>
			{error ? (
				<ErrorState error={error} />
			) : (
				<>
					<List.EmptyView
						icon={Icon.CheckList}
						title="No reminders found"
						description="Try another scope or search term."
					/>
					{data.map(reminder => {
						const due = dueAccessory(reminder.due);
						const priority = priorityLabel(reminder.priority);
						return (
							<List.Item
								key={reminder.id}
								icon={reminderIcon(reminder)}
								title={reminder.title}
								subtitle={reminder.list_name}
								accessories={[
									...(reminder.tags?.length ? [{ tag: { value: reminder.tags.join(", "), color: Color.Blue } }] : []),
									...(priority ? [{ tag: priority }] : []),
									...(due ? [due] : []),
								]}
								actions={
									<ActionPanel>
										<Action
											title={reminder.completed ? "Mark as Incomplete" : "Mark as Completed"}
											icon={reminder.completed ? Icon.Circle : Icon.CheckCircle}
											onAction={() => void toggleCompleted(reminder)}
										/>
										<Action.CopyToClipboard title="Copy Title" content={reminder.title} />
										<Action
											title="Delete Reminder"
											icon={Icon.Trash}
											style={Action.Style.Destructive}
											shortcut={{ modifiers: ["ctrl"], key: "x" }}
											onAction={() => void deleteReminder(reminder)}
										/>
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
