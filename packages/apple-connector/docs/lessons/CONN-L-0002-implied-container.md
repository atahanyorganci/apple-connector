---
id: CONN-L-0002
status: graduated
observed-on: "macOS 27 (not checked on 26); AddressBook-v22.abcddb ZABCDRECORD.ZCONTAINER"
graduated-to: "packages/apple-connector/tests/contacts_integration.rs"
provenance:
  - https://github.com/atahanyorganci/apple-connector/issues/153
  - https://github.com/atahanyorganci/apple-connector/pull/154
  - https://github.com/atahanyorganci/apple-connector/commit/201a0cb
  - https://github.com/atahanyorganci/apple-connector/commit/1d06251
---

# In an AddressBook source with one container, contact rows leave `ZCONTAINER` NULL; only groups set it.

## What happened

`GET /v1/contacts`, `/v1/contacts/search`, and `/v1/groups/{id}/contacts` returned empty pages on
an AddressBook with 1005 contacts, while `/healthz` was ok and `GET /v1/contacts/{id}` worked.

## Why

The list paths dropped any row without a container (since #113). macOS does not store a contact's
container when the source has a single one; the container is implied by the source database.

| Source | Contact rows | `ZCONTAINER IS NULL` | `CNCDContainer` rows |
| --- | --- | --- | --- |
| iCloud | 1005 | 1005 | 1 |
| two other sources | 0 | — | 1 each |

Group rows in the same source did set `ZCONTAINER`.

## Rule

A contact's container is `COALESCE(ZCONTAINER, <the source's only CNCDContainer>)`; when a source
has several containers and the row stores none, the contact has no container — and is still
listed.

## Graduated

`packages/apple-connector/tests/contacts_integration.rs`:

- `integration_contacts_with_null_container_are_listed`: a contact with `ZCONTAINER` NULL
  (`SEED_UNCONTAINED_CONTACT_ID`) appears in list, search, the `container_id` filter, group
  members, vCard export, and detail.
- `integration_contacts_with_ambiguous_container_are_listed_without_one`: with a second container,
  it is listed with `container_id: null`.
