import { Action, ActionPanel, Icon, List, openExtensionPreferences } from "@raycast/api";
import { remediationFor, titleFor } from "./errors";

/**
 * Empty state for a failed request.
 *
 * Most failures here are environmental — the server is not running, or a TCC
 * grant is missing — so the state names the remedy instead of echoing an error
 * code the user cannot act on.
 */
export function ErrorState({ error }: { error: unknown }) {
	const remediation = remediationFor(error);
	return (
		<List.EmptyView
			icon={Icon.Plug}
			title={titleFor(error)}
			description={remediation ?? "Check that the apple-connector server is running."}
			actions={
				<ActionPanel>
					<Action title="Open Extension Preferences" icon={Icon.Gear} onAction={openExtensionPreferences} />
				</ActionPanel>
			}
		/>
	);
}
