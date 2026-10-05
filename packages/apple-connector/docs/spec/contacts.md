# Contacts: AddressBook mapping

Reads from SQLite; writes through the Contacts framework (`apple-contacts`). Source: every
`~/Library/Application Support/AddressBook/Sources/<source>/` directory, using its newest
`AddressBook-v*.abcddb` (`--contacts-sources-dir`, `APPLE_CONNECTOR_CONTACTS_SOURCES_DIR`).

## Tables and entities

Contacts, groups, and containers are rows of `ZABCDRECORD`, distinguished by `Z_ENT`; the entity
numbers for `ABCDContact`, `ABCDGroup`, and `CNCDContainer`, and the group-membership join table,
are discovered per source at startup ([CONN-0020](../decisions/CONN-0020-schema-fail-fast.md)).
Details: `ZABCDPHONENUMBER`, `ZABCDEMAILADDRESS`, `ZABCDPOSTALADDRESS`, `ZABCDURLADDRESS`,
`ZABCDSOCIALPROFILE`, `ZABCDNOTE`, `ZABCDLIKENESS` (photos).

## Identifiers

`ContactId`, `GroupId`, `ContainerId`: the part of `ZUNIQUEID` before the first `:`
(`ZUNIQUEID` is `<identifier>:<suffix>`), compared case-insensitively. `SourceId` is the source
directory name.

## Rows

- Order within a source: `Z_PK DESC`. Sources are read one after another in ascending `SourceId`;
  the cursor records the source and its row ([CONN-0005](../decisions/CONN-0005-keyset-pagination.md)).
- A contact's container is `COALESCE(ZCONTAINER, <the source's only CNCDContainer>)`; with several
  containers and no stored one, `container_id` is `null` and the contact is still listed
  ([CONN-0024](../decisions/CONN-0024-contact-container.md),
  [CONN-L-0002](../lessons/CONN-L-0002-implied-container.md)).
- `q` is a `LIKE` match on first name, last name, organization, and name; `%` and `_` are wildcards.
- Birthdays are Core Data seconds. A birthday entered without a year is stored in year 1604,
  Apple's convention, and returned as such
  ([CONN-L-0006](../lessons/CONN-L-0006-apple-date-sentinels.md)).
- Containers carry no writability flag ([CN-0002](../../../apple-contacts/docs/decisions/CN-0002-container-writability.md)).

## Formats

JSON on the base routes; vCard on `…/vcard` and CardDAV XML on `…/carddav`, through `serde-vcard`
and `serde-carddav`. `GET /v1/contacts/{id}/photo` returns the primary `ZABCDLIKENESS` image, or
the record's `ZIMAGEDATA` when there is none. It matches the record with
`LIKE '<id>:' || '%'`, so `%` and `_` in the id are wildcards (known bug).

## Writes

`POST /v1/containers/{id}/contacts`, `PATCH`/`DELETE /v1/contacts/{id}`;
`POST /v1/containers/{id}/groups`, `PATCH`/`DELETE /v1/groups/{id}`;
`POST`/`DELETE /v1/groups/{id}/contacts/{contact_id}`. JSON bodies only. A container the framework
refuses is `403 read_only_container`; an ambiguous container is `409 ambiguous_contacts_match`.
