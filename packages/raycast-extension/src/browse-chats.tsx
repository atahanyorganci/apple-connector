import { Action, ActionPanel, Color, Icon, List } from "@raycast/api";
import { ErrorState } from "./lib/ErrorState";
import { useApiList } from "./lib/hooks";
import { contentIcon, handleLabel, messagesAppUrl, previewText, TRANSPORT_LABEL } from "./lib/messages";
import { toDate } from "./lib/time";
import type { ChatId, ChatSummaryDto } from "./lib/api.gen";

function chatTitle(chat: ChatSummaryDto): string {
	return chat.display_name?.trim() || (chat.is_group ? "Group conversation" : chat.guid);
}

function ChatMessages({ chat }: { chat: ChatSummaryDto }) {
	const { data, isLoading, error, pagination } = useApiList("listChatMessages", {
		path: { chat_id: chat.id },
		query: { limit: 50 },
	});

	return (
		<List isLoading={isLoading} pagination={pagination} searchBarPlaceholder={`Filter ${chatTitle(chat)}…`}>
			{error ? (
				<ErrorState error={error} />
			) : (
				<>
					<List.EmptyView icon={Icon.Message} title="No messages in this conversation" />
					{data.map(message => {
						const sentAt = toDate(message.sent_at);
						const preview = previewText(message.content);
						return (
							<List.Item
								key={message.guid}
								icon={{
									source: contentIcon(message.content),
									tintColor: message.direction === "sent" ? Color.Blue : Color.SecondaryText,
								}}
								title={preview}
								subtitle={handleLabel(message.sender, message.direction)}
								accessories={sentAt ? [{ date: sentAt, tooltip: sentAt.toLocaleString() }] : []}
								actions={
									<ActionPanel>
										<Action.CopyToClipboard title="Copy Message Text" content={preview} />
										<Action.CopyToClipboard title="Copy Message ID" content={message.guid} />
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

export default function BrowseChats() {
	const { data, isLoading, error, pagination } = useApiList("listChats", { query: { limit: 50 } });

	return (
		<List isLoading={isLoading} pagination={pagination} searchBarPlaceholder="Filter conversations…">
			{error ? (
				<ErrorState error={error} />
			) : (
				<>
					<List.EmptyView icon={Icon.TwoPeople} title="No conversations found" />
					{data.map(chat => {
						const openUrl = messagesAppUrl(chat.guid.split(";").pop());
						return (
							<List.Item
								key={String(chat.id satisfies ChatId)}
								icon={chat.is_group ? Icon.TwoPeople : Icon.Person}
								title={chatTitle(chat)}
								accessories={[
									{ tag: TRANSPORT_LABEL[chat.transport] },
									...(chat.is_group ? [{ text: `${chat.participant_count} people` }] : []),
								]}
								actions={
									<ActionPanel>
										<Action.Push title="Show Messages" icon={Icon.Message} target={<ChatMessages chat={chat} />} />
										{openUrl ? (
											<Action.OpenInBrowser title="Open in Messages" icon={Icon.Message} url={openUrl} />
										) : null}
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
