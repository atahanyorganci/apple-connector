import { Action, ActionPanel, Clipboard, Color, Detail, Icon, List } from "@raycast/api";
import { showFailureToast, useCachedPromise } from "@raycast/utils";
import { useState } from "react";
import { fetchToFile } from "./lib/attachments";
import { request, urlFor } from "./lib/client";
import { contactInitials, contactName } from "./lib/contacts";
import { ApiError } from "./lib/errors";
import { ErrorState } from "./lib/ErrorState";
import { useApiItem, useApiList } from "./lib/hooks";
import type { ContactAddressDto, ContactSummaryDto } from "./lib/api.gen";

const ALL = "";

function formatAddress(address: ContactAddressDto): string {
	return [address.street, address.city, address.state, address.postal_code, address.country]
		.map(part => part?.trim())
		.filter(Boolean)
		.join(", ");
}

/**
 * Photo bytes have to land on disk before Raycast can render them, and a
 * contact without a photo answers `contact_photo_not_found`. That is an
 * expected outcome rather than a failure, so it resolves to no photo instead of
 * surfacing an error.
 */
function useContactPhoto(contact: ContactSummaryDto, hasPhoto: boolean) {
	const { data } = useCachedPromise(
		async (contactId: string) => {
			try {
				return await fetchToFile(
					urlFor("getContactPhoto", { path: { contact_id: contactId } }),
					`contact-photo:${contactId}`,
					"jpg",
				);
			} catch (error) {
				// A contact without a photo is an expected outcome, not a failure.
				if (error instanceof ApiError && error.code === "contact_photo_not_found") {
					return undefined;
				}
				throw error;
			}
		},
		[contact.id],
		{ execute: hasPhoto },
	);
	return data;
}

function ContactDetail({ contact }: { contact: ContactSummaryDto }) {
	const { data, isLoading, error } = useApiItem("getContact", { path: { contact_id: contact.id } });
	const photoPath = useContactPhoto(contact, data?.has_photo ?? false);

	const name = contactName(contact);
	const primaryPhone = data?.phones.find(phone => phone.is_primary) ?? data?.phones[0];
	const primaryEmail = data?.emails.find(email => email.is_primary) ?? data?.emails[0];

	const markdown = error
		? `# ${name}\n\nCould not load this contact.`
		: [
				photoPath
					? `![${name}](${encodeURI(`file://${photoPath}`)}?raycast-height=180)`
					: `# ${contactInitials(contact)}`,
				`# ${name}`,
				data?.job_title || data?.organization
					? `_${[data.job_title, data.organization].filter(Boolean).join(" · ")}_`
					: "",
				data?.note ? `\n${data.note}` : "",
			]
				.filter(Boolean)
				.join("\n\n");

	return (
		<Detail
			isLoading={isLoading}
			markdown={markdown}
			navigationTitle={name}
			metadata={
				data ? (
					<Detail.Metadata>
						{data.phones.map(phone => (
							<Detail.Metadata.Label key={phone.id} title={phone.label ?? "Phone"} text={phone.number} />
						))}
						{data.emails.map(email => (
							<Detail.Metadata.Label key={email.id} title={email.label ?? "Email"} text={email.address} />
						))}
						{data.addresses.map(address => (
							<Detail.Metadata.Label
								key={address.id}
								title={address.label ?? "Address"}
								text={formatAddress(address)}
							/>
						))}
						{data.urls.map(url => (
							<Detail.Metadata.Link key={url.id} title={url.label ?? "URL"} target={url.url} text={url.url} />
						))}
					</Detail.Metadata>
				) : null
			}
			actions={
				<ActionPanel>
					{primaryPhone ? (
						<>
							<Action.OpenInBrowser
								title="Call"
								icon={Icon.Phone}
								url={`tel:${encodeURIComponent(primaryPhone.number)}`}
							/>
							<Action.OpenInBrowser
								title="Send Message"
								icon={Icon.Message}
								url={`imessage://${encodeURIComponent(primaryPhone.number)}`}
							/>
						</>
					) : null}
					{primaryEmail ? (
						<Action.OpenInBrowser
							title="Send Email"
							icon={Icon.Envelope}
							url={`mailto:${encodeURIComponent(primaryEmail.address)}`}
						/>
					) : null}
					<Action
						title="Copy vCard"
						icon={Icon.Clipboard}
						shortcut={{ modifiers: ["cmd", "shift"], key: "c" }}
						onAction={() => {
							void (async () => {
								try {
									const vcard = await request("getContactVcard", { path: { contact_id: contact.id } });
									await Clipboard.copy(vcard);
								} catch (failure) {
									await showFailureToast(failure, { title: "Could not copy vCard" });
								}
							})();
						}}
					/>
				</ActionPanel>
			}
		/>
	);
}

export default function SearchContacts() {
	const [search, setSearch] = useState("");
	const [scope, setScope] = useState(ALL);

	// `ContainerPageDto` has no `page` field — containers are not paginated.
	const containers = useApiItem("listContainers", {});
	const groups = useApiList("listGroups", { query: { limit: 200 } });

	const [kind, id] = scope === ALL ? [undefined, undefined] : scope.split(":");

	const { data, isLoading, error, pagination } = useApiList("searchContacts", {
		query: {
			q: search.trim().slice(0, 256),
			container_id: kind === "container" ? id : undefined,
			group_id: kind === "group" ? id : undefined,
			limit: 50,
		},
	});

	return (
		<List
			isLoading={isLoading}
			searchText={search}
			onSearchTextChange={setSearch}
			searchBarPlaceholder="Search contacts…"
			pagination={pagination}
			throttle
			searchBarAccessory={
				<List.Dropdown tooltip="Scope" value={scope} onChange={setScope}>
					<List.Dropdown.Item title="All contacts" value={ALL} />
					<List.Dropdown.Section title="Containers">
						{(containers.data?.items ?? []).map(container => (
							<List.Dropdown.Item
								key={container.id}
								title={container.name ?? "Unnamed container"}
								value={`container:${container.id}`}
							/>
						))}
					</List.Dropdown.Section>
					<List.Dropdown.Section title="Groups">
						{groups.data.map(group => (
							<List.Dropdown.Item key={group.id} title={group.name ?? "Unnamed group"} value={`group:${group.id}`} />
						))}
					</List.Dropdown.Section>
				</List.Dropdown>
			}
		>
			{error ? (
				<ErrorState error={error} />
			) : (
				<>
					<List.EmptyView icon={Icon.Person} title="No contacts found" />
					{data.map(contact => (
						<List.Item
							key={contact.id}
							icon={{ source: Icon.Person, tintColor: Color.SecondaryText }}
							title={contactName(contact)}
							subtitle={contact.organization ?? undefined}
							actions={
								<ActionPanel>
									<Action.Push title="Show Details" icon={Icon.Sidebar} target={<ContactDetail contact={contact} />} />
									<Action.CopyToClipboard title="Copy Name" content={contactName(contact)} />
								</ActionPanel>
							}
						/>
					))}
				</>
			)}
		</List>
	);
}
