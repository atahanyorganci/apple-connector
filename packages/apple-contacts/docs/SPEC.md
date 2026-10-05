# `apple-contacts`: Contacts framework writes for contacts and groups

Creates, updates, and deletes contacts and groups, and changes group membership, through
`CNContactStore`; reports Contacts authorization. `apple-connector` uses it for every Contacts
write; reads never go through it ([REC-0005](../../../docs/decisions/REC-0005-hybrid-read-write.md)).
All Objective-C and `unsafe` code for Contacts is confined here.

## Kind

Framework.

## Public API

- `ContactsStore`, a cloneable handle (`new()`); clones share one worker and one authorization
  snapshot.
  - Authorization: `auth_status()`, `refresh_auth_status()`, `request_access()` → `AuthOutcome`,
    `ensure_contacts_access()`.
  - Contacts: `create_contact(ContainerResolveHint, CreateContactInput)`,
    `update_contact(contact_id, UpdateContactInput)`, `delete_contact(contact_id)` →
    `SavedContact { identifier }`.
  - Groups: `create_group(ContainerResolveHint, CreateGroupInput)`,
    `update_group(group_id, UpdateGroupInput)`, `delete_group(group_id)`,
    `add_contact_to_group(group_id, contact_id)`, `remove_contact_from_group(group_id, contact_id)`.
- Inputs: `CreateContactInput`, `UpdateContactInput`, `LabeledStringInput`, `PostalAddressInput`,
  `CreateGroupInput`, `UpdateGroupInput`; `ContainerResolveHint { api_id, external_id, name }`,
  `ContainerResolveMetadata`, `ContainerStoreType`.
- `AuthStatus`, `AuthOutcome`.
- `ContactsError`: `NotFound`, `AccessDenied`, `ReadOnlyContainer`, `ValidationFailed(String)`,
  `AmbiguousMatch(String)`, `UnsupportedPlatform`, `Framework(String)`, `Timeout`.

## Guarantees

- The `CNContactStore` is created on, owned by, and only used from one dedicated thread, with the
  same worker as `apple-eventkit` ([REC-0014](../../../docs/decisions/REC-0014-framework-worker.md)).
  Enforced by: `src/worker.rs` tests; `src/worker.rs` is identical to
  `apple-eventkit/src/worker.rs`.
- Writability is decided by the framework at save time; there is no writability hint
  ([CN-0002](decisions/CN-0002-container-writability.md)). `RecordNotWritable`,
  `ParentContainerNotWritable`, and `NoAccessableWritableContainers` → `ReadOnlyContainer`.
  Enforced by: `src/error.rs::not_writable_codes_map_to_read_only_container`.
- Container resolution: stored external identifier, then API id, then a case-insensitive name
  match; only a genuine miss falls through, a container returned under another identifier is
  `NotFound`, and two matches are `AmbiguousMatch`
  ([CN-0003](decisions/CN-0003-ambiguous-match.md)). Enforced by: code (`src/container.rs`);
  `apple-connector/src/api/contacts_convert.rs::ambiguity_answers_the_same_way_as_eventkit` for the
  HTTP mapping.
- A new contact must have a non-blank given name, family name, or organization. Enforced by:
  `validate_create_contact_input` in `src/contact.rs`.
- `CNErrorDomain` errors are classified: `RecordDoesNotExist` → `NotFound`,
  `AuthorizationDenied` → `AccessDenied`, validation codes → `ValidationFailed` with the
  framework's `localizedDescription`, everything else → `Framework`. Enforced by: `src/error.rs`
  tests.
- `Limited` authorization counts as granted. Enforced by: `src/auth.rs`.
- The crate does not build off macOS. Enforced by: `compile_error!` in `src/lib.rs`.

### Not yet enforced

- Container resolution has no unit test of its own; it needs a live store.
- The live round trip (`integration_contacts_mutations_on_macos` in
  `apple-connector/tests/contacts_integration.rs`) is ignored and runs only by hand.

### Known bugs

None known in this crate. The HTTP layer forwards `ValidationFailed` text to clients (see the
apple-connector spec).

## Limits and non-goals

- Writable fields: given, family, and middle name, nickname, organization, job title, department,
  note, phone numbers, email addresses, postal addresses, and URLs. Photos, birthdays, social
  profiles, and relations are not writable.
- A list given on update (phones, emails, addresses, URLs) replaces the stored list.
- Updates and deletes address the unified contact (`unifiedContactWithIdentifier:`).
- No smart-group writes.
- No reads: the framework is never queried for data the API returns.

## Platform and permissions

macOS only. Contacts TCC grant for the process (prompted at startup when not yet determined).
