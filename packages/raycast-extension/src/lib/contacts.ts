import type { ContactSummaryDto } from "./api.gen";

/** Best available display name, falling back through the name parts. */
export function contactName(contact: ContactSummaryDto): string {
	const display = contact.display_name?.trim();
	if (display) {
		return display;
	}
	const parts = [contact.first_name, contact.last_name].map(part => part?.trim()).filter(Boolean);
	if (parts.length > 0) {
		return parts.join(" ");
	}
	return contact.organization?.trim() || "Unnamed contact";
}

/** Initials for the avatar fallback when a contact has no photo. */
export function contactInitials(contact: ContactSummaryDto): string {
	const name = contactName(contact);
	const initials = name
		.split(/\s+/)
		.slice(0, 2)
		.map(part => part.charAt(0).toUpperCase())
		.join("");
	return initials || "?";
}
