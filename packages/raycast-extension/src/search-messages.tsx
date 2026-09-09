import { Action, ActionPanel, Color, Icon, List } from "@raycast/api";
import { useState } from "react";
import { DirectionFilterDtoValues, TransportFilterDtoValues } from "./lib/api.gen";
import { ErrorState } from "./lib/ErrorState";
import { useApiList } from "./lib/hooks";
import { contentIcon, handleLabel, messagesAppUrl, previewText, TRANSPORT_LABEL } from "./lib/messages";
import { toDate } from "./lib/time";
import type { DirectionFilterDto, TransportFilterDto } from "./lib/api.gen";

type Filter = { direction?: DirectionFilterDto; transport?: TransportFilterDto };

/** Encode both dropdowns in one value, since `List` allows a single accessory dropdown. */
function parseFilter(value: string): Filter {
	if (value === "all") {
		return {};
	}
	const [kind, item] = value.split(":");
	if (kind === "direction") {
		return { direction: DirectionFilterDtoValues.find(d => d === item) };
	}
	return { transport: TransportFilterDtoValues.find(t => t === item) };
}

export default function SearchMessages() {
	const [query, setQuery] = useState("");
	const [filterValue, setFilterValue] = useState("all");
	const filter = parseFilter(filterValue);

	// `q` is capped at 256 characters by the contract; sending more is a 400.
	const trimmedQuery = query.trim().slice(0, 256);

	const { data, isLoading, error, pagination } = useApiList("listMessages", {
		query: {
			q: trimmedQuery === "" ? undefined : trimmedQuery,
			direction: filter.direction,
			transport: filter.transport,
			limit: 50,
		},
	});

	return (
		<List
			isLoading={isLoading}
			searchText={query}
			onSearchTextChange={setQuery}
			searchBarPlaceholder="Search messages…"
			pagination={pagination}
			throttle
			searchBarAccessory={
				<List.Dropdown tooltip="Filter" value={filterValue} onChange={setFilterValue}>
					<List.Dropdown.Item title="All messages" value="all" />
					<List.Dropdown.Section title="Direction">
						{DirectionFilterDtoValues.map(direction => (
							<List.Dropdown.Item
								key={direction}
								title={direction === "sent" ? "Sent" : "Received"}
								value={`direction:${direction}`}
							/>
						))}
					</List.Dropdown.Section>
					<List.Dropdown.Section title="Transport">
						{TransportFilterDtoValues.map(transport => (
							<List.Dropdown.Item key={transport} title={TRANSPORT_LABEL[transport]} value={`transport:${transport}`} />
						))}
					</List.Dropdown.Section>
				</List.Dropdown>
			}
		>
			{error ? (
				<ErrorState error={error} />
			) : (
				<>
					<List.EmptyView
						icon={Icon.Message}
						title={trimmedQuery === "" ? "Search your messages" : "No matching messages"}
						description={
							trimmedQuery === ""
								? "Type to search across message text and decoded attributed bodies."
								: "Try a different term or clear the filter."
						}
					/>
					{data.map(message => {
						const sentAt = toDate(message.sent_at);
						const preview = previewText(message.content);
						const openUrl = messagesAppUrl(message.sender?.id);
						return (
							<List.Item
								key={message.guid}
								icon={{
									source: contentIcon(message.content),
									tintColor: message.direction === "sent" ? Color.Blue : Color.SecondaryText,
								}}
								title={preview}
								subtitle={handleLabel(message.sender, message.direction)}
								accessories={[
									{ tag: TRANSPORT_LABEL[message.transport] },
									...(sentAt ? [{ date: sentAt, tooltip: sentAt.toLocaleString() }] : []),
								]}
								actions={
									<ActionPanel>
										<Action.CopyToClipboard title="Copy Message Text" content={preview} />
										{openUrl ? (
											<Action.OpenInBrowser title="Open in Messages" icon={Icon.Message} url={openUrl} />
										) : null}
										<Action.CopyToClipboard
											title="Copy Message ID"
											content={message.guid}
											shortcut={{ modifiers: ["cmd", "shift"], key: "." }}
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
