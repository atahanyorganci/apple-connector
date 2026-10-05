import { Action, ActionPanel, Form, Icon, popToRoot, showToast, Toast } from "@raycast/api";
import { showFailureToast } from "@raycast/utils";
import { useState } from "react";
import { RecurrenceFrequencyDtoValues } from "./lib/api.gen";
import { request } from "./lib/client";
import { remediationFor } from "./lib/errors";
import { useApiList } from "./lib/hooks";
import { PRIORITIES } from "./lib/reminders";
import { toUnixSeconds } from "./lib/time";
import type { CreateReminderRequest, RecurrenceFrequencyDto } from "./lib/api.gen";

type FormValues = {
	title: string;
	notes: string;
	listId: string;
	dueDate: Date | null;
	allDay: boolean;
	priority: string;
	url: string;
	recurrence: string;
	recurrenceInterval: string;
};

const NO_RECURRENCE = "none";

export default function CreateReminder() {
	const [listId, setListId] = useState("");

	const lists = useApiList("listReminderLists", { query: { limit: 200 } });

	// Smart lists cannot be written to (`smart_list_read_only`), so they are not
	// offered as a target at all rather than failing on submit.
	const writableLists = lists.data.filter(list => list.kind !== "smart");
	const selectedList = listId === "" ? writableLists[0] : writableLists.find(list => list.id === listId);

	async function onSubmit(values: FormValues) {
		const targetList = values.listId || selectedList?.id;
		if (!targetList) {
			await showFailureToast(new Error("Pick a list first."), { title: "No list selected" });
			return;
		}

		const frequency = RecurrenceFrequencyDtoValues.find(
			(value): value is RecurrenceFrequencyDto => value === values.recurrence,
		);
		const interval = Number.parseInt(values.recurrenceInterval, 10);

		// Only fields EventKit can store. Sections, flags, tags, subtasks and
		// attachments are readable through the API but rejected on write with
		// `unsupported_reminder_field`, so the form does not offer them.
		const body: CreateReminderRequest = {
			title: values.title.trim(),
			notes: values.notes.trim() || null,
			priority: Number.parseInt(values.priority, 10),
			url: values.url.trim() || null,
			due: values.dueDate ? { at: toUnixSeconds(values.dueDate), all_day: values.allDay } : null,
			recurrence: frequency ? { frequency, interval: Number.isFinite(interval) && interval > 0 ? interval : 1 } : null,
		};

		const toast = await showToast({ style: Toast.Style.Animated, title: "Creating reminder…" });
		try {
			await request("createReminder", { path: { list_id: targetList }, body });
			toast.style = Toast.Style.Success;
			toast.title = "Reminder created";
			await popToRoot();
		} catch (error) {
			await toast.hide();
			await showFailureToast(error, {
				title: "Could not create reminder",
				message: remediationFor(error),
			});
		}
	}

	return (
		<Form
			isLoading={lists.isLoading}
			actions={
				<ActionPanel>
					<Action.SubmitForm title="Create Reminder" icon={Icon.Plus} onSubmit={onSubmit} />
				</ActionPanel>
			}
		>
			<Form.TextField id="title" title="Title" placeholder="Reminder title" autoFocus />
			<Form.TextArea id="notes" title="Notes" placeholder="Optional notes" />

			<Form.Dropdown id="listId" title="List" value={selectedList?.id ?? ""} onChange={setListId}>
				{writableLists.map(list => (
					<Form.Dropdown.Item key={list.id} value={list.id} title={list.name} />
				))}
			</Form.Dropdown>

			<Form.Separator />

			<Form.DatePicker id="dueDate" title="Due" />
			<Form.Checkbox id="allDay" label="All day" />
			<Form.Dropdown id="recurrence" title="Repeat" defaultValue={NO_RECURRENCE}>
				<Form.Dropdown.Item value={NO_RECURRENCE} title="Never" />
				{RecurrenceFrequencyDtoValues.map(frequency => (
					<Form.Dropdown.Item key={frequency} value={frequency} title={frequency} />
				))}
			</Form.Dropdown>
			<Form.TextField id="recurrenceInterval" title="Every" placeholder="1" defaultValue="1" />

			<Form.Separator />

			<Form.Dropdown id="priority" title="Priority" defaultValue="0">
				{PRIORITIES.map(priority => (
					<Form.Dropdown.Item key={priority.value} value={String(priority.value)} title={priority.label} />
				))}
			</Form.Dropdown>
			<Form.TextField id="url" title="URL" placeholder="https://…" />
		</Form>
	);
}
