# `serde-carddav`: RFC 6352 CardDAV multistatus reader and writer

Reads and writes CardDAV `multistatus` XML documents whose `address-data` holds a vCard,
delegating the vCard text to `serde-vcard`. `apple-connector` uses it for the `/carddav` contact
routes. It describes resources; it is not a CardDAV server or client.

## Kind

Codec.

## Public API

- `parse_multistatus(&[u8])`, `from_str(&str)`, `from_slice(&[u8])`, `from_reader(R)`,
  `from_reader_with_limit(R, limit)` → `Result<CardDavMultistatus>`.
- `parse_address_object(&[u8]) -> Result<CardDavAddressObject>`: the first response with address
  data.
- `multistatus_to_string(&CardDavMultistatus)`, `to_string`, `to_writer`;
  `address_object_to_string(&CardDavAddressObject)` writes a one-response multistatus.
- Model: `CardDavMultistatus`, `CardDavResponse`, `CardDavAddressObject`, and
  `CardDavAddressBookResource`, which no function reads or writes.
- `xmlns`: `DAV_NS`, `CARD_NS`.
- `Error`, including `LimitExceeded`. `MAX_INPUT_BYTES`: 64 MiB.

## Guarantees

- Elements are matched on resolved namespace and local name; neither a prefix, a suffix match, nor
  document text steers parsing
  ([CALDAV-L-0001](../../serde-caldav/docs/lessons/CALDAV-L-0001-qualified-names.md)). Enforced by:
  `tests/multistatus.rs::prefixed_elements_parse`,
  `a_note_containing_multistatus_does_not_steer_routing`, `a_type_alias_parses_as_a_multistatus`.
- Every response is kept with its own href and etag, and both round-trip. Enforced by:
  `tests/multistatus.rs::every_response_is_retained_with_its_own_href_and_etag`,
  `href_and_etag_round_trip_per_response`.
- The writer emits one document with one `d:multistatus` root, `d:` and `card:` bound on the root.
  Enforced by: `tests/multistatus.rs::the_serializer_emits_one_document`,
  `a_single_object_serializes_as_a_one_response_multistatus`.
- Embedded vCard text is XML-escaped. Enforced by: `tests/escaping.rs`.
- Readers never read more than their limit. Enforced by: `tests/limits.rs`.
- No input panics `parse_multistatus` followed by `multistatus_to_string`. Enforced by: the
  `carddav_multistatus` fuzz target.

### Known bugs

- A `DAV:href` nested inside a property value (for example `current-user-principal`) replaces the
  response's own href. Proven by:
  `tests/spec.rs::a_nested_href_does_not_replace_the_response_href` (ignored, fails today). Tracked in #167.
- The `Cargo.toml` description still says "Serde Serializer and Deserializer"
  ([REC-0013](../../../docs/decisions/REC-0013-typed-format-apis.md)). Tracked in #171.

## Limits and non-goals

- Read properties: `href`, `getetag`, `getcontenttype`, `status`, and `address-data`.
- One `address-data` that is not a valid vCard fails the whole document
  (`tests/spec.rs::an_invalid_payload_fails_the_whole_document`).
- Each `address-data` carries one card; only the first card in it is read.
- A missing content type is reported as `text/vcard; charset=utf-8`.
- Inherits `serde-vcard`'s known bugs (LF line endings inside `address-data`).
- No PROPFIND/REPORT request bodies and no server behaviour.

## Platform and permissions

Any platform; no permissions.
