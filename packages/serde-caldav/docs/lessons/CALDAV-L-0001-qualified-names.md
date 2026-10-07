---
id: CALDAV-L-0001
status: graduated
observed-on: "quick-xml 0.37 BytesStart::name; CalDAV/CardDAV multistatus from iCloud, Google, and prefixed servers"
graduated-to: "packages/serde-caldav/tests/multistatus.rs; packages/serde-carddav/tests/multistatus.rs"
provenance:
  - https://github.com/atahanyorganci/apple-connector/issues/88
  - https://github.com/atahanyorganci/apple-connector/issues/92
  - https://github.com/atahanyorganci/apple-connector/pull/140
---

# quick-xml's `name()` is the qualified name, so comparing it to `"calendar-data"` misses every prefixed element.

## What happened

Every real server's `<C:calendar-data>` was missed and the whole document failed to parse. The
CardDAV parser instead matched on name suffixes, which also accepted `<x-my-address-data>` and
`<myhref>`.

## Why

`BytesStart::name()` returns the name as written, prefix included. Servers choose their own
prefixes (`d:`, `C:`, `cal:`) or a default namespace, so neither the qualified name nor a suffix
identifies an element.

## Rule

DAV elements are matched on their resolved namespace URI and local name (`NsReader`,
`read_resolved_event_into`, `local_name()`).

## Graduated

- `packages/serde-caldav/tests/multistatus.rs`: `default_namespaced_icloud_responses_parse`,
  `d_and_c_prefixed_responses_parse`.
- `packages/serde-carddav/tests/multistatus.rs`: `prefixed_elements_parse`,
  `a_note_containing_multistatus_does_not_steer_routing`.
