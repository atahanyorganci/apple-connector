# `serde-caldav`: RFC 4791 CalDAV multistatus reader and writer

Reads and writes CalDAV `multistatus` XML documents whose `calendar-data` holds an iCalendar event,
delegating the iCalendar text to `serde-icalendar`. `apple-connector` uses it for the `/caldav`
event routes. It describes resources; it is not a CalDAV server or client.

## Kind

Codec.

## Public API

- `parse_multistatus(&[u8])`, `from_str(&str)`, `from_slice(&[u8])`, `from_reader(R)`,
  `from_reader_with_limit(R, limit)` → `Result<CalDavMultistatus>`.
- `parse_calendar_object(&[u8]) -> Result<CalDavCalendarObject>`: the first response with
  calendar data.
- `multistatus_to_string(&CalDavMultistatus)`, `to_string`, `to_writer`;
  `calendar_object_to_string(&CalDavCalendarObject)` writes a one-response multistatus.
- Model: `CalDavMultistatus { responses }`,
  `CalDavResponse { href, etag, status, calendar_object }`,
  `CalDavCalendarObject { href, etag, content_type, event }`, and `CalDavCalendarResource`, which
  no function reads or writes.
- `xmlns`: `DAV_NS`, `CALDAV_NS`, `ICAL_NS`.
- `Error`, including `LimitExceeded`. `MAX_INPUT_BYTES`: 64 MiB.

## Guarantees

- Elements are matched on resolved namespace and local name, so any prefix or a default namespace
  parses ([CALDAV-L-0001](lessons/CALDAV-L-0001-qualified-names.md)). Enforced by:
  `tests/multistatus.rs::default_namespaced_icloud_responses_parse`,
  `d_and_c_prefixed_responses_parse`.
- Every response is kept, each with its own href, etag, and status; a propstat status stands in
  when the response has none.
  A response's href is its own direct `DAV:href`; hrefs nested in property values such as
  `current-user-principal` are ignored
  (`tests/spec.rs::a_nested_href_does_not_replace_the_response_href`). Enforced by:
  `tests/multistatus.rs::every_response_is_retained_with_its_own_href`,
  `a_mixed_status_multistatus_keeps_the_not_found_response`.
- The writer emits one XML declaration and one `d:multistatus` root, with `d:` and `c:` bound on the
  root, and the result parses back with a namespace-aware reader. Enforced by:
  `tests/multistatus.rs::the_serializer_emits_one_document_with_bound_prefixes`,
  `documents_this_crate_writes_parse_back_with_a_namespace_aware_reader`,
  `metadata_round_trips_through_the_serializer`.
- Embedded iCalendar text is XML-escaped (`&`, `<`, `>`, quotes) and the document stays
  well-formed. Enforced by: `tests/escaping.rs`.
- Readers never read more than their limit. Enforced by: `tests/limits.rs`.
- No input panics `parse_multistatus` followed by `multistatus_to_string`. Enforced by: the
  `caldav_multistatus` fuzz target.

### Known bugs

- The `Cargo.toml` description still says "Serde Serializer and Deserializer"
  ([REC-0013](../../../docs/decisions/REC-0013-typed-format-apis.md)). Tracked in #171.

## Limits and non-goals

- Read properties: `href`, `getetag`, `getcontenttype`, `status`, and `calendar-data`. Everything
  else in a `prop` is ignored.
- One `calendar-data` that is not valid iCalendar fails the whole document
  (`tests/spec.rs::an_invalid_payload_fails_the_whole_document`).
- Each `calendar-data` carries one event (see `serde-icalendar`'s limits).
- A missing content type is reported as `text/calendar; charset=utf-8`; a response written without
  a status gets `HTTP/1.1 200 OK` inside its propstat.
- No PROPFIND/REPORT request bodies and no server behaviour.

## Platform and permissions

Any platform; no permissions.
