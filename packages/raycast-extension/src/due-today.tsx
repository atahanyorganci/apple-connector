import { Color, Icon, launchCommand, LaunchType, MenuBarExtra, open, showToast, Toast } from "@raycast/api";
import { showFailureToast } from "@raycast/utils";
import { request } from "./lib/client";
import { remediationFor, titleFor } from "./lib/errors";
import { useApiItem } from "./lib/hooks";
import { endOfToday, isOverdue } from "./lib/reminders";
import { toUnixSeconds } from "./lib/time";
import type { ReminderSummaryDto } from "./lib/api.gen";

export default function DueToday() {
	const { data, isLoading, error, revalidate } = useApiItem("listReminders", {
		query: {
			completed: false,
			has_due_date: true,
			due_before: toUnixSeconds(endOfToday()),
			limit: 50,
		},
	});

	const reminders = data?.items ?? [];
	const overdue = reminders.filter(reminder => isOverdue(reminder.due));
	const today = reminders.filter(reminder => !isOverdue(reminder.due));

	async function complete(reminder: ReminderSummaryDto) {
		try {
			await request("updateReminder", { path: { reminder_id: reminder.id }, body: { completed: true } });
			await showToast({ style: Toast.Style.Success, title: "Completed", message: reminder.title });
			// Refresh so the menu bar count reflects the change immediately.
			revalidate();
		} catch (failure) {
			await showFailureToast(failure, { title: "Could not complete reminder", message: remediationFor(failure) });
		}
	}

	const title = reminders.length === 0 ? undefined : String(reminders.length);

	return (
		<MenuBarExtra
			icon={{
				source: overdue.length > 0 ? Icon.ExclamationMark : Icon.CheckList,
				tintColor: overdue.length > 0 ? Color.Red : undefined,
			}}
			title={title}
			tooltip="Reminders due today"
			// Raycast keeps the command loaded while this is true and unloads it once
			// the request settles; leaving it stuck would leak the process.
			isLoading={isLoading}
		>
			{error ? (
				<MenuBarExtra.Section title="Unavailable">
					<MenuBarExtra.Item
						title={titleFor(error)}
						subtitle={remediationFor(error)}
						onAction={() => void open("raycast://extensions/atahanyorganci/apple-connector")}
					/>
				</MenuBarExtra.Section>
			) : (
				<>
					{overdue.length > 0 ? (
						<MenuBarExtra.Section title="Overdue">
							{overdue.map(reminder => (
								<MenuBarExtra.Item
									key={reminder.id}
									icon={{ source: Icon.ExclamationMark, tintColor: Color.Red }}
									title={reminder.title}
									subtitle={reminder.list_name}
									onAction={() => void complete(reminder)}
								/>
							))}
						</MenuBarExtra.Section>
					) : null}

					{today.length > 0 ? (
						<MenuBarExtra.Section title="Due today">
							{today.map(reminder => (
								<MenuBarExtra.Item
									key={reminder.id}
									icon={Icon.Circle}
									title={reminder.title}
									subtitle={reminder.list_name}
									onAction={() => void complete(reminder)}
								/>
							))}
						</MenuBarExtra.Section>
					) : null}

					{reminders.length === 0 ? (
						<MenuBarExtra.Section>
							<MenuBarExtra.Item title="Nothing due today" icon={Icon.CheckCircle} />
						</MenuBarExtra.Section>
					) : null}
				</>
			)}

			<MenuBarExtra.Section>
				<MenuBarExtra.Item
					title="Search Reminders…"
					icon={Icon.MagnifyingGlass}
					onAction={() => void launchCommand({ name: "search-reminders", type: LaunchType.UserInitiated })}
				/>
				<MenuBarExtra.Item
					title="New Reminder…"
					icon={Icon.Plus}
					onAction={() => void launchCommand({ name: "create-reminder", type: LaunchType.UserInitiated })}
				/>
			</MenuBarExtra.Section>
		</MenuBarExtra>
	);
}
